use futures::channel::oneshot;
use futures::SinkExt;
use jungle_client::{RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    BoundAnimal, BoundAnimalJourney, BuildFlowWithContext, ContextExecutor, DynFlow,
    ExecutorError, Observable, ObservationBridge, Perturbable, PerturbationBridge, RunnerOut,
    Sleep,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

pub enum RunnerAdvance {
    Completed,
    SuspendedSleep { wake_at_unix_ms: i64, node_id: u32 },
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
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    {
        let mut executor = self.new_executor::<A>(state);
        let appearance = self.initial_appearance::<A>(&executor)?;
        self.emit_appearance(journey_id, appearance, &mut tx)
            .await?;
        match self
            .drive_until_sleep_or_complete::<A, _>(&mut executor, (), journey_id, &mut tx)
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
        A: BoundAnimal,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    {
        ContextExecutor::new(Arc::clone(&self.jungle), state)
    }

    pub fn initial_appearance<A>(
        &self,
        executor: &ContextExecutor<T, A>,
    ) -> Result<Option<Vec<u8>>, ExecutorError>
    where
        A: BoundAnimal + Observable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    {
        <<A as Observable>::Observation as ObservationBridge<A>>::snapshot(executor.state())
    }

    pub async fn emit_appearance(
        &self,
        journey_id: Uuid,
        appearance: Option<Vec<u8>>,
        tx: &mut RunnerChannelTx,
    ) -> Result<(), ExecutorError> {
        if let Some(appearance) = appearance {
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

    pub async fn drive_until_sleep_or_complete<A, Initial>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        initial_input: Initial,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
        Initial: Serialize + Clone,
    {
        while !executor.is_complete() {
            process_perturbations(executor, journey_id, tx).await?;
            let request = match executor.next_executable_request(initial_input.clone()) {
                Ok(request) => request,
                Err(ExecutorError::Complete) => break,
                Err(err) => return Err(err),
            };
            let node_id = request.node_id();
            send_history(
                tx,
                RunnerOut::EffectInput {
                    node_id,
                    data: request.request_bytes().to_vec(),
                    uuid: journey_id,
                },
            )
            .await?;

            if request.effect_type() == core::any::type_name::<Sleep>() {
                let duration: std::time::Duration = request.deserialize_request()?;
                let duration_millis = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                let wake_at_unix_ms = chrono::Utc::now()
                    .timestamp_millis()
                    .saturating_add(duration_millis);
                return Ok(RunnerAdvance::SuspendedSleep {
                    wake_at_unix_ms,
                    node_id,
                });
            }

            let completion = request.run().await?;
            apply_completion_and_emit_appearance::<T, A>(
                executor, journey_id, tx, node_id, completion,
            )
            .await?;
        }
        Ok(RunnerAdvance::Completed)
    }

    pub async fn resume_after_sleep<A>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        journey_id: Uuid,
        sleep_node_id: u32,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    {
        let sleep_out = postcard::to_allocvec(&())
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
        let completion = Ok(sleep_out);
        apply_completion_and_emit_appearance::<T, A>(
            executor,
            journey_id,
            tx,
            sleep_node_id,
            completion,
        )
        .await?;
        self.drive_until_sleep_or_complete::<A, _>(executor, (), journey_id, tx)
            .await
    }
}

async fn apply_completion_and_emit_appearance<T, A>(
    executor: &mut ContextExecutor<T, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
    node_id: u32,
    completion: Result<Vec<u8>, Vec<u8>>,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: BoundAnimal + Observable,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    match &completion {
        Ok(output) => {
            send_history(
                tx,
                RunnerOut::EffectSuccessOutput {
                    node_id,
                    data: output.clone(),
                    uuid: journey_id,
                },
            )
            .await?
        }
        Err(error) => {
            send_history(
                tx,
                RunnerOut::EffectFailureOutput {
                    node_id,
                    data: error.clone(),
                    uuid: journey_id,
                },
            )
            .await?
        }
    }
    let _emitted = executor.complete_serialized(completion)?;
    if let Some(appearance) =
        <<A as Observable>::Observation as ObservationBridge<A>>::snapshot(executor.state())?
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
    A: BoundAnimal + Perturbable,
    Ctx: 'static,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<Ctx>, DynFlow<A::State>), Output = (Arc<Ctx>, DynFlow<A::State>)>,
{
    if !<<A as Perturbable>::Perturbation as PerturbationBridge<A>>::enabled() {
        return Ok(());
    }

    loop {
        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::ClaimPerturbable { journey_id },
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
            let applied = <<A as Perturbable>::Perturbation as PerturbationBridge<A>>::apply(
                state,
                &claimed.data,
            )?;
            if !applied {
                break;
            }
        }

        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::AckPerturbable {
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
