use futures::channel::oneshot;
use futures::SinkExt;
use jungle_client::{RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    Animal, AnimalObservation, AnimalPerturbation, BuildFlowWithContext, ContextExecutor, DynFlow,
    ExecutorError, ObservationAdapter, PerturbationAdapter, RunnerOut, Sleep,
};
use std::sync::Arc;
use uuid::Uuid;

pub enum RunnerAdvance {
    Completed,
    SuspendedSleep { wake_at_unix_ms: i64 },
}

pub struct JungleRunner<T> {
    jungle: Arc<T>,
}

impl<T> JungleRunner<T> {
    pub fn new(jungle: T) -> Self {
        Self {
            jungle: Arc::new(jungle),
        }
    }

    pub fn jungle(&self) -> &T {
        self.jungle.as_ref()
    }
}

impl<T> JungleRunner<T>
where
    T: 'static,
{
    pub async fn spawn<A>(
        &self,
        state: A::State,
        journey_id: Uuid,
        mut tx: RunnerChannelTx,
    ) -> Result<A::State, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::State: Clone,
        A::Journey:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = self.new_executor::<A>(state);
        self.emit_initial_appearance::<A>(&executor, journey_id, &mut tx)
            .await?;
        match self
            .drive_until_sleep_or_complete::<A>(&mut executor, journey_id, &mut tx)
            .await?
        {
            RunnerAdvance::Completed => Ok(executor.into_state()),
            RunnerAdvance::SuspendedSleep { .. } => Err(ExecutorError::ClientTransport(
                "runner.spawn encountered Sleep; use worker runtime for sleep-capable flows"
                    .to_string(),
            )),
        }
    }

    pub fn new_executor<A>(&self, state: A::State) -> ContextExecutor<T, A>
    where
        A: Animal,
        A::State: Clone,
        A::Journey:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        ContextExecutor::new(Arc::clone(&self.jungle), state)
    }

    pub async fn emit_initial_appearance<A>(
        &self,
        executor: &ContextExecutor<T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<(), ExecutorError>
    where
        A: Animal + AnimalObservation,
        A::State: Clone,
        A::Journey:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        if let Some(appearance) =
            <<A as AnimalObservation>::Adapter as ObservationAdapter<A>>::snapshot(
                executor.state(),
            )?
        {
            send_history(
                tx,
                RunnerOut::Appearance {
                    data: appearance,
                    uuid: journey_id,
                },
            )
            .await?;
        }
        Ok(())
    }

    pub async fn drive_until_sleep_or_complete<A>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::State: Clone,
        A::Journey:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        while !executor.is_complete() {
            process_perturbations(executor, journey_id, tx).await?;
            let request = match executor.next_executable_request(()) {
                Ok(request) => request,
                Err(ExecutorError::Complete) => break,
                Err(err) => return Err(err),
            };
            send_history(
                tx,
                RunnerOut::ActionInput {
                    data: request.request_bytes().to_vec(),
                    uuid: journey_id,
                },
            )
            .await?;

            if request.action_type() == core::any::type_name::<Sleep>() {
                let duration: std::time::Duration = request.deserialize_request()?;
                let duration_millis = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                let wake_at_unix_ms = chrono::Utc::now()
                    .timestamp_millis()
                    .saturating_add(duration_millis);
                return Ok(RunnerAdvance::SuspendedSleep { wake_at_unix_ms });
            }

            let completion = request.run().await?;
            apply_completion_and_emit_appearance::<T, A>(executor, journey_id, tx, completion)
                .await?;
        }
        Ok(RunnerAdvance::Completed)
    }

    pub async fn resume_after_sleep<A>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::State: Clone,
        A::Journey:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let sleep_out = postcard::to_allocvec(&())
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
        let completion = Ok(sleep_out);
        apply_completion_and_emit_appearance::<T, A>(executor, journey_id, tx, completion).await?;
        self.drive_until_sleep_or_complete::<A>(executor, journey_id, tx)
            .await
    }
}

async fn apply_completion_and_emit_appearance<T, A>(
    executor: &mut ContextExecutor<T, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
    completion: Result<Vec<u8>, Vec<u8>>,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: Animal + AnimalObservation,
    A::Journey: BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = DynFlow<A::State>>,
{
    match &completion {
        Ok(output) => {
            send_history(
                tx,
                RunnerOut::ActionSuccessOutput {
                    data: output.clone(),
                    uuid: journey_id,
                },
            )
            .await?
        }
        Err(error) => {
            send_history(
                tx,
                RunnerOut::ActionFailureOutput {
                    data: error.clone(),
                    uuid: journey_id,
                },
            )
            .await?
        }
    }
    let _emitted = executor.complete_serialized(completion)?;
    if let Some(appearance) =
        <<A as AnimalObservation>::Adapter as ObservationAdapter<A>>::snapshot(executor.state())?
    {
        send_history(
            tx,
            RunnerOut::Appearance {
                data: appearance,
                uuid: journey_id,
            },
        )
        .await?;
    }
    Ok(())
}

async fn send_history(tx: &mut RunnerChannelTx, history: RunnerOut) -> Result<(), ExecutorError> {
    let (done_tx, done_rx) = oneshot::channel();
    tx.send((RunnerChannelMessage::History(history), done_tx))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
    match done_rx
        .await
        .map_err(|_| ExecutorError::ClientTransportAckDropped)??
    {
        RunnerChannelResponse::Ack => Ok(()),
        RunnerChannelResponse::ClaimedPerturbation(_) => Err(ExecutorError::ClientTransport(
            "runner expected ack response for history message".to_string(),
        )),
    }
}

async fn process_perturbations<A, Ctx>(
    executor: &mut ContextExecutor<Ctx, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    A: Animal + AnimalPerturbation,
    Ctx: 'static,
    A::Journey: BuildFlowWithContext<(Arc<Ctx>, DynFlow<A::State>), Output = DynFlow<A::State>>,
{
    if !<<A as AnimalPerturbation>::Adapter as PerturbationAdapter<A>>::enabled() {
        return Ok(());
    }

    loop {
        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::ClaimAnimalPerturbation { journey_id },
            done_tx,
        ))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
        let claimed = match done_rx
            .await
            .map_err(|_| ExecutorError::ClientTransportAckDropped)??
        {
            RunnerChannelResponse::ClaimedPerturbation(claimed) => claimed,
            RunnerChannelResponse::Ack => {
                return Err(ExecutorError::ClientTransport(
                    "runner expected claimed perturbation response".to_string(),
                ));
            }
        };

        let Some(claimed) = claimed else {
            break;
        };

        {
            let state = executor.state_mut();
            let applied = <<A as AnimalPerturbation>::Adapter as PerturbationAdapter<A>>::apply(
                state,
                &claimed.data,
            )?;
            if !applied {
                break;
            }
        }

        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::AckAnimalPerturbation {
                journey_id,
                perturbation_id: claimed.id,
            },
            done_tx,
        ))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
        match done_rx
            .await
            .map_err(|_| ExecutorError::ClientTransportAckDropped)??
        {
            RunnerChannelResponse::Ack => {}
            RunnerChannelResponse::ClaimedPerturbation(_) => {
                return Err(ExecutorError::ClientTransport(
                    "runner expected ack response for perturbation ack".to_string(),
                ));
            }
        }
    }

    Ok(())
}
