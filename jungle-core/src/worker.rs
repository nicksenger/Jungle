use crate::runner::{JungleRunner, PendingSleep, RunnerAdvance};
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use jungle_client::{JungleClient, RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    AnimalIdValue, AnimalSet, Animals, ArgputForState, BoundAnimal, BoundAnimalJourney,
    BuildFlowWithContext, ContextExecutor, DynFlow, Ecosystem, ExecutableEffectRequest,
    ExecutorError, Failure, JourneyEvent, JourneyStatus, NoEffect, NodeLifecycle, Observable,
    ObservationBridge, OwnerWake, Perturbable, PerturbationBridge, RunnerOut, Sleep,
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
        let mut pending_wakes = HashMap::<Uuid, VecDeque<Uuid>>::new();
        let mut pending_wake_journeys = VecDeque::<Uuid>::new();
        let mut pending_wake_journey_ids = HashSet::<Uuid>::new();
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
                        pending_wakes.remove(&journey_id);
                        pending_wake_journey_ids.remove(&journey_id);
                        pending_wake_journeys.retain(|id| *id != journey_id);
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
                    enqueue_owner_wake(
                        &mut pending_wakes,
                        &mut pending_wake_journeys,
                        &mut pending_wake_journey_ids,
                        wake,
                    );
                }

                while in_flight.len() < self.max_in_flight_journeys {
                    if let Some(wake_journey_id) = pending_wake_journeys.pop_front() {
                        pending_wake_journey_ids.remove(&wake_journey_id);
                        if active_journeys.contains(&wake_journey_id) {
                            if pending_wakes
                                .get(&wake_journey_id)
                                .is_some_and(|timer_ids| !timer_ids.is_empty())
                            {
                                pending_wake_journey_ids.insert(wake_journey_id);
                                pending_wake_journeys.push_back(wake_journey_id);
                            }
                            continue;
                        }
                        if is_terminal_journey_status(
                            self.client.journey_details(wake_journey_id).await?,
                        ) {
                            suspended.remove(&wake_journey_id);
                            pending_wakes.remove(&wake_journey_id);
                            continue;
                        }
                        if let Some(journey) = suspended.remove(&wake_journey_id) {
                            let Some(timer_id) = pending_wakes
                                .get_mut(&wake_journey_id)
                                .and_then(|timer_ids| timer_ids.pop_front())
                            else {
                                continue;
                            };
                            if pending_wakes
                                .get(&wake_journey_id)
                                .is_some_and(|timer_ids| timer_ids.is_empty())
                            {
                                pending_wakes.remove(&wake_journey_id);
                            }
                            active_journeys.insert(wake_journey_id);
                            in_flight.push(run_resume_suspended(
                                wake_journey_id,
                                timer_id,
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
                    &mut pending_wakes,
                    &mut pending_wake_journeys,
                    &mut pending_wake_journey_ids,
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
                        &mut pending_wakes,
                        &mut pending_wake_journeys,
                        &mut pending_wake_journey_ids,
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
        RunnerOut::PerturbationApplied { .. } => "perturbation_applied",
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

fn is_terminal_journey_status(status: JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Stopped | JourneyStatus::Completed | JourneyStatus::Dead
    )
}

fn enqueue_owner_wake(
    pending_wakes: &mut HashMap<Uuid, VecDeque<Uuid>>,
    pending_wake_journeys: &mut VecDeque<Uuid>,
    pending_wake_journey_ids: &mut HashSet<Uuid>,
    wake: OwnerWake,
) {
    pending_wakes
        .entry(wake.journey_id)
        .or_default()
        .push_back(wake.timer_id);
    if pending_wake_journey_ids.insert(wake.journey_id) {
        pending_wake_journeys.push_back(wake.journey_id);
    }
}

async fn handle_in_flight_result<T>(
    result: InFlightJourneyResult<T>,
    active_journeys: &mut HashSet<Uuid>,
    suspended: &mut HashMap<Uuid, Box<dyn SuspendedJourney<T>>>,
    pending_wakes: &mut HashMap<Uuid, VecDeque<Uuid>>,
    pending_wake_journeys: &mut VecDeque<Uuid>,
    pending_wake_journey_ids: &mut HashSet<Uuid>,
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
                    sleeps,
                    mut journey,
                } => {
                    if is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        return Ok(());
                    }
                    for sleep in sleeps {
                        let timer_id = Uuid::new_v4();
                        client
                            .schedule_sleep_timer(journey_id, timer_id, sleep.wake_at_unix_ms)
                            .await?;
                        journey.record_sleep(timer_id, sleep);
                    }
                    client
                        .heartbeat_journey_lease(journey_id, owner_id, owner_lease_ttl_ms)
                        .await?;
                    suspended.insert(journey_id, journey);
                    if pending_wakes
                        .get(&journey_id)
                        .is_some_and(|timer_ids| !timer_ids.is_empty())
                        && pending_wake_journey_ids.insert(journey_id)
                    {
                        pending_wake_journeys.push_back(journey_id);
                    }
                }
            }
        }
        InFlightJourneyResult::ResumedWake {
            journey_id,
            mut journey,
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
                SuspendedOutcome::Sleeping { new_sleeps } => {
                    if is_terminal_journey_status(client.journey_details(journey_id).await?) {
                        return Ok(());
                    }
                    for sleep in new_sleeps {
                        let timer_id = Uuid::new_v4();
                        client
                            .schedule_sleep_timer(journey_id, timer_id, sleep.wake_at_unix_ms)
                            .await?;
                        journey.record_sleep(timer_id, sleep);
                    }
                    client
                        .heartbeat_journey_lease(journey_id, owner_id, owner_lease_ttl_ms)
                        .await?;
                    suspended.insert(journey_id, journey);
                    if pending_wakes
                        .get(&journey_id)
                        .is_some_and(|timer_ids| !timer_ids.is_empty())
                        && pending_wake_journey_ids.insert(journey_id)
                    {
                        pending_wake_journeys.push_back(journey_id);
                    }
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
    timer_id: Uuid,
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
        let outcome = journey.resume(timer_id, runner, tx).await?;
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
        sleeps: Vec<PendingSleep>,
        journey: Box<dyn SuspendedJourney<T> + Send>,
    },
}

pub enum SuspendedOutcome {
    Completed,
    Failed { failure: Failure },
    Sleeping { new_sleeps: Vec<PendingSleep> },
}

pub trait SuspendedJourney<T>: Send {
    fn record_sleep(&mut self, timer_id: Uuid, sleep: PendingSleep);

    fn resume<'a>(
        &'a mut self,
        timer_id: Uuid,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
struct ScheduledSleep {
    timer_id: Uuid,
    sleep: PendingSleep,
}

struct SuspendedAnimalJourney<T, A>
where
    T: 'static,
    A: BoundAnimal + Observable + Perturbable + Send + Sync + 'static,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    journey_id: Uuid,
    pending_sleeps: Vec<ScheduledSleep>,
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
    fn record_sleep(&mut self, timer_id: Uuid, sleep: PendingSleep) {
        self.pending_sleeps.push(ScheduledSleep { timer_id, sleep });
    }

    fn resume<'a>(
        &'a mut self,
        timer_id: Uuid,
        runner: &'a JungleRunner<T>,
        mut tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<SuspendedOutcome, ExecutorError>> + Send + 'a>> {
        Box::pin(async move {
            let sleep_index = self
                .pending_sleeps
                .iter()
                .position(|scheduled| scheduled.timer_id == timer_id)
                .ok_or_else(|| {
                    ExecutorError::ClientTransport(format!(
                        "unknown sleep wake timer for journey {}: {timer_id}",
                        self.journey_id
                    ))
                })?;
            let scheduled = self.pending_sleeps.remove(sleep_index);
            let pending_sleeps = self
                .pending_sleeps
                .iter()
                .map(|scheduled| scheduled.sleep.clone())
                .collect::<Vec<_>>();
            let advance = runner
                .resume_after_sleep::<A>(
                    &mut self.executor,
                    self.journey_id,
                    scheduled.sleep,
                    pending_sleeps.clone(),
                    &mut tx,
                )
                .await?;
            match advance {
                RunnerAdvance::Completed => Ok(SuspendedOutcome::Completed),
                RunnerAdvance::Failed { failure } => Ok(SuspendedOutcome::Failed { failure }),
                RunnerAdvance::SuspendedSleep { sleeps } => {
                    let mut unmatched_pending = pending_sleeps;
                    let mut new_sleeps = Vec::new();
                    for sleep in sleeps {
                        if let Some(index) = unmatched_pending
                            .iter()
                            .position(|pending| *pending == sleep)
                        {
                            unmatched_pending.remove(index);
                        } else {
                            new_sleeps.push(sleep);
                        }
                    }
                    if !unmatched_pending.is_empty() {
                        return Err(ExecutorError::ClientTransport(format!(
                            "runner dropped pending sleeps for journey {}",
                            self.journey_id
                        )));
                    }
                    Ok(SuspendedOutcome::Sleeping { new_sleeps })
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
                    RunnerAdvance::SuspendedSleep { sleeps } => {
                        let suspended = SuspendedAnimalJourney::<T, Head> {
                            journey_id,
                            pending_sleeps: Vec::new(),
                            executor,
                        };
                        Ok(JourneyStartOutcome::Sleeping {
                            sleeps,
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
                let mut replay_visible_appearance = client.animal_appearance(journey_id).await?;
                let mut replay = ReplayCursor::new(client, journey_id, replay_page_size);
                let replay_pending = match replay_history::<T, Head, _>(
                    &mut executor,
                    initial_input.clone(),
                    journey_id,
                    &mut replay,
                    &mut tx,
                    &mut replay_visible_appearance,
                )
                .await
                {
                    Ok(pending_requests) => pending_requests,
                    Err(err) => match err {
                        ExecutorError::ActionFailure(failure) => {
                            return Ok(JourneyStartOutcome::Failed { failure });
                        }
                        other => return Err(other),
                    },
                };
                let mut replay_pending_sleeps = replay_pending
                    .scheduled_sleeps
                    .iter()
                    .map(|scheduled| scheduled.sleep.clone())
                    .collect::<Vec<_>>();
                replay_pending_sleeps.extend(replay_pending.unscheduled_sleeps.iter().cloned());
                let replay_pending_sleeps_snapshot = replay_pending_sleeps.clone();
                match runner
                    .drive_until_sleep_or_complete_with_replay_pending::<Head, _>(
                        &mut executor,
                        initial_input,
                        journey_id,
                        &mut tx,
                        replay_pending_sleeps,
                        replay_pending.requests,
                        replay_visible_appearance,
                    )
                    .await?
                {
                    RunnerAdvance::Completed => Ok(JourneyStartOutcome::Completed),
                    RunnerAdvance::Failed { failure } => {
                        Ok(JourneyStartOutcome::Failed { failure })
                    }
                    RunnerAdvance::SuspendedSleep { sleeps } => {
                        let mut unmatched_pending = replay_pending_sleeps_snapshot;
                        let mut scheduled_sleeps = replay_pending.scheduled_sleeps;
                        let mut replayed_sleeps = Vec::new();
                        let mut new_sleeps = Vec::new();
                        for sleep in sleeps {
                            if let Some(index) = unmatched_pending
                                .iter()
                                .position(|pending| *pending == sleep)
                            {
                                unmatched_pending.remove(index);
                                if let Some(scheduled_index) = scheduled_sleeps
                                    .iter()
                                    .position(|scheduled| scheduled.sleep == sleep)
                                {
                                    replayed_sleeps.push(scheduled_sleeps.remove(scheduled_index));
                                } else {
                                    new_sleeps.push(sleep);
                                }
                            } else {
                                new_sleeps.push(sleep);
                            }
                        }
                        if !unmatched_pending.is_empty() {
                            return Err(ExecutorError::ClientTransport(format!(
                                "runner dropped replay-recovered pending sleeps for journey {}",
                                journey_id
                            )));
                        }
                        let suspended = SuspendedAnimalJourney::<T, Head> {
                            journey_id,
                            pending_sleeps: replayed_sleeps,
                            executor,
                        };
                        Ok(JourneyStartOutcome::Sleeping {
                            sleeps: new_sleeps,
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

#[derive(Default)]
struct ReplayPendingRequests {
    requests: Vec<ExecutableEffectRequest>,
    scheduled_sleeps: Vec<ScheduledSleep>,
    unscheduled_sleeps: Vec<PendingSleep>,
}

#[derive(Debug, Clone, Copy)]
struct ReplaySleepSchedule {
    timer_id: Uuid,
    wake_at_unix_ms: i64,
}

async fn replay_history<T, A, Initial>(
    executor: &mut ContextExecutor<T, A>,
    initial_input: Initial,
    journey_id: Uuid,
    replay: &mut ReplayCursor,
    tx: &mut RunnerChannelTx,
    replay_visible_appearance: &mut Option<Vec<u8>>,
) -> Result<ReplayPendingRequests, ExecutorError>
where
    T: 'static,
    A: BoundAnimal + Observable + Perturbable,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
    Initial: Serialize + Clone,
{
    let no_effect_type = core::any::type_name::<NoEffect>();
    let sleep_effect_type = core::any::type_name::<Sleep>();
    let mut pending = HashMap::<u32, ReplayPendingRequest>::new();
    let mut pending_order = VecDeque::<u32>::new();
    let mut pending_sleep_schedules = VecDeque::<ReplaySleepSchedule>::new();
    while !executor.is_complete() || !pending.is_empty() {
        if pending.is_empty() && replay.peek().await?.is_none() {
            break;
        }
        loop {
            apply_replay_perturbations::<T, A>(executor, replay, tx, journey_id).await?;
            let current_event = replay.peek().await?;
            if replay_event_completes_pending_request(
                current_event.as_ref(),
                journey_id,
                &pending,
                sleep_effect_type,
            ) {
                break;
            }

            let request = match executor.next_executable_request(initial_input.clone()) {
                Ok(request) => request,
                Err(ExecutorError::Complete) | Err(ExecutorError::AwaitingCompletion) => break,
                Err(err) => return Err(err),
            };
            let request_node_id = request.node_id();
            let request_effect_type = request.effect_type();
            let request_has_live_history = request.has_live_history();
            reconcile_replay_effect_input(
                replay,
                tx,
                journey_id,
                request_node_id,
                request.request_bytes(),
                request_effect_type,
                request_has_live_history,
                no_effect_type,
                sleep_effect_type,
            )
            .await?;
            pending_order.push_back(request_node_id);
            pending.insert(
                request_node_id,
                ReplayPendingRequest {
                    request,
                    effect_type: request_effect_type,
                    has_live_history: request_has_live_history,
                },
            );
        }

        if replay.peek().await?.is_none() {
            return take_replay_pending_requests(
                pending,
                pending_order,
                sleep_effect_type,
                pending_sleep_schedules,
            );
        }

        let Some((_, completion, recovered_live_completion)) = resolve_next_replay_completion(
            replay,
            tx,
            journey_id,
            &mut pending,
            &mut pending_order,
            sleep_effect_type,
            &mut pending_sleep_schedules,
        )
        .await?
        else {
            break;
        };

        let _emitted = executor.complete_serialized(completion)?;
        if recovered_live_completion {
            let lifecycle_updates = executor.take_node_lifecycle_updates();
            let appearance = <<A as Observable>::Observation as ObservationBridge<A>>::snapshot(
                executor.state(),
            )?;
            if should_emit_replay_live_edge(
                replay_visible_appearance.as_deref(),
                appearance.as_deref(),
            ) {
                send_executor_lifecycle_updates(lifecycle_updates, tx).await?;
                emit_live_edge_appearance(journey_id, appearance.clone(), tx).await?;
                *replay_visible_appearance = appearance;
            }
        }
    }

    Ok(ReplayPendingRequests::default())
}

struct ReplayPendingRequest {
    request: ExecutableEffectRequest,
    effect_type: &'static str,
    has_live_history: bool,
}

async fn discard_replay_informational_events(
    replay: &mut ReplayCursor,
    journey_id: Uuid,
) -> Result<(), ExecutorError> {
    while is_informational_history_event(replay.peek().await?.as_ref(), journey_id) {
        let _ = replay.discard_front().await?;
    }
    Ok(())
}

async fn apply_replay_perturbations<T, A>(
    executor: &mut ContextExecutor<T, A>,
    replay: &mut ReplayCursor,
    tx: &mut RunnerChannelTx,
    journey_id: Uuid,
) -> Result<(), ExecutorError>
where
    T: 'static,
    A: BoundAnimal + Perturbable,
    BoundAnimalJourney<A>:
        BuildFlowWithContext<(Arc<T>, DynFlow<A::State>), Output = (Arc<T>, DynFlow<A::State>)>,
{
    loop {
        discard_replay_informational_events(replay, journey_id).await?;
        let Some(RunnerOut::PerturbationApplied {
            uuid,
            perturbation_id,
            data,
        }) = replay.peek().await?
        else {
            return Ok(());
        };
        if uuid != journey_id {
            return Err(ExecutorError::ClientTransport(format!(
                "history replay found perturbation for another journey (journey={journey_id}, history_journey={uuid})"
            )));
        }

        let applied = <<A as Perturbable>::Perturbation as PerturbationBridge<A>>::apply(
            executor.state_mut(),
            &data,
        )?;
        if !applied {
            return Err(ExecutorError::ClientTransport(format!(
                "history replay found perturbation for an animal that does not support perturbations (journey={journey_id}, perturbation_id={perturbation_id})"
            )));
        }
        let _ = replay.discard_front().await?;

        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::AckPerturbable {
                journey_id,
                perturbation_id,
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
                    "worker expected ack response for replayed perturbation".to_string(),
                ));
            }
        }
    }
}

fn replay_event_completes_pending_request(
    event: Option<&RunnerOut>,
    journey_id: Uuid,
    pending: &HashMap<u32, ReplayPendingRequest>,
    sleep_effect_type: &'static str,
) -> bool {
    match event {
        Some(RunnerOut::EffectSuccessOutput { node_id, uuid, .. })
        | Some(RunnerOut::EffectFailureOutput { node_id, uuid, .. }) => {
            *uuid == journey_id && pending.contains_key(node_id)
        }
        Some(RunnerOut::SleepFired { uuid, .. }) if *uuid == journey_id => pending
            .values()
            .any(|request| request.effect_type == sleep_effect_type),
        _ => false,
    }
}

async fn reconcile_replay_effect_input(
    replay: &mut ReplayCursor,
    tx: &mut RunnerChannelTx,
    journey_id: Uuid,
    request_node_id: u32,
    expected_input: &[u8],
    effect_type: &'static str,
    request_has_live_history: bool,
    no_effect_type: &'static str,
    sleep_effect_type: &'static str,
) -> Result<(), ExecutorError> {
    while is_informational_history_event(replay.peek().await?.as_ref(), journey_id) {
        let _ = replay.discard_front().await?;
    }
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
    // Reconnect recovery can append synthesized replay events after the original
    // cursor position. On later replays these can become stale orphan entries that
    // should not abort reconciliation for the next executable request.
    loop {
        match replay.peek().await? {
            Some(RunnerOut::EffectInput {
                node_id,
                data,
                uuid,
            }) if uuid == journey_id => {
                if node_id == request_node_id && data.as_slice() == expected_input {
                    break;
                }
                let _ = replay.discard_front().await?;
            }
            Some(RunnerOut::EffectSuccessOutput { node_id, uuid, .. })
                if uuid == journey_id && node_id != request_node_id =>
            {
                let _ = replay.discard_front().await?;
            }
            Some(RunnerOut::EffectFailureOutput { node_id, uuid, .. })
                if uuid == journey_id && node_id != request_node_id =>
            {
                let _ = replay.discard_front().await?;
            }
            _ => break,
        }
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
        return Ok(());
    }

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

    if current_event.is_none()
        || effect_type == no_effect_type
        || has_recoverable_cursor_event
        || (request_has_live_history
            && is_journey_history_event(current_event.as_ref(), journey_id))
    {
        send_recovered_effect_input(tx, journey_id, request_node_id, expected_input).await
    } else {
        Err(ExecutorError::ClientTransport(format!(
            "history replay expected EffectInput event (journey={journey_id}, node_id={request_node_id}, effect_type={effect_type}, after_sequence_id={:?}, history_event={current_event:?})",
            replay.after_sequence_id
        )))
    }
}

async fn resolve_next_replay_completion(
    replay: &mut ReplayCursor,
    tx: &mut RunnerChannelTx,
    journey_id: Uuid,
    pending: &mut HashMap<u32, ReplayPendingRequest>,
    pending_order: &mut VecDeque<u32>,
    sleep_effect_type: &'static str,
    pending_sleep_schedules: &mut VecDeque<ReplaySleepSchedule>,
) -> Result<Option<(u32, Result<Vec<u8>, Vec<u8>>, bool)>, ExecutorError> {
    loop {
        let Some(current_event) = replay.peek().await? else {
            return recover_oldest_replay_completion(tx, journey_id, pending, pending_order)
                .await
                .map(|completion| {
                    completion.map(|(node_id, completion)| (node_id, completion, true))
                });
        };

        if is_informational_history_event(Some(&current_event), journey_id) {
            let _ = replay.discard_front().await?;
            continue;
        }

        match current_event {
            RunnerOut::EffectSuccessOutput {
                node_id,
                data,
                uuid,
            } if uuid == journey_id && pending.contains_key(&node_id) => {
                let _ = replay.discard_front().await?;
                remove_pending_order(pending_order, node_id);
                let _ = pending.remove(&node_id);
                return Ok(Some((node_id, Ok(data), false)));
            }
            RunnerOut::EffectFailureOutput {
                node_id,
                data,
                uuid,
            } if uuid == journey_id && pending.contains_key(&node_id) => {
                let _ = replay.discard_front().await?;
                remove_pending_order(pending_order, node_id);
                let _ = pending.remove(&node_id);
                return Ok(Some((node_id, Err(data), false)));
            }
            RunnerOut::SleepScheduled {
                uuid,
                timer_id,
                wake_at_unix_ms,
            } if uuid == journey_id => {
                if pending
                    .values()
                    .any(|request| request.effect_type == sleep_effect_type)
                {
                    pending_sleep_schedules.push_back(ReplaySleepSchedule {
                        timer_id,
                        wake_at_unix_ms,
                    });
                }
                let _ = replay.discard_front().await?;
            }
            RunnerOut::SleepFired { uuid, timer_id, .. } if uuid == journey_id => {
                let Some(node_id) =
                    take_pending_sleep_request_node(pending, pending_order, sleep_effect_type)
                else {
                    let _ = replay.discard_front().await?;
                    continue;
                };
                if let Some(index) = pending_sleep_schedules
                    .iter()
                    .position(|scheduled| scheduled.timer_id == timer_id)
                {
                    let _ = pending_sleep_schedules.remove(index);
                } else {
                    let _ = pending_sleep_schedules.pop_front();
                }
                let _ = replay.discard_front().await?;
                while matches!(
                    replay.peek().await?,
                    Some(RunnerOut::EffectSuccessOutput {
                        node_id: completion_node_id,
                        uuid,
                        ..
                    }) if uuid == journey_id && completion_node_id == node_id
                ) || matches!(
                    replay.peek().await?,
                    Some(RunnerOut::EffectFailureOutput {
                        node_id: completion_node_id,
                        uuid,
                        ..
                    }) if uuid == journey_id && completion_node_id == node_id
                ) {
                    let _ = replay.discard_front().await?;
                }
                let sleep_out = postcard::to_allocvec(&())
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                return Ok(Some((node_id, Ok(sleep_out), false)));
            }
            RunnerOut::EffectInput { uuid, .. } if uuid == journey_id => {
                let _ = replay.discard_front().await?;
            }
            RunnerOut::EffectSuccessOutput { uuid, .. } if uuid == journey_id => {
                let _ = replay.discard_front().await?;
            }
            RunnerOut::EffectFailureOutput { uuid, .. } if uuid == journey_id => {
                let _ = replay.discard_front().await?;
            }
            history
                if pending.values().any(|request| request.has_live_history)
                    && is_journey_history_event(Some(&history), journey_id) =>
            {
                let _ = replay.discard_front().await?;
            }
            _ => {
                return recover_oldest_replay_completion(tx, journey_id, pending, pending_order)
                    .await
                    .map(|completion| {
                        completion.map(|(node_id, completion)| (node_id, completion, true))
                    });
            }
        }
    }
}

async fn emit_live_edge_appearance(
    journey_id: Uuid,
    appearance: Option<Vec<u8>>,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError> {
    if let Some(appearance) = appearance {
        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::History(RunnerOut::Appearance {
                data: appearance,
                uuid: journey_id,
            }),
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
                    "runner expected ack response while emitting replay live-edge appearance"
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
}

async fn send_executor_lifecycle_updates(
    lifecycle_updates: Vec<NodeLifecycle>,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError> {
    for update in lifecycle_updates {
        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::History(RunnerOut::NodeLifecycle(update)),
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
                    "runner expected ack response while replaying lifecycle updates".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn remove_pending_order(pending_order: &mut VecDeque<u32>, node_id: u32) {
    pending_order.retain(|pending_node_id| *pending_node_id != node_id);
}

fn take_pending_sleep_request_node(
    pending: &mut HashMap<u32, ReplayPendingRequest>,
    pending_order: &mut VecDeque<u32>,
    sleep_effect_type: &'static str,
) -> Option<u32> {
    let node_id = pending_order.iter().copied().find(|pending_node_id| {
        pending
            .get(pending_node_id)
            .is_some_and(|request| request.effect_type == sleep_effect_type)
    })?;
    remove_pending_order(pending_order, node_id);
    let _ = pending.remove(&node_id);
    Some(node_id)
}

async fn recover_oldest_replay_completion(
    tx: &mut RunnerChannelTx,
    _journey_id: Uuid,
    pending: &mut HashMap<u32, ReplayPendingRequest>,
    pending_order: &mut VecDeque<u32>,
) -> Result<Option<(u32, Result<Vec<u8>, Vec<u8>>)>, ExecutorError> {
    let pending_len = pending_order.len();
    for _ in 0..pending_len {
        let Some(node_id) = pending_order.pop_front() else {
            break;
        };
        let Some(request) = pending.remove(&node_id) else {
            continue;
        };
        if request.effect_type == core::any::type_name::<Sleep>() {
            pending.insert(node_id, request);
            pending_order.push_back(node_id);
            continue;
        }
        let completion = request.request.run().await?;
        send_recovered_completion(tx, _journey_id, node_id, &completion).await?;
        return Ok(Some((node_id, completion)));
    }
    Ok(None)
}

fn should_emit_replay_live_edge(previous: Option<&[u8]>, next: Option<&[u8]>) -> bool {
    match (previous, next) {
        (None, Some(_)) => true,
        (Some(previous), Some(next)) => next != previous && next.starts_with(previous),
        _ => false,
    }
}

fn take_replay_pending_requests(
    mut pending: HashMap<u32, ReplayPendingRequest>,
    mut pending_order: VecDeque<u32>,
    sleep_effect_type: &'static str,
    mut pending_sleep_schedules: VecDeque<ReplaySleepSchedule>,
) -> Result<ReplayPendingRequests, ExecutorError> {
    let mut requests = Vec::with_capacity(pending.len());
    let mut scheduled_sleeps = Vec::new();
    let mut unscheduled_sleeps = Vec::new();
    while let Some(node_id) = pending_order.pop_front() {
        let Some(request) = pending.remove(&node_id) else {
            continue;
        };
        if request.effect_type == sleep_effect_type {
            let schedule = pending_sleep_schedules.pop_front();
            let sleep = recover_pending_sleep(request.request, schedule.as_ref())?;
            if let Some(schedule) = schedule {
                scheduled_sleeps.push(ScheduledSleep {
                    timer_id: schedule.timer_id,
                    sleep,
                });
            } else {
                unscheduled_sleeps.push(sleep);
            }
            continue;
        }
        requests.push(request.request);
    }
    for request in pending.into_values().map(|request| request.request) {
        if request.effect_type() == sleep_effect_type {
            let schedule = pending_sleep_schedules.pop_front();
            let sleep = recover_pending_sleep(request, schedule.as_ref())?;
            if let Some(schedule) = schedule {
                scheduled_sleeps.push(ScheduledSleep {
                    timer_id: schedule.timer_id,
                    sleep,
                });
            } else {
                unscheduled_sleeps.push(sleep);
            }
            continue;
        }
        requests.push(request);
    }
    Ok(ReplayPendingRequests {
        requests,
        scheduled_sleeps,
        unscheduled_sleeps,
    })
}

fn recover_pending_sleep(
    request: ExecutableEffectRequest,
    schedule: Option<&ReplaySleepSchedule>,
) -> Result<PendingSleep, ExecutorError> {
    let node_id = request.node_id();
    let duration: Duration = request.deserialize_request()?;
    let duration_millis = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    let wake_at_unix_ms = schedule.map_or_else(
        || {
            chrono::Utc::now()
                .timestamp_millis()
                .saturating_add(duration_millis)
        },
        |scheduled| scheduled.wake_at_unix_ms,
    );
    let completion = match request.suspended_completion() {
        Some(completion) => completion.clone(),
        None => Ok(postcard::to_allocvec(&())
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?),
    };
    Ok(PendingSleep {
        wake_at_unix_ms,
        node_id,
        completion,
    })
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
