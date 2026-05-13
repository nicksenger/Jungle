use crate::runner::{JungleRunner, RunnerAdvance};
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::SinkExt;
use futures::StreamExt;
use jungle_client::{JungleClient, RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    Animal, AnimalIdValue, AnimalObservation, AnimalPerturbation, AnimalSet, Animals,
    BuildFlowWithContext, ContextExecutor, DynFlow, Ecosystem, ExecutorError, RunnerOut, Sleep,
    StripAnimalHeaders, SupportedAnimal, Work,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use typosaurus::collections::list;
use typosaurus::collections::sp::{FlattenNodes, SPFlatten};
use typosaurus::num::Unsigned;
use uuid::Uuid;

const OWNER_LEASE_TTL_MS: i64 = 30_000;

pub struct JungleWorker<T> {
    client: Box<dyn JungleClient>,
    runner: JungleRunner<T>,
    owner_lease_ttl_ms: i64,
}

impl<T> JungleWorker<T>
where
    T: Ecosystem + 'static,
    T::Animals: Animals,
    <T::Animals as Animals>::List: FlattenNodes,
    SPFlatten<<T::Animals as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T::Animals>: SupportedAnimalGenerations<T>,
{
    pub fn new<C>(jungle: T, client: C) -> Self
    where
        C: JungleClient + 'static,
    {
        Self {
            client: Box::new(client),
            runner: JungleRunner::new(jungle),
            owner_lease_ttl_ms: OWNER_LEASE_TTL_MS,
        }
    }

    pub fn with_owner_lease_ttl_ms(mut self, owner_lease_ttl_ms: i64) -> Self {
        self.owner_lease_ttl_ms = owner_lease_ttl_ms.max(0);
        self
    }

    pub fn client(&self) -> &dyn JungleClient {
        self.client.as_ref()
    }

    pub fn runner(&self) -> &JungleRunner<T> {
        &self.runner
    }

    pub async fn spawn(&self) -> Result<(), ExecutorError> {
        let owner_id = Uuid::new_v4();
        let (tx, mut rx): (RunnerChannelTx, _) = mpsc::channel(64);
        let client_for_transport = self.client.clone();
        tokio::spawn(async move {
            while let Some((message, done)) = rx.next().await {
                let result: Result<RunnerChannelResponse, ExecutorError> = match message {
                    RunnerChannelMessage::History(history) => {
                        let out = match history {
                            RunnerOut::ActionInput {
                                node_id,
                                data,
                                uuid,
                            } => client_for_transport.action_input(uuid, node_id, data).await,
                            RunnerOut::ActionSuccessOutput {
                                node_id,
                                data,
                                uuid,
                            } => {
                                client_for_transport
                                    .action_success_output(uuid, node_id, data)
                                    .await
                            }
                            RunnerOut::ActionFailureOutput {
                                node_id,
                                data,
                                uuid,
                            } => {
                                client_for_transport
                                    .action_failure_output(uuid, node_id, data)
                                    .await
                            }
                            RunnerOut::Appearance { data, uuid } => {
                                client_for_transport
                                    .animal_appearance_update(uuid, data)
                                    .await
                            }
                            RunnerOut::SleepScheduled { .. } | RunnerOut::SleepFired { .. } => {
                                Ok(())
                            }
                        };
                        out.map(|_| RunnerChannelResponse::Ack)
                    }
                    RunnerChannelMessage::ClaimAnimalPerturbation { journey_id } => {
                        client_for_transport
                            .claim_animal_perturbation(journey_id)
                            .await
                            .map(RunnerChannelResponse::ClaimedPerturbation)
                    }
                    RunnerChannelMessage::AckAnimalPerturbation {
                        journey_id,
                        perturbation_id,
                    } => client_for_transport
                        .ack_animal_perturbation(journey_id, perturbation_id)
                        .await
                        .map(|_| RunnerChannelResponse::Ack),
                };
                let _ = done.send(result);
            }
        });

        let mut suspended: HashMap<Uuid, Box<dyn SuspendedJourney<T>>> = HashMap::new();
        let supported_animals = <AnimalSet<T::Animals> as SupportedAnimalGenerations<T>>::collect();

        loop {
            for journey_id in suspended.keys().copied().collect::<Vec<_>>() {
                self.client
                    .heartbeat_journey_lease(journey_id, owner_id, self.owner_lease_ttl_ms)
                    .await?;
            }

            let _ = self.client.poll_timers().await?;

            if let Some(wake) = self.client.poll_owner_wake(owner_id).await? {
                if let Some(mut journey) = suspended.remove(&wake.journey_id) {
                    match journey.resume(&self.runner, tx.clone()).await? {
                        SuspendedOutcome::Completed => {
                            self.client.complete_journey(wake.journey_id).await?;
                        }
                        SuspendedOutcome::Sleeping {
                            wake_at_unix_ms,
                            node_id: _,
                        } => {
                            let timer_id = Uuid::new_v4();
                            self.client
                                .schedule_sleep_timer(wake.journey_id, timer_id, wake_at_unix_ms)
                                .await?;
                            self.client
                                .heartbeat_journey_lease(
                                    wake.journey_id,
                                    owner_id,
                                    self.owner_lease_ttl_ms,
                                )
                                .await?;
                            suspended.insert(wake.journey_id, journey);
                        }
                    }
                }
            }

            match self.client.poll_work(supported_animals.clone()).await? {
                Some(Work::StartJourney {
                    journey_id,
                    animal_id,
                    generation,
                    seed,
                }) => {
                    let history = self.client.journey_history(journey_id).await?;
                    match <AnimalSet<T::Animals> as SupportedAnimalGenerations<T>>::resume_by_animal(
                        animal_id,
                        generation,
                        seed,
                        journey_id,
                        history,
                        &self.runner,
                        tx.clone(),
                    )
                    .await?
                    {
                        JourneyStartOutcome::NotMatched => {
                            return Err(ExecutorError::InputDeserialize(format!(
                                "unknown animal: id={animal_id}, generation={generation}"
                            )));
                        }
                        JourneyStartOutcome::Completed => {
                            self.client.complete_journey(journey_id).await?;
                        }
                        JourneyStartOutcome::Sleeping {
                            wake_at_unix_ms,
                            journey,
                        } => {
                            let timer_id = Uuid::new_v4();
                            self.client
                                .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
                                .await?;
                            self.client
                                .heartbeat_journey_lease(
                                    journey_id,
                                    owner_id,
                                    self.owner_lease_ttl_ms,
                                )
                                .await?;
                            suspended.insert(journey_id, journey);
                        }
                    }
                }
                Some(Work::ResumeJourney {
                    journey_id,
                    animal_id,
                    generation,
                    seed,
                }) => {
                    let history = self.client.journey_history(journey_id).await?;
                    match <AnimalSet<T::Animals> as SupportedAnimalGenerations<T>>::resume_by_animal(
                        animal_id,
                        generation,
                        seed,
                        journey_id,
                        history,
                        &self.runner,
                        tx.clone(),
                    )
                    .await?
                    {
                        JourneyStartOutcome::NotMatched => {
                            return Err(ExecutorError::InputDeserialize(format!(
                                "unknown animal: id={animal_id}, generation={generation}"
                            )));
                        }
                        JourneyStartOutcome::Completed => {
                            self.client.complete_journey(journey_id).await?;
                        }
                        JourneyStartOutcome::Sleeping {
                            wake_at_unix_ms,
                            journey,
                        } => {
                            let timer_id = Uuid::new_v4();
                            self.client
                                .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
                                .await?;
                            self.client
                                .heartbeat_journey_lease(
                                    journey_id,
                                    owner_id,
                                    self.owner_lease_ttl_ms,
                                )
                                .await?;
                            suspended.insert(journey_id, journey);
                        }
                    }
                }
                None => {}
            }

            sleep(Duration::from_millis(200)).await;
        }
    }
}

pub enum JourneyStartOutcome<T> {
    NotMatched,
    Completed,
    Sleeping {
        wake_at_unix_ms: i64,
        journey: Box<dyn SuspendedJourney<T>>,
    },
}

pub enum SuspendedOutcome {
    Completed,
    Sleeping { wake_at_unix_ms: i64, node_id: u32 },
}

pub trait SuspendedJourney<T> {
    fn resume<'a>(
        &'a mut self,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + 'a>>;
}

struct SuspendedAnimalJourney<T, A>
where
    T: 'static,
    A: Animal + AnimalObservation + AnimalPerturbation + Send + Sync + 'static,
    A::Journey:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    journey_id: Uuid,
    sleep_node_id: u32,
    executor: ContextExecutor<T, A>,
}

impl<T, A> SuspendedJourney<T> for SuspendedAnimalJourney<T, A>
where
    T: 'static,
    A: Animal + AnimalObservation + AnimalPerturbation + Send + Sync + 'static,
    A::Journey:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    fn resume<'a>(
        &'a mut self,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + 'a>> {
        Box::pin(async move {
            let advance = runner
                .resume_after_sleep::<A>(
                    &mut self.executor,
                    self.journey_id,
                    self.sleep_node_id,
                    &mut tx,
                )
                .await?;
            match advance {
                RunnerAdvance::Completed => Ok(SuspendedOutcome::Completed),
                RunnerAdvance::SuspendedSleep {
                    wake_at_unix_ms,
                    node_id,
                } => {
                    self.sleep_node_id = node_id;
                    Ok(SuspendedOutcome::Sleeping {
                        wake_at_unix_ms,
                        node_id,
                    })
                }
            }
        })
    }
}

pub trait SupportedAnimalGenerations<T> {
    fn collect() -> Vec<SupportedAnimal>;

    fn spawn_by_animal<'a>(
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
        journey_id: Uuid,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>>;

    fn resume_by_animal<'a>(
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
        journey_id: Uuid,
        history: Vec<RunnerOut>,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>>;
}

impl<T> SupportedAnimalGenerations<T> for list::Empty {
    fn collect() -> Vec<SupportedAnimal> {
        Vec::new()
    }

    fn spawn_by_animal<'a>(
        _animal_id: u32,
        _generation: u32,
        _seed: Vec<u8>,
        _journey_id: Uuid,
        _runner: &'a JungleRunner<T>,
        _tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>> {
        Box::pin(async { Ok(JourneyStartOutcome::NotMatched) })
    }

    fn resume_by_animal<'a>(
        _animal_id: u32,
        _generation: u32,
        _seed: Vec<u8>,
        _journey_id: Uuid,
        _history: Vec<RunnerOut>,
        _runner: &'a JungleRunner<T>,
        _tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>> {
        Box::pin(async { Ok(JourneyStartOutcome::NotMatched) })
    }
}

impl<T, Head, Tail> SupportedAnimalGenerations<T> for list::List<(Head, Tail)>
where
    Head: Animal + AnimalObservation + AnimalPerturbation + Send + Sync + 'static,
    Head::Id: AnimalIdValue,
    Head::Generation: Unsigned,
    Head::Seed: Send + 'static,
    Head::State: Send + 'static,
    Head::Journey: BuildFlowWithContext<
        (Arc<T>, DynFlow<Head::State>),
        Output = (Arc<T>, DynFlow<Head::State>),
    >,
    Tail: SupportedAnimalGenerations<T>,
    T: 'static,
{
    fn collect() -> Vec<SupportedAnimal> {
        let mut out = vec![SupportedAnimal {
            animal_id: <Head::Id as AnimalIdValue>::U32,
            generation: <Head::Generation as Unsigned>::U32,
        }];
        out.extend(Tail::collect());
        out
    }

    fn spawn_by_animal<'a>(
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
        journey_id: Uuid,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>> {
        Box::pin(async move {
            if animal_id == <Head::Id as AnimalIdValue>::U32
                && generation == <Head::Generation as Unsigned>::U32
            {
                let seed: Head::Seed = postcard::from_bytes(&seed)
                    .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
                let state: Head::State = seed.into();
                let mut executor = runner.new_executor::<Head>(state);
                runner
                    .emit_initial_appearance::<Head>(&executor, journey_id, &mut tx)
                    .await?;
                match runner
                    .drive_until_sleep_or_complete::<Head>(&mut executor, journey_id, &mut tx)
                    .await?
                {
                    RunnerAdvance::Completed => Ok(JourneyStartOutcome::Completed),
                    RunnerAdvance::SuspendedSleep {
                        wake_at_unix_ms,
                        node_id,
                    } => {
                        let suspended = SuspendedAnimalJourney::<T, Head> {
                            journey_id,
                            sleep_node_id: node_id,
                            executor,
                        };
                        Ok(JourneyStartOutcome::Sleeping {
                            wake_at_unix_ms,
                            journey: Box::new(suspended),
                        })
                    }
                }
            } else {
                Tail::spawn_by_animal(animal_id, generation, seed, journey_id, runner, tx).await
            }
        })
    }

    fn resume_by_animal<'a>(
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
        journey_id: Uuid,
        history: Vec<RunnerOut>,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + 'a>> {
        Box::pin(async move {
            if animal_id == <Head::Id as AnimalIdValue>::U32
                && generation == <Head::Generation as Unsigned>::U32
            {
                let seed: Head::Seed = postcard::from_bytes(&seed)
                    .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
                let state: Head::State = seed.into();
                let mut executor = runner.new_executor::<Head>(state);
                replay_history::<T, Head>(&mut executor, journey_id, &history, &mut tx).await?;
                runner
                    .emit_initial_appearance::<Head>(&executor, journey_id, &mut tx)
                    .await?;
                match runner
                    .drive_until_sleep_or_complete::<Head>(&mut executor, journey_id, &mut tx)
                    .await?
                {
                    RunnerAdvance::Completed => Ok(JourneyStartOutcome::Completed),
                    RunnerAdvance::SuspendedSleep {
                        wake_at_unix_ms,
                        node_id,
                    } => {
                        let suspended = SuspendedAnimalJourney::<T, Head> {
                            journey_id,
                            sleep_node_id: node_id,
                            executor,
                        };
                        Ok(JourneyStartOutcome::Sleeping {
                            wake_at_unix_ms,
                            journey: Box::new(suspended),
                        })
                    }
                }
            } else {
                Tail::resume_by_animal(animal_id, generation, seed, journey_id, history, runner, tx)
                    .await
            }
        })
    }
}

async fn replay_history<T, A>(
    executor: &mut ContextExecutor<T, A>,
    journey_id: Uuid,
    history: &[RunnerOut],
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: Animal + AnimalObservation + AnimalPerturbation,
    A::Journey:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    let mut index = 0usize;
    while !executor.is_complete() {
        if index >= history.len() {
            break;
        }

        let request = match executor.next_executable_request(()) {
            Ok(request) => request,
            Err(ExecutorError::Complete) => break,
            Err(err) => return Err(err),
        };
        let request_node_id = request.node_id();
        let expected_input = request.request_bytes();
        let action_type = request.action_type();

        let Some(RunnerOut::ActionInput {
            node_id,
            data,
            uuid,
        }) = history.get(index)
        else {
            return Err(ExecutorError::ClientTransport(
                "history replay expected ActionInput event".to_string(),
            ));
        };
        if *uuid != journey_id || *node_id != request_node_id || data.as_slice() != expected_input {
            return Err(ExecutorError::ClientTransport(
                "history replay ActionInput mismatch".to_string(),
            ));
        }
        index = index.saturating_add(1);

        let completion = if action_type == core::any::type_name::<Sleep>() {
            while matches!(
                history.get(index),
                Some(RunnerOut::SleepScheduled { uuid, .. }) if *uuid == journey_id
            ) {
                index = index.saturating_add(1);
            }

            match history.get(index) {
                Some(RunnerOut::SleepFired { uuid, .. }) if *uuid == journey_id => {
                    index = index.saturating_add(1);
                    let sleep_out = postcard::to_allocvec(&())
                        .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                    Ok(sleep_out)
                }
                _ => {
                    let completion = request.run().await?;
                    send_recovered_completion(tx, journey_id, request_node_id, &completion).await?;
                    completion
                }
            }
        } else {
            match history.get(index) {
                Some(RunnerOut::ActionSuccessOutput {
                    node_id,
                    data,
                    uuid,
                }) if *uuid == journey_id && *node_id == request_node_id => {
                    index = index.saturating_add(1);
                    Ok(data.clone())
                }
                Some(RunnerOut::ActionFailureOutput {
                    node_id,
                    data,
                    uuid,
                }) if *uuid == journey_id && *node_id == request_node_id => {
                    index = index.saturating_add(1);
                    Err(data.clone())
                }
                _ => {
                    let completion = request.run().await?;
                    send_recovered_completion(tx, journey_id, request_node_id, &completion).await?;
                    completion
                }
            }
        };

        let _emitted = executor.complete_serialized(completion)?;
    }

    Ok(())
}

async fn send_recovered_completion(
    tx: &mut RunnerChannelTx,
    journey_id: Uuid,
    node_id: u32,
    completion: &Result<Vec<u8>, Vec<u8>>,
) -> Result<(), ExecutorError> {
    let out = match completion {
        Ok(data) => RunnerOut::ActionSuccessOutput {
            node_id,
            data: data.clone(),
            uuid: journey_id,
        },
        Err(data) => RunnerOut::ActionFailureOutput {
            node_id,
            data: data.clone(),
            uuid: journey_id,
        },
    };
    let (done_tx, done_rx) = oneshot::channel();
    tx.send((RunnerChannelMessage::History(out), done_tx))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
    match done_rx
        .await
        .map_err(|_| ExecutorError::ClientTransportAckDropped)??
    {
        RunnerChannelResponse::Ack => Ok(()),
        RunnerChannelResponse::ClaimedPerturbation(_) => Err(ExecutorError::ClientTransport(
            "runner expected ack response while replaying recovered completion".to_string(),
        )),
    }
}
