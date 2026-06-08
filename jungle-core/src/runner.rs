use futures::channel::oneshot;
use futures::future;
use futures::stream::FuturesUnordered;
use futures::SinkExt;
use futures::StreamExt;
use jungle_client::{RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    BoundAnimal, BoundAnimalJourney, BuildFlowWithContext, ContextExecutor, DynFlow,
    ExecutableEffectRequest, ExecutorError, Failure, Observable, ObservationBridge, Perturbable,
    PerturbationBridge, RunnerOut, Sleep,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSleep {
    pub wake_at_unix_ms: i64,
    pub node_id: u32,
    pub completion: Result<Vec<u8>, Vec<u8>>,
}

pub enum RunnerAdvance {
    Completed,
    SuspendedSleep { sleeps: Vec<PendingSleep> },
    Failed { failure: Failure },
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
        executor.set_journey_id(journey_id);
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
            RunnerAdvance::Failed { failure } => Err(ExecutorError::ActionFailure(failure)),
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
        self.drive_until_sleep_or_complete_with_pending::<A, Initial>(
            executor,
            initial_input,
            journey_id,
            tx,
            Vec::new(),
            Vec::new(),
        )
        .await
    }

    pub async fn drive_until_sleep_or_complete_with_replay_pending<A, Initial>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        initial_input: Initial,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
        pending_requests: Vec<ExecutableEffectRequest>,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
        Initial: Serialize + Clone,
    {
        self.drive_until_sleep_or_complete_with_pending::<A, Initial>(
            executor,
            initial_input,
            journey_id,
            tx,
            Vec::new(),
            pending_requests,
        )
        .await
    }

    async fn drive_until_sleep_or_complete_with_pending<A, Initial>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        initial_input: Initial,
        journey_id: Uuid,
        tx: &mut RunnerChannelTx,
        mut pending_sleeps: Vec<PendingSleep>,
        pending_requests: Vec<ExecutableEffectRequest>,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
        Initial: Serialize + Clone,
    {
        let sleep_effect_type = core::any::type_name::<Sleep>();
        let mut in_flight = FuturesUnordered::new();
        let mut replay_pending_in_flight = pending_requests.len();
        let mut pending_wave_settle = false;
        for request in pending_requests {
            in_flight.push(run_request_task(tx.clone(), request));
        }
        while !executor.is_complete() || !in_flight.is_empty() || !pending_sleeps.is_empty() {
            if pending_wave_settle {
                let newly_dispatched = if replay_pending_in_flight == 0 {
                    collect_ready_requests(
                        executor,
                        initial_input.clone(),
                        tx,
                        &mut pending_sleeps,
                        sleep_effect_type,
                    )
                    .await?
                } else {
                    Vec::new()
                };
                if newly_dispatched.is_empty() {
                    let appearance =
                        <<A as Observable>::Observation as ObservationBridge<A>>::snapshot(
                            executor.state(),
                        )?;
                    emit_snapshot_appearance(journey_id, appearance, tx).await?;
                }
                pending_wave_settle = false;
                for request in newly_dispatched {
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
                    in_flight.push(run_request_task(tx.clone(), request));
                }
            }

            if !pending_sleeps.is_empty() && in_flight.is_empty() {
                return Ok(RunnerAdvance::SuspendedSleep {
                    sleeps: pending_sleeps,
                });
            }

            if in_flight.is_empty() && pending_sleeps.is_empty() {
                process_perturbations(executor, journey_id, tx).await?;
            }

            let newly_dispatched = if replay_pending_in_flight == 0 {
                collect_ready_requests(
                    executor,
                    initial_input.clone(),
                    tx,
                    &mut pending_sleeps,
                    sleep_effect_type,
                )
                .await?
            } else {
                Vec::new()
            };

            for request in newly_dispatched {
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
                in_flight.push(run_request_task(tx.clone(), request));
            }

            if in_flight.is_empty() {
                continue;
            }

            let Some((node_id, completion)) = in_flight.next().await else {
                if executor.is_complete() {
                    break;
                }
                return Err(ExecutorError::ClientTransport(
                    "runner found no in-flight requests while journey remained incomplete"
                        .to_string(),
                ));
            };

            if let Err(err) = apply_completion_and_emit_appearance::<T, A>(
                executor,
                journey_id,
                tx,
                node_id,
                completion?,
                false,
            )
            .await
            {
                return match err {
                    ExecutorError::ActionFailure(failure) => Ok(RunnerAdvance::Failed { failure }),
                    other => Err(other),
                };
            }
            if replay_pending_in_flight > 0 {
                replay_pending_in_flight -= 1;
            }
            if in_flight.is_empty() {
                pending_wave_settle = true;
            }
        }
        Ok(RunnerAdvance::Completed)
    }

    pub async fn resume_after_sleep<A>(
        &self,
        executor: &mut ContextExecutor<T, A>,
        journey_id: Uuid,
        completed_sleep: PendingSleep,
        pending_sleeps: Vec<PendingSleep>,
        tx: &mut RunnerChannelTx,
    ) -> Result<RunnerAdvance, ExecutorError>
    where
        A: BoundAnimal + Observable + Perturbable,
        BoundAnimalJourney<A>:
            BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    {
        apply_completion_and_emit_appearance::<T, A>(
            executor,
            journey_id,
            tx,
            completed_sleep.node_id,
            completed_sleep.completion,
            true,
        )
        .await?;
        self.drive_until_sleep_or_complete_with_pending::<A, _>(
            executor,
            (),
            journey_id,
            tx,
            pending_sleeps,
            Vec::new(),
        )
        .await
    }
}

async fn run_request_with_live_history_owned(
    mut tx: RunnerChannelTx,
    mut request: jungle_types::ExecutableEffectRequest,
) -> Result<Result<Vec<u8>, Vec<u8>>, ExecutorError> {
    let Some(mut live_history_rx) = request.take_live_history() else {
        return request.run().await;
    };
    let mut completion = Box::pin(request.run());
    loop {
        match future::select(completion, live_history_rx.next()).await {
            future::Either::Left((result, _)) => {
                while let Some(event) = live_history_rx.next().await {
                    send_history(&mut tx, event).await?;
                }
                return result;
            }
            future::Either::Right((maybe_event, next_completion)) => {
                completion = next_completion;
                match maybe_event {
                    Some(event) => send_history(&mut tx, event).await?,
                    None => return completion.await,
                }
            }
        }
    }
}

fn run_request_task(
    tx: RunnerChannelTx,
    request: jungle_types::ExecutableEffectRequest,
) -> impl std::future::Future<Output = (u32, Result<Result<Vec<u8>, Vec<u8>>, ExecutorError>)> + Send
{
    let node_id = request.node_id();
    async move {
        let completion = run_request_with_live_history_owned(tx, request).await;
        (node_id, completion)
    }
}

async fn apply_completion_and_emit_appearance<T, A>(
    executor: &mut ContextExecutor<T, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
    node_id: u32,
    completion: Result<Vec<u8>, Vec<u8>>,
    emit_appearance: bool,
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
    send_lifecycle_updates(executor, tx).await?;
    if emit_appearance {
        let appearance =
            <<A as Observable>::Observation as ObservationBridge<A>>::snapshot(executor.state())?;
        emit_snapshot_appearance(journey_id, appearance, tx).await?;
    }
    Ok(())
}

async fn emit_snapshot_appearance(
    journey_id: Uuid,
    appearance: Option<Vec<u8>>,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
{
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

async fn collect_ready_requests<T, A, Initial>(
    executor: &mut ContextExecutor<T, A>,
    initial_input: Initial,
    tx: &mut RunnerChannelTx,
    pending_sleeps: &mut Vec<PendingSleep>,
    sleep_effect_type: &'static str,
) -> Result<Vec<ExecutableEffectRequest>, ExecutorError>
where
    T: 'static,
    A: BoundAnimal + Observable + Perturbable,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    Initial: Serialize + Clone,
{
    let mut ready = Vec::new();
    loop {
        let request = match executor.next_executable_request(initial_input.clone()) {
            Ok(request) => request,
            Err(ExecutorError::Complete) => {
                send_lifecycle_updates(executor, tx).await?;
                break;
            }
            Err(ExecutorError::AwaitingCompletion) => break,
            Err(err) => return Err(err),
        };
        send_lifecycle_updates(executor, tx).await?;
        if request.effect_type() == sleep_effect_type {
            let duration: std::time::Duration = request.deserialize_request()?;
            let duration_millis = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
            let wake_at_unix_ms = chrono::Utc::now()
                .timestamp_millis()
                .saturating_add(duration_millis);
            let completion = match request.suspended_completion() {
                Some(completion) => completion.clone(),
                None => Ok(postcard::to_allocvec(&())
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?),
            };
            pending_sleeps.push(PendingSleep {
                wake_at_unix_ms,
                node_id: request.node_id(),
                completion,
            });
            continue;
        }
        ready.push(request);
    }
    Ok(ready)
}

async fn send_lifecycle_updates<T, A>(
    executor: &mut ContextExecutor<T, A>,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: BoundAnimal,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    for update in executor.take_node_lifecycle_updates() {
        send_history(tx, RunnerOut::NodeLifecycle(update)).await?;
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
