use futures::channel::oneshot;
use futures::SinkExt;
use jungle_client::{RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    Animal, AnimalObservation, AnimalPerturbation, BuildFlowWithContext, ContextExecutor, DynFlow,
    ExecutorError, ObservationAdapter, PerturbationAdapter, RunnerOut, Sleep, SleepInput,
};
use uuid::Uuid;

pub enum RunnerAdvance {
    Completed,
    SuspendedSleep { wake_at_unix_ms: i64 },
}

pub struct JungleRunner<T> {
    jungle: T,
}

impl<T> JungleRunner<T> {
    pub fn new(jungle: T) -> Self {
        Self { jungle }
    }

    pub fn jungle(&self) -> &T {
        &self.jungle
    }
}

impl<T> JungleRunner<T>
where
    T: 'static,
{
    pub fn new_executor<A>(&self, state: A::State) -> ContextExecutor<'_, T, A>
    where
        A: Animal,
        A::Journey: BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        ContextExecutor::new(&self.jungle, state)
    }

    pub async fn emit_initial_appearance<A>(
        &self,
        executor: &ContextExecutor<'_, T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<(), ExecutorError>
    where
        A: Animal + AnimalObservation,
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
        executor: &mut ContextExecutor<'_, T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::Journey: BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        while !executor.is_complete() {
            process_perturbations(executor, journey_id, tx).await?;
            let request = executor.next_executable_request(())?;
            send_history(
                tx,
                RunnerOut::ActionInput {
                    data: request.request_bytes().to_vec(),
                    uuid: journey_id,
                },
            )
            .await?;

            if request.action_type() == core::any::type_name::<Sleep>() {
                let sleep_input: SleepInput = request.deserialize_request()?;
                return Ok(RunnerAdvance::SuspendedSleep {
                    wake_at_unix_ms: sleep_input.wake_at_unix_ms,
                });
            }

            let completion = request.run().await?;
            apply_completion_and_emit_appearance::<T, A>(executor, journey_id, tx, completion).await?;
        }
        Ok(RunnerAdvance::Completed)
    }

    pub async fn resume_after_sleep<A>(
        &self,
        executor: &mut ContextExecutor<'_, T, A>,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::Journey: BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let sleep_out =
            postcard::to_allocvec(&()).map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
        let completion = Ok(sleep_out);
        apply_completion_and_emit_appearance::<T, A>(executor, journey_id, tx, completion).await?;
        self.drive_until_sleep_or_complete::<A>(executor, journey_id, tx)
            .await
    }
}

async fn apply_completion_and_emit_appearance<T, A>(
    executor: &mut ContextExecutor<'_, T, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
    completion: Result<Vec<u8>, Vec<u8>>,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: Animal + AnimalObservation,
    A::Journey: BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
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
    executor: &mut ContextExecutor<'_, Ctx, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    A: Animal + AnimalPerturbation,
    Ctx: 'static,
    A::Journey: BuildFlowWithContext<(*const Ctx, DynFlow<A::State>), Output = DynFlow<A::State>>,
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
