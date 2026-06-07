use crate::runner::{JungleRunner, RunnerAdvance};
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use jungle_client::{JungleClient, RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    AnimalIdValue, AnimalSet, Animals, ArgputForState, BoundAnimal, BoundAnimalJourney,
    BuildFlowWithContext, ContextExecutor, DynFlow, Ecosystem, ExecutorError, Failure,
    JourneyEvent, JourneyStatus, NoEffect, Observable, Perturbable, RunnerOut, Sleep,
    StripAnimalHeaders, SupportedAnimal, Work,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio::time::Instant;
use tracing::{debug, warn};
use typosaurus::collections::list;
use typosaurus::collections::sp::{FlattenNodes, SPFlatten};
use typosaurus::num::Unsigned;
use uuid::Uuid;

const OWNER_LEASE_TTL_MS: i64 = 30_000;
const MAX_IN_FLIGHT_JOURNEYS: usize = 8;
const ACTIVE_WAKE_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
const IDLE_WAKE_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
const WORKER_ACTIVITY_LOG_INTERVAL: u64 = 256;
const WORKER_SLOW_POLL_WARN_THRESHOLD: Duration = Duration::from_millis(100);
const WORKER_SLOW_HISTORY_WARN_THRESHOLD: Duration = Duration::from_millis(200);
const WORKER_SLOW_WAKE_WAIT_WARN_THRESHOLD: Duration = Duration::from_millis(120);
const WORKER_SLOW_HISTORY_SUBMIT_WARN_THRESHOLD: Duration = Duration::from_millis(80);
const DEFAULT_REPLAY_PAGE_SIZE: u32 = 256;

fn heartbeat_interval_for_lease_ttl(lease_ttl_ms: i64) -> Duration {
    // Refresh at ~3x faster than expiration to keep ownership stable without hot-looping.
    let ttl_ms = lease_ttl_ms.max(1) as u64;
    Duration::from_millis((ttl_ms / 3).max(1))
}

pub struct JungleWorker<T> {
    client: Box<dyn JungleClient>,
    runner: JungleRunner<T>,
    owner_lease_ttl_ms: i64,
    max_in_flight_journeys: usize,
    replay_page_size: u32,
}

impl<T> JungleWorker<T>
where
    T: Ecosystem + Send + Sync + 'static,
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
            max_in_flight_journeys: MAX_IN_FLIGHT_JOURNEYS,
            replay_page_size: DEFAULT_REPLAY_PAGE_SIZE,
        }
    }

    pub fn with_owner_lease_ttl_ms(mut self, owner_lease_ttl_ms: i64) -> Self {
        self.owner_lease_ttl_ms = owner_lease_ttl_ms.max(0);
        self
    }

    pub fn with_max_in_flight_journeys(mut self, max_in_flight_journeys: usize) -> Self {
        self.max_in_flight_journeys = max_in_flight_journeys.max(1);
        self
    }

    pub fn with_replay_page_size(mut self, replay_page_size: u32) -> Self {
        self.replay_page_size = replay_page_size.max(1);
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
        let heartbeat_interval = heartbeat_interval_for_lease_ttl(self.owner_lease_ttl_ms);
        let mut next_heartbeat_at = Instant::now();
        let (tx, mut rx): (RunnerChannelTx, _) = mpsc::channel(64);
        let client_for_transport = self.client.clone();
        tokio::spawn(async move {
            let mut history_submissions = 0_u64;
            while let Some((message, done)) = rx.next().await {
                let result: Result<RunnerChannelResponse, ExecutorError> = match message {
                    RunnerChannelMessage::History(history) => {
                        let history_kind = runner_out_kind(&history);
                        let submit_started_at = Instant::now();
                        let out = client_for_transport.submit_history_event(history).await;
                        history_submissions = history_submissions.saturating_add(1);
                        let submit_elapsed = submit_started_at.elapsed();
                        if submit_elapsed > WORKER_SLOW_HISTORY_SUBMIT_WARN_THRESHOLD {
                            warn!(
                                owner_id = %owner_id,
                                history_kind,
                                submit_elapsed_ms = submit_elapsed.as_millis(),
                                history_submissions,
                                "slow worker history submission"
                            );
                        } else if history_submissions % WORKER_ACTIVITY_LOG_INTERVAL == 0 {
                            debug!(
                                owner_id = %owner_id,
                                history_kind,
                                submit_elapsed_ms = submit_elapsed.as_millis(),
                                history_submissions,
                                "worker history submission heartbeat"
                            );
                        }
                        out.map(|_| RunnerChannelResponse::Ack)
                    }
                    RunnerChannelMessage::ClaimPerturbable { journey_id } => client_for_transport
                        .claim_animal_perturbation(journey_id)
                        .await
                        .map(RunnerChannelResponse::ClaimedPerturbation),
                    RunnerChannelMessage::AckPerturbable {
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
        let mut active_journeys: HashSet<Uuid> = HashSet::new();
        let mut pending_wakes = VecDeque::<Uuid>::new();
        let mut pending_wake_ids = HashSet::<Uuid>::new();
        let mut in_flight: FuturesUnordered<InFlightFuture<'_, T>> = FuturesUnordered::new();
        let supported_animals = <AnimalSet<T::Animals> as SupportedAnimalGenerations<T>>::collect();
        let mut should_poll = true;
        let mut poll_attempts = 0_u64;
        let mut wake_waits = 0_u64;
        let mut claimed_work_total = 0_u64;
        let mut in_flight_completions = 0_u64;

        loop {
            if (!suspended.is_empty() || !active_journeys.is_empty())
                && Instant::now() >= next_heartbeat_at
            {
                for journey_id in suspended.keys().copied().collect::<Vec<_>>() {
                    if is_terminal_journey_status(self.client.journey_details(journey_id).await?) {
                        suspended.remove(&journey_id);
                        pending_wake_ids.remove(&journey_id);
                        pending_wakes.retain(|id| *id != journey_id);
                        continue;
                    }
                    self.client
                        .heartbeat_journey_lease(journey_id, owner_id, self.owner_lease_ttl_ms)
                        .await?;
                }
                for journey_id in active_journeys.iter().copied() {
                    self.client
                        .heartbeat_journey_lease(journey_id, owner_id, self.owner_lease_ttl_ms)
                        .await?;
                }
                next_heartbeat_at = Instant::now() + heartbeat_interval;
            }

            if should_poll {
                if let Some(wake) = self.client.poll_owner_wake(owner_id).await? {
                    if !pending_wake_ids.contains(&wake.journey_id) {
                        pending_wake_ids.insert(wake.journey_id);
                        pending_wakes.push_back(wake.journey_id);
                    }
                }

                while in_flight.len() < self.max_in_flight_journeys {
                    if let Some(wake_journey_id) = pending_wakes.pop_front() {
                        pending_wake_ids.remove(&wake_journey_id);
                        if active_journeys.contains(&wake_journey_id) {
                            continue;
                        }
                        if is_terminal_journey_status(
                            self.client.journey_details(wake_journey_id).await?,
                        ) {
                            suspended.remove(&wake_journey_id);
                            continue;
                        }
                        if let Some(journey) = suspended.remove(&wake_journey_id) {
                            active_journeys.insert(wake_journey_id);
                            in_flight.push(run_resume_suspended(
                                wake_journey_id,
                                journey,
                                &self.runner,
                                tx.clone(),
                            ));
                        }
                        continue;
                    }

                    poll_attempts = poll_attempts.saturating_add(1);
                    let poll_started_at = Instant::now();
                    let Some(work) = self.client.poll_work(supported_animals.clone()).await? else {
                        let poll_elapsed = poll_started_at.elapsed();
                        if poll_elapsed > WORKER_SLOW_POLL_WARN_THRESHOLD {
                            warn!(
                                owner_id = %owner_id,
                                poll_attempts,
                                poll_elapsed_ms = poll_elapsed.as_millis(),
                                suspended_count = suspended.len(),
                                active_count = active_journeys.len(),
                                in_flight_count = in_flight.len(),
                                "slow poll_work with no available work"
                            );
                        }
                        break;
                    };
                    let poll_elapsed = poll_started_at.elapsed();
                    if poll_elapsed > WORKER_SLOW_POLL_WARN_THRESHOLD {
                        warn!(
                            owner_id = %owner_id,
                            poll_attempts,
                            poll_elapsed_ms = poll_elapsed.as_millis(),
                            suspended_count = suspended.len(),
                            active_count = active_journeys.len(),
                            in_flight_count = in_flight.len(),
                            "slow poll_work while claiming work"
                        );
                    }

                    let (journey_id, animal_id, generation, seed) = match work {
                        Work::StartJourney {
                            journey_id,
                            animal_id,
                            generation,
                            seed,
                        } => (journey_id, animal_id, generation, seed),
                        Work::ResumeJourney {
                            journey_id,
                            animal_id,
                            generation,
                            seed,
                        } => (journey_id, animal_id, generation, seed),
                    };

                    if active_journeys.contains(&journey_id) || suspended.contains_key(&journey_id)
                    {
                        continue;
                    }

                    claimed_work_total = claimed_work_total.saturating_add(1);
                    if claimed_work_total % WORKER_ACTIVITY_LOG_INTERVAL == 0 {
                        debug!(
                            owner_id = %owner_id,
                            journey_id = %journey_id,
                            claimed_work_total,
                            replay_page_size = self.replay_page_size,
                            suspended_count = suspended.len(),
                            active_count = active_journeys.len(),
                            in_flight_count = in_flight.len(),
                            "worker claim heartbeat"
                        );
                    }
                    active_journeys.insert(journey_id);
                    in_flight.push(run_claimed_work(
                        journey_id,
                        animal_id,
                        generation,
                        seed,
                        self.client.clone(),
                        self.replay_page_size,
                        &self.runner,
                        tx.clone(),
                    ));
                }
            }

            while let Some(result) = in_flight.next().now_or_never().flatten() {
                in_flight_completions = in_flight_completions.saturating_add(1);
                handle_in_flight_result(
                    result?,
                    &mut active_journeys,
                    &mut suspended,
                    self.client.as_ref(),
                    owner_id,
                    self.owner_lease_ttl_ms,
                )
                .await?;
                if in_flight_completions % WORKER_ACTIVITY_LOG_INTERVAL == 0 {
                    debug!(
                        owner_id = %owner_id,
                        in_flight_completions,
                        suspended_count = suspended.len(),
                        active_count = active_journeys.len(),
                        in_flight_count = in_flight.len(),
                        "worker in-flight completion heartbeat"
                    );
                }
            }

            should_poll = false;
            if !in_flight.is_empty() {
                if let Ok(Some(result)) = timeout(Duration::from_millis(25), in_flight.next()).await
                {
                    handle_in_flight_result(
                        result?,
                        &mut active_journeys,
                        &mut suspended,
                        self.client.as_ref(),
                        owner_id,
                        self.owner_lease_ttl_ms,
                    )
                    .await?;
                    should_poll = true;
                }
            } else {
                let has_owned_journeys = !suspended.is_empty() || !active_journeys.is_empty();
                let wake_timeout = if has_owned_journeys {
                    ACTIVE_WAKE_WAIT_TIMEOUT
                } else {
                    IDLE_WAKE_WAIT_TIMEOUT
                };
                wake_waits = wake_waits.saturating_add(1);
                let wake_wait_started_at = Instant::now();
                self.client
                    .wait_for_worker_wake(owner_id, supported_animals.clone(), wake_timeout)
                    .await?;
                let wake_wait_elapsed = wake_wait_started_at.elapsed();
                if wake_wait_elapsed > WORKER_SLOW_WAKE_WAIT_WARN_THRESHOLD {
                    debug!(
                        owner_id = %owner_id,
                        wake_waits,
                        wake_timeout_ms = wake_timeout.as_millis(),
                        wake_wait_elapsed_ms = wake_wait_elapsed.as_millis(),
                        suspended_count = suspended.len(),
                        active_count = active_journeys.len(),
                        "slow wait_for_worker_wake"
                    );
                } else if wake_waits % WORKER_ACTIVITY_LOG_INTERVAL == 0 {
                    debug!(
                        owner_id = %owner_id,
                        wake_waits,
                        wake_timeout_ms = wake_timeout.as_millis(),
                        wake_wait_elapsed_ms = wake_wait_elapsed.as_millis(),
                        suspended_count = suspended.len(),
                        active_count = active_journeys.len(),
                        "worker wake wait heartbeat"
                    );
                }
                should_poll = true;
            }

            if !pending_wakes.is_empty() {
                should_poll = true;
            }
        }
    }
}

fn runner_out_kind(history: &RunnerOut) -> &'static str {
    match history {
        RunnerOut::NodeLifecycle(..) => "node_lifecycle",
        RunnerOut::EffectInput { .. } => "effect_input",
        RunnerOut::EffectSuccessOutput { .. } => "effect_success_output",
        RunnerOut::EffectFailureOutput { .. } => "effect_failure_output",
        RunnerOut::Appearance { .. } => "appearance",
        RunnerOut::SleepScheduled { .. } => "sleep_scheduled",
        RunnerOut::SleepFired { .. } => "sleep_fired",
    }
}

fn is_informational_history_event(history: Option<&RunnerOut>, journey_id: Uuid) -> bool {
    matches!(
        history,
        Some(RunnerOut::Appearance { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::NodeLifecycle(node)) if node.uuid == journey_id
    )
}

fn is_journey_history_event(history: Option<&RunnerOut>, journey_id: Uuid) -> bool {
    matches!(
        history,
        Some(RunnerOut::Appearance { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::NodeLifecycle(node)) if node.uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::EffectInput { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::EffectSuccessOutput { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::EffectFailureOutput { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::SleepScheduled { uuid, .. }) if *uuid == journey_id
    ) || matches!(
        history,
        Some(RunnerOut::SleepFired { uuid, .. }) if *uuid == journey_id
    )
}

async fn drain_live_history_until_parent_completion(
    replay: &mut ReplayCursor,
    journey_id: Uuid,
    request_node_id: u32,
) -> Result<Option<Result<Vec<u8>, Vec<u8>>>, ExecutorError> {
    loop {
        match replay.peek().await? {
            Some(RunnerOut::EffectSuccessOutput {
                node_id,
                data,
                uuid,
            }) if uuid == journey_id && node_id == request_node_id => {
                let _ = replay.discard_front().await?;
                return Ok(Some(Ok(data)));
            }
            Some(RunnerOut::EffectFailureOutput {
                node_id,
                data,
                uuid,
            }) if uuid == journey_id && node_id == request_node_id => {
                let _ = replay.discard_front().await?;
                return Ok(Some(Err(data)));
            }
            history if is_journey_history_event(history.as_ref(), journey_id) => {
                let _ = replay.discard_front().await?;
            }
            None => return Ok(None),
            _ => return Ok(None),
        }
    }
}

fn is_terminal_journey_status(status: JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Stopped | JourneyStatus::Completed | JourneyStatus::Dead
    )
}

async fn handle_in_flight_result<T>(
    result: InFlightJourneyResult<T>,
    active_journeys: &mut HashSet<Uuid>,
    suspended: &mut HashMap<Uuid, Box<dyn SuspendedJourney<T>>>,
    client: &dyn JungleClient,
    owner_id: Uuid,
    owner_lease_ttl_ms: i64,
) -> Result<(), ExecutorError> {
    match result {
        InFlightJourneyResult::ClaimedWork {
            journey_id,
            animal_id,
            generation,
            outcome,
        } => {
            active_journeys.remove(&journey_id);
            match outcome {
                JourneyStartOutcome::NotMatched => {
                    return Err(ExecutorError::InputDeserialize(format!(
                        "unknown animal: id={animal_id}, generation={generation}"
                    )));
                }
                JourneyStartOutcome::Completed => {
                    if !is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        client.complete_journey(journey_id).await?;
                    }
                }
                JourneyStartOutcome::Failed { failure: _ } => {
                    if !is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        client.dead_journey(journey_id).await?;
                    }
                }
                JourneyStartOutcome::Sleeping {
                    wake_at_unix_ms,
                    journey,
                } => {
                    if is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        return Ok(());
                    }
                    let timer_id = Uuid::new_v4();
                    client
                        .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
                        .await?;
                    client
                        .heartbeat_journey_lease(journey_id, owner_id, owner_lease_ttl_ms)
                        .await?;
                    suspended.insert(journey_id, journey);
                }
            }
        }
        InFlightJourneyResult::ResumedWake {
            journey_id,
            journey,
            outcome,
        } => {
            active_journeys.remove(&journey_id);
            match outcome {
                SuspendedOutcome::Completed => {
                    if !is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        client.complete_journey(journey_id).await?;
                    }
                }
                SuspendedOutcome::Failed { failure: _ } => {
                    if !is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        client.dead_journey(journey_id).await?;
                    }
                }
                SuspendedOutcome::Sleeping {
                    wake_at_unix_ms,
                    node_id: _,
                } => {
                    if is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        return Ok(());
                    }
                    let timer_id = Uuid::new_v4();
                    client
                        .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
                        .await?;
                    client
                        .heartbeat_journey_lease(journey_id, owner_id, owner_lease_ttl_ms)
                        .await?;
                    suspended.insert(journey_id, journey);
                }
            }
        }
    }
    Ok(())
}

type InFlightFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<InFlightJourneyResult<T>, ExecutorError>> + Send + 'a>>;

enum InFlightJourneyResult<T> {
    ClaimedWork {
        journey_id: Uuid,
        animal_id: u32,
        generation: u32,
        outcome: JourneyStartOutcome<T>,
    },
    ResumedWake {
        journey_id: Uuid,
        journey: Box<dyn SuspendedJourney<T>>,
        outcome: SuspendedOutcome,
    },
}

fn run_claimed_work<'a, T>(
    journey_id: Uuid,
    animal_id: u32,
    generation: u32,
    seed: Vec<u8>,
    client: Box<dyn JungleClient>,
    replay_page_size: u32,
    runner: &'a JungleRunner<T>,
    tx: RunnerChannelTx,
) -> InFlightFuture<'a, T>
where
    T: Ecosystem + Send + Sync + 'static,
    T::Animals: Animals,
    <T::Animals as Animals>::List: FlattenNodes,
    SPFlatten<<T::Animals as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T::Animals>: SupportedAnimalGenerations<T>,
{
    Box::pin(async move {
        let outcome = <AnimalSet<T::Animals> as SupportedAnimalGenerations<T>>::resume_by_animal(
            animal_id,
            generation,
            seed,
            journey_id,
            client,
            replay_page_size,
            runner,
            tx,
        )
        .await?;
        Ok(InFlightJourneyResult::ClaimedWork {
            journey_id,
            animal_id,
            generation,
            outcome,
        })
    })
}

fn run_resume_suspended<'a, T>(
    journey_id: Uuid,
    mut journey: Box<dyn SuspendedJourney<T>>,
    runner: &'a JungleRunner<T>,
    tx: RunnerChannelTx,
) -> InFlightFuture<'a, T>
where
    T: Ecosystem + Send + Sync + 'static,
    T::Animals: Animals,
    <T::Animals as Animals>::List: FlattenNodes,
    SPFlatten<<T::Animals as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T::Animals>: SupportedAnimalGenerations<T>,
{
    Box::pin(async move {
        let outcome = journey.resume(runner, tx).await?;
        Ok(InFlightJourneyResult::ResumedWake {
            journey_id,
            journey,
            outcome,
        })
    })
}

pub enum JourneyStartOutcome<T> {
    NotMatched,
    Completed,
    Failed {
        failure: Failure,
    },
    Sleeping {
        wake_at_unix_ms: i64,
        journey: Box<dyn SuspendedJourney<T> + Send>,
    },
}

pub enum SuspendedOutcome {
    Completed,
    Failed { failure: Failure },
    Sleeping { wake_at_unix_ms: i64, node_id: u32 },
}

pub trait SuspendedJourney<T>: Send {
    fn resume<'a>(
        &'a mut self,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + Send + 'a>>;
}

struct SuspendedAnimalJourney<T, A>
where
    T: 'static,
    A: BoundAnimal + Observable + Perturbable + Send + Sync + 'static,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    journey_id: Uuid,
    sleep_node_id: u32,
    executor: ContextExecutor<T, A>,
}

impl<T, A> SuspendedJourney<T> for SuspendedAnimalJourney<T, A>
where
    T: Send + Sync + 'static,
    A: BoundAnimal + Observable + Perturbable + Send + Sync + 'static,
    A::State: Send + 'static,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    fn resume<'a>(
        &'a mut self,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + Send + 'a>> {
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
                RunnerAdvance::Failed { failure } => Ok(SuspendedOutcome::Failed { failure }),
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
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>;

    fn resume_by_animal<'a>(
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
        journey_id: Uuid,
        client: Box<dyn JungleClient>,
        replay_page_size: u32,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>;
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
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>
    {
        Box::pin(async { Ok(JourneyStartOutcome::NotMatched) })
    }

    fn resume_by_animal<'a>(
        _animal_id: u32,
        _generation: u32,
        _seed: Vec<u8>,
        _journey_id: Uuid,
        _client: Box<dyn JungleClient>,
        _replay_page_size: u32,
        _runner: &'a JungleRunner<T>,
        _tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>
    {
        Box::pin(async { Ok(JourneyStartOutcome::NotMatched) })
    }
}

impl<T, Head, Tail> SupportedAnimalGenerations<T> for list::List<(Head, Tail)>
where
    Head: BoundAnimal + Observable + Perturbable + Send + Sync + 'static,
    Head::Id: AnimalIdValue,
    Head::Generation: Unsigned,
    Head::Seed: Send + 'static,
    Head::State: Send + 'static,
    BoundAnimalJourney<Head>: BuildFlowWithContext<
            (Arc<T>, DynFlow<Head::State>),
            Output = (Arc<T>, DynFlow<Head::State>),
        > + ArgputForState<Head::State>,
    Head::Seed: Into<<BoundAnimalJourney<Head> as ArgputForState<Head::State>>::Carry>,
    <BoundAnimalJourney<Head> as ArgputForState<Head::State>>::Carry: Serialize + Clone + Send,
    Tail: SupportedAnimalGenerations<T>,
    T: Send + Sync + 'static,
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
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>
    {
        Box::pin(async move {
            if animal_id == <Head::Id as AnimalIdValue>::U32
                && generation == <Head::Generation as Unsigned>::U32
            {
                let seed: Head::Seed = postcard::from_bytes(&seed)
                    .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
                let initial_input: <BoundAnimalJourney<Head> as ArgputForState<Head::State>>::Carry =
                    seed.into();
                let state: Head::State = Default::default();
                let mut executor = runner.new_executor::<Head>(state);
                executor.set_journey_id(journey_id);
                let appearance = runner.initial_appearance::<Head>(&executor)?;
                runner
                    .emit_appearance(journey_id, appearance, &mut tx)
                    .await?;
                match runner
                    .drive_until_sleep_or_complete::<Head, _>(
                        &mut executor,
                        initial_input,
                        journey_id,
                        &mut tx,
                    )
                    .await?
                {
                    RunnerAdvance::Completed => Ok(JourneyStartOutcome::Completed),
                    RunnerAdvance::Failed { failure } => {
                        Ok(JourneyStartOutcome::Failed { failure })
                    }
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
        client: Box<dyn JungleClient>,
        replay_page_size: u32,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<JourneyStartOutcome<T>, ExecutorError>> + Send + 'a>>
    {
        Box::pin(async move {
            if animal_id == <Head::Id as AnimalIdValue>::U32
                && generation == <Head::Generation as Unsigned>::U32
            {
                let seed: Head::Seed = postcard::from_bytes(&seed)
                    .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
                let initial_input: <BoundAnimalJourney<Head> as ArgputForState<Head::State>>::Carry =
                    seed.into();
                let state: Head::State = Default::default();
                let mut executor = runner.new_executor::<Head>(state);
                executor.set_journey_id(journey_id);
                let mut replay = ReplayCursor::new(client, journey_id, replay_page_size);
                if let Err(err) = replay_history::<T, Head, _>(
                    &mut executor,
                    initial_input.clone(),
                    journey_id,
                    &mut replay,
                    &mut tx,
                )
                .await
                {
                    match err {
                        ExecutorError::ActionFailure(failure) => {
                            return Ok(JourneyStartOutcome::Failed { failure });
                        }
                        other => return Err(other),
                    }
                }
                let appearance = runner.initial_appearance::<Head>(&executor)?;
                runner
                    .emit_appearance(journey_id, appearance, &mut tx)
                    .await?;
                match runner
                    .drive_until_sleep_or_complete::<Head, _>(
                        &mut executor,
                        initial_input,
                        journey_id,
                        &mut tx,
                    )
                    .await?
                {
                    RunnerAdvance::Completed => Ok(JourneyStartOutcome::Completed),
                    RunnerAdvance::Failed { failure } => {
                        Ok(JourneyStartOutcome::Failed { failure })
                    }
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
                Tail::resume_by_animal(
                    animal_id,
                    generation,
                    seed,
                    journey_id,
                    client,
                    replay_page_size,
                    runner,
                    tx,
                )
                .await
            }
        })
    }
}

struct ReplayCursor {
    client: Box<dyn JungleClient>,
    journey_id: Uuid,
    after_sequence_id: Option<u64>,
    snapshot_end_sequence_id: Option<u64>,
    page_size: u32,
    buffer: VecDeque<JourneyEvent>,
    fetched_pages: u64,
    fetched_events: u64,
    exhausted: bool,
}

impl ReplayCursor {
    fn new(client: Box<dyn JungleClient>, journey_id: Uuid, page_size: u32) -> Self {
        Self {
            client,
            journey_id,
            after_sequence_id: None,
            snapshot_end_sequence_id: None,
            page_size: page_size.max(1),
            buffer: VecDeque::new(),
            fetched_pages: 0,
            fetched_events: 0,
            exhausted: false,
        }
    }

    async fn peek(&mut self) -> Result<Option<RunnerOut>, ExecutorError> {
        self.fill_buffer_if_needed().await?;
        Ok(self.buffer.front().map(|event| event.event.clone()))
    }

    async fn discard_front(&mut self) -> Result<bool, ExecutorError> {
        self.fill_buffer_if_needed().await?;
        let Some(event) = self.buffer.pop_front() else {
            return Ok(false);
        };
        self.after_sequence_id = Some(event.sequence_id);
        if self.snapshot_end_sequence_id == Some(event.sequence_id) {
            self.exhausted = true;
        }
        Ok(true)
    }

    async fn fill_buffer_if_needed(&mut self) -> Result<(), ExecutorError> {
        if !self.buffer.is_empty() || self.exhausted {
            return Ok(());
        }

        if self
            .snapshot_end_sequence_id
            .is_some_and(|snapshot_end| self.after_sequence_id >= Some(snapshot_end))
        {
            self.exhausted = true;
            return Ok(());
        }

        let fetch_started_at = Instant::now();
        let page = self
            .client
            .journey_replay_page(
                self.journey_id,
                self.after_sequence_id,
                self.snapshot_end_sequence_id,
                self.page_size,
            )
            .await?;
        let fetch_elapsed = fetch_started_at.elapsed();
        self.fetched_pages = self.fetched_pages.saturating_add(1);
        self.fetched_events = self
            .fetched_events
            .saturating_add(u64::try_from(page.events.len()).unwrap_or(u64::MAX));

        if let Some(expected) = self.snapshot_end_sequence_id {
            if page.snapshot_end_sequence_id != Some(expected) {
                return Err(ExecutorError::ClientTransport(format!(
                    "journey replay snapshot changed during cursor fetch (journey={}, expected_snapshot_end={}, actual_snapshot_end={:?})",
                    self.journey_id, expected, page.snapshot_end_sequence_id
                )));
            }
        } else {
            self.snapshot_end_sequence_id = page.snapshot_end_sequence_id;
        }

        if fetch_elapsed > WORKER_SLOW_HISTORY_WARN_THRESHOLD {
            warn!(
                journey_id = %self.journey_id,
                after_sequence_id = self.after_sequence_id.unwrap_or(0),
                snapshot_end_sequence_id = self.snapshot_end_sequence_id.unwrap_or(0),
                page_size = self.page_size,
                page_len = page.events.len(),
                fetched_pages = self.fetched_pages,
                fetched_events = self.fetched_events,
                fetch_elapsed_ms = fetch_elapsed.as_millis(),
                "slow journey replay page fetch"
            );
        } else if self.fetched_pages % WORKER_ACTIVITY_LOG_INTERVAL == 0 {
            debug!(
                journey_id = %self.journey_id,
                after_sequence_id = self.after_sequence_id.unwrap_or(0),
                snapshot_end_sequence_id = self.snapshot_end_sequence_id.unwrap_or(0),
                page_size = self.page_size,
                page_len = page.events.len(),
                fetched_pages = self.fetched_pages,
                fetched_events = self.fetched_events,
                fetch_elapsed_ms = fetch_elapsed.as_millis(),
                "journey replay page fetch heartbeat"
            );
        }

        let snapshot_end_sequence_id = self.snapshot_end_sequence_id;
        for event in page.events {
            if self
                .after_sequence_id
                .is_some_and(|after| event.sequence_id <= after)
            {
                return Err(ExecutorError::ClientTransport(format!(
                    "journey replay cursor received stale event (journey={}, after_sequence_id={}, event_sequence_id={})",
                    self.journey_id,
                    self.after_sequence_id.unwrap_or(0),
                    event.sequence_id
                )));
            }
            if snapshot_end_sequence_id.is_some_and(|snapshot_end| event.sequence_id > snapshot_end)
            {
                return Err(ExecutorError::ClientTransport(format!(
                    "journey replay cursor received event beyond snapshot end (journey={}, snapshot_end_sequence_id={}, event_sequence_id={})",
                    self.journey_id,
                    snapshot_end_sequence_id.unwrap_or(0),
                    event.sequence_id
                )));
            }
            self.buffer.push_back(event);
        }

        if self.buffer.is_empty() {
            if self.snapshot_end_sequence_id.is_none()
                || self
                    .snapshot_end_sequence_id
                    .is_some_and(|snapshot_end| self.after_sequence_id >= Some(snapshot_end))
            {
                self.exhausted = true;
                return Ok(());
            }

            return Err(ExecutorError::ClientTransport(format!(
                "journey replay cursor hit empty page before reaching snapshot end (journey={}, after_sequence_id={}, snapshot_end_sequence_id={})",
                self.journey_id,
                self.after_sequence_id.unwrap_or(0),
                self.snapshot_end_sequence_id.unwrap_or(0)
            )));
        }

        Ok(())
    }
}

async fn replay_history<T, A, Initial>(
    executor: &mut ContextExecutor<T, A>,
    initial_input: Initial,
    journey_id: Uuid,
    replay: &mut ReplayCursor,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: BoundAnimal + Observable + Perturbable,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    Initial: Serialize + Clone,
{
    let no_effect_type = core::any::type_name::<NoEffect>();
    let sleep_effect_type = core::any::type_name::<Sleep>();
    while !executor.is_complete() {
        if replay.peek().await?.is_none() {
            break;
        }

        let request = match executor.next_executable_request(initial_input.clone()) {
            Ok(request) => request,
            Err(ExecutorError::Complete) => break,
            Err(err) => return Err(err),
        };
        let request_node_id = request.node_id();
        let expected_input = request.request_bytes();
        let effect_type = request.effect_type();
        let request_has_live_history = request.has_live_history();

        // Appearance snapshots are informational and can interleave with step history.
        while is_informational_history_event(replay.peek().await?.as_ref(), journey_id) {
            let _ = replay.discard_front().await?;
        }
        // Non-sleep requests should ignore stale sleep bookkeeping records that may remain
        // after restart recovery.
        while effect_type != sleep_effect_type
            && (matches!(
                replay.peek().await?,
                Some(RunnerOut::SleepScheduled { uuid, .. }) if uuid == journey_id
            ) || matches!(
                replay.peek().await?,
                Some(RunnerOut::SleepFired { uuid, .. }) if uuid == journey_id
            ))
        {
            let _ = replay.discard_front().await?;
        }

        let current_event = replay.peek().await?;
        let matched_effect_input = matches!(
            current_event.as_ref(),
            Some(RunnerOut::EffectInput {
                node_id,
                data,
                uuid
            }) if *uuid == journey_id && *node_id == request_node_id && data.as_slice() == expected_input
        );
        if matched_effect_input {
            let _ = replay.discard_front().await?;
        } else {
            // Recovery path: tolerate history records where EffectInput is missing but completion/sleep bookkeeping exists.
            let has_recoverable_cursor_event = matches!(
                current_event.as_ref(),
                Some(RunnerOut::EffectSuccessOutput { node_id, uuid, .. })
                    if *uuid == journey_id && *node_id == request_node_id
            ) || matches!(
                current_event.as_ref(),
                Some(RunnerOut::EffectFailureOutput { node_id, uuid, .. })
                    if *uuid == journey_id && *node_id == request_node_id
            ) || (effect_type == sleep_effect_type
                && matches!(
                    current_event.as_ref(),
                    Some(RunnerOut::SleepScheduled { uuid, .. }) if *uuid == journey_id
                ))
                || (effect_type == sleep_effect_type
                    && matches!(
                        current_event.as_ref(),
                    Some(RunnerOut::SleepFired { uuid, .. }) if *uuid == journey_id
                    ));

            if effect_type == no_effect_type
                || has_recoverable_cursor_event
                || (request_has_live_history
                    && is_journey_history_event(current_event.as_ref(), journey_id))
            {
                send_recovered_effect_input(tx, journey_id, request_node_id, expected_input)
                    .await?;
            } else {
                return Err(ExecutorError::ClientTransport(
                    format!(
                        "history replay expected EffectInput event (journey={journey_id}, node_id={request_node_id}, effect_type={effect_type}, after_sequence_id={:?}, history_event={current_event:?})",
                        replay.after_sequence_id
                    ),
                ));
            }
        }

        // Appearance snapshots can also be emitted after completion events.
        while is_informational_history_event(replay.peek().await?.as_ref(), journey_id) {
            let _ = replay.discard_front().await?;
        }

        let completion = if effect_type == sleep_effect_type {
            while matches!(
                replay.peek().await?,
                Some(RunnerOut::SleepScheduled { uuid, .. }) if uuid == journey_id
            ) || matches!(
                replay.peek().await?,
                Some(RunnerOut::NodeLifecycle(node)) if node.uuid == journey_id
            ) || matches!(
                replay.peek().await?,
                Some(RunnerOut::Appearance { uuid, .. }) if uuid == journey_id
            ) {
                let _ = replay.discard_front().await?;
            }

            match replay.peek().await? {
                Some(RunnerOut::SleepFired { uuid, .. }) if uuid == journey_id => {
                    let _ = replay.discard_front().await?;
                    while matches!(
                        replay.peek().await?,
                        Some(RunnerOut::EffectSuccessOutput { node_id, uuid, .. })
                            if uuid == journey_id && node_id == request_node_id
                    ) || matches!(
                        replay.peek().await?,
                        Some(RunnerOut::EffectFailureOutput { node_id, uuid, .. })
                            if uuid == journey_id && node_id == request_node_id
                    ) {
                        let _ = replay.discard_front().await?;
                    }
                    let sleep_out = postcard::to_allocvec(&())
                        .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                    Ok(sleep_out)
                }
                Some(RunnerOut::EffectSuccessOutput {
                    node_id,
                    data,
                    uuid,
                }) if uuid == journey_id && node_id == request_node_id => {
                    let _ = replay.discard_front().await?;
                    Ok(data)
                }
                Some(RunnerOut::EffectFailureOutput {
                    node_id,
                    data,
                    uuid,
                }) if uuid == journey_id && node_id == request_node_id => {
                    let _ = replay.discard_front().await?;
                    Err(data)
                }
                _ => {
                    let completion = request.run().await?;
                    send_recovered_completion(tx, journey_id, request_node_id, &completion).await?;
                    completion
                }
            }
        } else {
            if request_has_live_history {
                if let Some(completion) =
                    drain_live_history_until_parent_completion(replay, journey_id, request_node_id)
                        .await?
                {
                    completion
                } else {
                    let completion = request.run().await?;
                    send_recovered_completion(tx, journey_id, request_node_id, &completion).await?;
                    completion
                }
            } else {
                match replay.peek().await? {
                    Some(RunnerOut::EffectSuccessOutput {
                        node_id,
                        data,
                        uuid,
                    }) if uuid == journey_id && node_id == request_node_id => {
                        let _ = replay.discard_front().await?;
                        Ok(data)
                    }
                    Some(RunnerOut::EffectFailureOutput {
                        node_id,
                        data,
                        uuid,
                    }) if uuid == journey_id && node_id == request_node_id => {
                        let _ = replay.discard_front().await?;
                        Err(data)
                    }
                    _ => {
                        let completion = request.run().await?;
                        send_recovered_completion(tx, journey_id, request_node_id, &completion)
                            .await?;
                        completion
                    }
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
        Ok(data) => RunnerOut::EffectSuccessOutput {
            node_id,
            data: data.clone(),
            uuid: journey_id,
        },
        Err(data) => RunnerOut::EffectFailureOutput {
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

async fn send_recovered_effect_input(
    tx: &mut RunnerChannelTx,
    journey_id: Uuid,
    node_id: u32,
    data: &[u8],
) -> Result<(), ExecutorError> {
    let out = RunnerOut::EffectInput {
        node_id,
        data: data.to_vec(),
        uuid: journey_id,
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
            "runner expected ack response while replaying recovered effect input".to_string(),
        )),
    }
}
