use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
#[cfg(any(feature = "postgres", feature = "redb"))]
use jungle_persist::{JungleStore, Kind, StoreBuilder};
#[cfg(any(feature = "postgres", feature = "redb"))]
use jungle_types::JourneyStatus;
use jungle_types::{BackendError, WireIn, WireOut};
#[cfg(feature = "postgres")]
use sqlx::postgres::PgListener;
#[cfg(any(feature = "postgres", feature = "redb"))]
use std::sync::Arc;
use std::time::Instant;
#[cfg(any(feature = "postgres", feature = "redb"))]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(feature = "redb")]
use tokio::sync::Notify;
#[cfg(any(feature = "postgres", feature = "redb"))]
use tokio::time::Instant as TokioInstant;
use tracing::{debug, warn};

use crate::{JungleServer, Result, WireRx, WireTx};

#[cfg(feature = "postgres")]
const PG_JOURNEY_EVENTS_CHANNEL: &str = "jungle_journey_events";
#[cfg(any(feature = "postgres", feature = "redb"))]
const SUBSCRIPTION_LOG_INTERVAL: u64 = 256;
#[cfg(any(feature = "postgres", feature = "redb"))]
const SUBSCRIPTION_SLOW_FETCH_WARN_THRESHOLD: Duration = Duration::from_millis(100);
#[cfg(any(feature = "postgres", feature = "redb"))]
const WORKER_WAKE_WAIT_WARN_THRESHOLD: Duration = Duration::from_millis(150);
const SERVER_REQUEST_SLOW_WARN_MS: u128 = 100;
const SERVER_RESPONSE_SEND_SLOW_WARN_MS: u128 = 50;

#[derive(Clone)]
pub struct Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    store: Arc<dyn JungleStore>,
    #[cfg(feature = "redb")]
    journey_update_notify: Arc<Notify>,
}

impl Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn from_store(store: Arc<dyn JungleStore>) -> Self {
        Self {
            store,
            #[cfg(feature = "redb")]
            journey_update_notify: Arc::new(Notify::new()),
        }
    }
}

#[cfg(any(feature = "postgres", feature = "redb"))]
#[derive(Default, Clone)]
pub struct ServerBuilder {
    db: StoreBuilder,
}

#[cfg(any(feature = "postgres", feature = "redb"))]
impl ServerBuilder {
    #[cfg(feature = "postgres")]
    pub fn postgres(mut self, builder: jungle_persist::pg::PgStoreBuilder) -> Self {
        self.db = self.db.kind(Kind::Postgres(builder));
        self
    }

    #[cfg(feature = "postgres")]
    pub fn postgres_connection_string(self, value: impl Into<String>) -> Self {
        self.postgres(jungle_persist::pg::PgStore::builder().connection_string(value))
    }

    #[cfg(feature = "redb")]
    pub fn redb(mut self, builder: jungle_persist::redb::RedbStoreBuilder) -> Self {
        self.db = self.db.kind(Kind::Redb(builder));
        self
    }

    #[cfg(feature = "redb")]
    pub fn redb_path(self, value: impl Into<std::path::PathBuf>) -> Self {
        self.redb(jungle_persist::redb::RedbStore::builder().path(value))
    }

    #[cfg(feature = "redb")]
    pub fn memory(mut self) -> Self {
        self.db = self.db.kind(Kind::Memory);
        self
    }

    pub async fn build(self) -> jungle_persist::Result<Server> {
        #[cfg(all(feature = "postgres", feature = "redb"))]
        let has_configured_database = self.db.has_kind();
        #[cfg(all(feature = "postgres", feature = "redb"))]
        if !has_configured_database {
            tracing::info!(
                "both `postgres` and `redb` features are enabled; defaulting to postgres"
            );
        }
        let store = self.db.build().await?;
        store.migrate().await?;
        Ok(Server::from_store(Arc::from(store)))
    }
}

#[async_trait]
impl JungleServer for Server {
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request_started_at = Instant::now();
        let request = rx.next().await;
        let request_kind = request.as_ref().map(wire_in_kind).unwrap_or("NoRequest");
        debug!(
            has_request = request.is_some(),
            request_kind, "received request"
        );

        let response = match request {
            Some(WireIn::CreateJourney {
                namespace,
                animal_id,
                generation,
                seed,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    match self
                        .store
                        .create_journey(namespace, animal_id, generation, seed)
                        .await
                    {
                        Ok(journey_id) => {
                            #[cfg(feature = "redb")]
                            self.journey_update_notify.notify_waiters();
                            tx.send(Ok(WireOut::JourneyCreated(journey_id))).await?;
                        }
                        Err(err) => {
                            tx.send(Err(BackendError::Message(err.to_string()))).await?;
                        }
                    }
                    return Ok(());
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (namespace, animal_id, generation, seed);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "create_journey is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::JourneyHistory(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let history = self
                        .store
                        .journey_history(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::JourneyHistory(history)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "journey_history is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::JourneyStatus(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let status = self.store.journey_status(journey_id).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::JourneyStatus(status)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "journey_status is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::SubscribeJourneyUpdates {
                journey_id,
                after_sequence_id,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let mut cursor = after_sequence_id;
                    let mut fetch_iterations = 0_u64;
                    let mut emitted_updates = 0_u64;
                    #[cfg(feature = "postgres")]
                    let mut pg_listener = pg_listener_for_store(self.store.as_ref()).await;
                    loop {
                        fetch_iterations = fetch_iterations.saturating_add(1);
                        let fetch_started_at = TokioInstant::now();
                        let updates = self
                            .store
                            .journey_update_events_since(journey_id, cursor)
                            .await
                            .map_err(|err| {
                                crate::ServerError::Backend(BackendError::Message(err.to_string()))
                            })?;
                        let fetch_elapsed = fetch_started_at.elapsed();
                        if fetch_elapsed > SUBSCRIPTION_SLOW_FETCH_WARN_THRESHOLD {
                            warn!(
                                journey_id = %journey_id,
                                fetch_iterations,
                                fetch_elapsed_ms = fetch_elapsed.as_millis(),
                                update_batch_len = updates.len(),
                                cursor = cursor.unwrap_or(0),
                                "slow journey update fetch from persistence backend"
                            );
                        } else if fetch_iterations % SUBSCRIPTION_LOG_INTERVAL == 0 {
                            debug!(
                                journey_id = %journey_id,
                                fetch_iterations,
                                fetch_elapsed_ms = fetch_elapsed.as_millis(),
                                update_batch_len = updates.len(),
                                emitted_updates,
                                cursor = cursor.unwrap_or(0),
                                "journey update subscription fetch heartbeat"
                            );
                        }

                        if !updates.is_empty() {
                            let now_ms = now_unix_ms();
                            let mut min_event_age_ms = i64::MAX;
                            let mut max_event_age_ms = 0_i64;
                            for update in updates {
                                let event_age_ms = now_ms.saturating_sub(update.event_unix_ms);
                                min_event_age_ms = min_event_age_ms.min(event_age_ms);
                                max_event_age_ms = max_event_age_ms.max(event_age_ms);
                                cursor = Some(update.sequence_id);
                                tx.send(Ok(WireOut::JourneyUpdate(update))).await?;
                                emitted_updates = emitted_updates.saturating_add(1);
                            }
                            if emitted_updates % SUBSCRIPTION_LOG_INTERVAL == 0 {
                                debug!(
                                    journey_id = %journey_id,
                                    emitted_updates,
                                    update_batch_event_age_min_ms = min_event_age_ms,
                                    update_batch_event_age_max_ms = max_event_age_ms,
                                    cursor = cursor.unwrap_or(0),
                                    "journey update subscription emit heartbeat"
                                );
                            }
                            if max_event_age_ms > 1_000 {
                                warn!(
                                    journey_id = %journey_id,
                                    emitted_updates,
                                    update_batch_event_age_min_ms = min_event_age_ms,
                                    update_batch_event_age_max_ms = max_event_age_ms,
                                    cursor = cursor.unwrap_or(0),
                                    "journey update batch already stale at server emit"
                                );
                            }
                            #[cfg(feature = "redb")]
                            tokio::task::yield_now().await;
                            continue;
                        }

                        let status =
                            self.store.journey_status(journey_id).await.map_err(|err| {
                                crate::ServerError::Backend(BackendError::Message(err.to_string()))
                            })?;

                        if matches!(
                            status,
                            JourneyStatus::Stopped | JourneyStatus::Completed | JourneyStatus::Dead
                        ) {
                            tx.close().await?;
                            debug!("complete");
                            return Ok(());
                        }

                        #[cfg(feature = "postgres")]
                        if let Some(listener) = pg_listener.as_mut() {
                            match tokio::time::timeout(Duration::from_secs(5), listener.recv())
                                .await
                            {
                                Ok(Ok(_)) => {}
                                Ok(Err(err)) => {
                                    warn!("journey updates listener recv failed: {err}");
                                    pg_listener = None;
                                }
                                Err(_) => {}
                            }
                            continue;
                        }

                        #[cfg(feature = "redb")]
                        {
                            let notified = self.journey_update_notify.notified();
                            let _ = tokio::time::timeout(Duration::from_secs(5), notified).await;
                            continue;
                        }

                        #[cfg(not(feature = "redb"))]
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (journey_id, after_sequence_id);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "subscribe_journey_updates is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::AnimalAppearance(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let appearance =
                        self.store
                            .animal_appearance(journey_id)
                            .await
                            .map_err(|err| {
                                crate::ServerError::Backend(BackendError::Message(err.to_string()))
                            })?;
                    WireOut::AnimalAppearance(appearance)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "animal_appearance is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::PerturbAnimal { journey_id, data }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .enqueue_animal_perturbation(journey_id, data)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (journey_id, data);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "perturb_animal is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::ClaimPerturbable(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let claimed = self
                        .store
                        .claim_animal_perturbation(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::ClaimedPerturbable(claimed)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "claim_animal_perturbation is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::AckPerturbable {
                journey_id,
                perturbation_id,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .ack_animal_perturbation(journey_id, perturbation_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (journey_id, perturbation_id);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "ack_animal_perturbation is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::HeartbeatJourneyLease {
                journey_id,
                owner_id,
                lease_ttl_ms,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .heartbeat_journey_lease(journey_id, owner_id, lease_ttl_ms)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (journey_id, owner_id, lease_ttl_ms);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "heartbeat_journey_lease is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::PollOwnerWake { owner_id }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let wake = self.store.claim_owner_wake(owner_id).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::OwnerWake(wake)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = owner_id;
                    WireOut::OwnerWake(None)
                }
            }
            Some(WireIn::ScheduleSleep {
                journey_id,
                timer_id,
                wake_at_unix_ms,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    #[cfg(feature = "redb")]
                    self.journey_update_notify.notify_waiters();
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (journey_id, timer_id, wake_at_unix_ms);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "schedule_sleep_timer is unavailable without a persistence backend"
                            .to_string(),
                    )));
                }
            }
            Some(WireIn::JourneyComplete(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .journey_complete(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    #[cfg(feature = "redb")]
                    self.journey_update_notify.notify_waiters();
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    WireOut::Ack
                }
            }
            Some(WireIn::JourneyDead(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store
                        .journey_dead(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    #[cfg(feature = "redb")]
                    self.journey_update_notify.notify_waiters();
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = journey_id;
                    WireOut::Ack
                }
            }
            Some(WireIn::PollStep {
                namespace,
                supported_animals,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let claim_started_at = TokioInstant::now();
                    let claimed = self
                        .store
                        .claim_work(namespace, supported_animals)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    let claim_elapsed_ms = claim_started_at.elapsed().as_millis();
                    if claim_elapsed_ms > SUBSCRIPTION_SLOW_FETCH_WARN_THRESHOLD.as_millis() {
                        warn!(
                            claim_elapsed_ms,
                            had_work = claimed.is_some(),
                            "slow claim_work request"
                        );
                    }
                    match claimed {
                        Some(work) => WireOut::PendingStep(work),
                        None => WireOut::NoAvailableSteps,
                    }
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (namespace, supported_animals);
                    WireOut::NoAvailableSteps
                }
            }
            Some(WireIn::WaitForWorkerWake {
                owner_id,
                namespace,
                supported_animals,
                timeout_ms,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let timeout = Duration::from_millis(
                        timeout_ms
                            .max(1)
                            .min(Duration::from_secs(30).as_millis() as u64),
                    );
                    let _ = (owner_id, namespace, supported_animals);
                    let wait_started_at = TokioInstant::now();
                    wait_for_worker_wake(self, timeout).await?;
                    let wait_elapsed = wait_started_at.elapsed();
                    if wait_elapsed > WORKER_WAKE_WAIT_WARN_THRESHOLD {
                        warn!(
                            owner_id = %owner_id,
                            timeout_ms = timeout.as_millis(),
                            wait_elapsed_ms = wait_elapsed.as_millis(),
                            "slow WaitForWorkerWake request handling"
                        );
                    }
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (owner_id, namespace, supported_animals, timeout_ms);
                    WireOut::Ack
                }
            }
            Some(WireIn::PollTimers) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let polled = self.store.poll_timers().await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    #[cfg(feature = "redb")]
                    if polled.is_some() {
                        self.journey_update_notify.notify_waiters();
                    }
                    #[cfg(not(feature = "redb"))]
                    let _ = polled;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    WireOut::Ack
                }
            }
            Some(WireIn::HistoryEvent {
                event: history,
                event_unix_ms,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let history_event_age_ms = now_unix_ms().saturating_sub(event_unix_ms);
                    let journey_id = match &history {
                        jungle_types::RunnerOut::EffectInput { uuid, .. }
                        | jungle_types::RunnerOut::EffectSuccessOutput { uuid, .. }
                        | jungle_types::RunnerOut::EffectFailureOutput { uuid, .. }
                        | jungle_types::RunnerOut::Appearance { uuid, .. }
                        | jungle_types::RunnerOut::SleepScheduled { uuid, .. }
                        | jungle_types::RunnerOut::SleepFired { uuid, .. } => *uuid,
                    };
                    self.store
                        .journey_alive_if_created(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    match history {
                        jungle_types::RunnerOut::Appearance { data, uuid } => {
                            let appearance_started_at = TokioInstant::now();
                            self.store
                                .upsert_animal_appearance(uuid, data)
                                .await
                                .map_err(|err| {
                                    crate::ServerError::Backend(BackendError::Message(
                                        err.to_string(),
                                    ))
                                })?;
                            let appearance_elapsed_ms = appearance_started_at.elapsed().as_millis();
                            if appearance_elapsed_ms
                                > SUBSCRIPTION_SLOW_FETCH_WARN_THRESHOLD.as_millis()
                            {
                                warn!(
                                    journey_id = %journey_id,
                                    history_event_age_ms,
                                    appearance_elapsed_ms,
                                    "slow appearance upsert from HistoryEvent"
                                );
                            }
                        }
                        event => {
                            let append_started_at = TokioInstant::now();
                            self.store
                                .append_history(event, event_unix_ms)
                                .await
                                .map_err(|err| {
                                    crate::ServerError::Backend(BackendError::Message(
                                        err.to_string(),
                                    ))
                                })?;
                            let append_elapsed_ms = append_started_at.elapsed().as_millis();
                            if append_elapsed_ms
                                > SUBSCRIPTION_SLOW_FETCH_WARN_THRESHOLD.as_millis()
                            {
                                warn!(
                                    journey_id = %journey_id,
                                    history_event_age_ms,
                                    append_elapsed_ms,
                                    "slow append_history from HistoryEvent"
                                );
                            } else if history_event_age_ms > 1_000 {
                                warn!(
                                    journey_id = %journey_id,
                                    history_event_age_ms,
                                    append_elapsed_ms,
                                    "HistoryEvent already stale before append_history"
                                );
                            }
                            #[cfg(feature = "redb")]
                            self.journey_update_notify.notify_waiters();
                        }
                    }
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (history, event_unix_ms);
                    WireOut::Ack
                }
            }
            None => WireOut::NoAvailableSteps,
        };
        let response_send_started_at = Instant::now();
        tx.send(Ok(response)).await?;
        let response_send_elapsed_ms = response_send_started_at.elapsed().as_millis();
        if response_send_elapsed_ms > SERVER_RESPONSE_SEND_SLOW_WARN_MS {
            warn!(
                request_kind,
                response_send_elapsed_ms, "slow server response send"
            );
        }
        tx.close().await?;
        let request_elapsed_ms = request_started_at.elapsed().as_millis();
        if request_elapsed_ms > SERVER_REQUEST_SLOW_WARN_MS {
            warn!(
                request_kind,
                request_elapsed_ms, "slow server request handling"
            );
        } else {
            debug!(request_kind, request_elapsed_ms, "complete");
        }
        Ok(())
    }
}

#[cfg(any(feature = "postgres", feature = "redb"))]
async fn wait_for_worker_wake(server: &Server, timeout: Duration) -> Result<()> {
    let deadline = TokioInstant::now() + timeout;
    let mut wait_iterations = 0_u64;

    #[cfg(feature = "postgres")]
    let mut pg_listener = pg_listener_for_store(server.store.as_ref()).await;

    loop {
        wait_iterations = wait_iterations.saturating_add(1);
        if progress_due_timers(server).await? {
            return Ok(());
        }

        let remaining = deadline.saturating_duration_since(TokioInstant::now());
        if remaining.is_zero() {
            return Ok(());
        }

        let wait_for = wait_duration_until_next_timer(server, remaining).await?;
        if wait_iterations % SUBSCRIPTION_LOG_INTERVAL == 0 {
            debug!(
                wait_iterations,
                timeout_ms = timeout.as_millis(),
                remaining_ms = remaining.as_millis(),
                wait_for_ms = wait_for.as_millis(),
                "wait_for_worker_wake loop heartbeat"
            );
        }

        #[cfg(feature = "postgres")]
        let mut waited_on_pg_listener = false;

        #[cfg(feature = "postgres")]
        if let Some(listener) = pg_listener.as_mut() {
            waited_on_pg_listener = true;
            match tokio::time::timeout(wait_for, listener.recv()).await {
                Ok(Ok(_)) => return Ok(()),
                Ok(Err(err)) => {
                    warn!("worker wake listener recv failed: {err}");
                    pg_listener = None;
                }
                Err(_) => {}
            }
        }

        #[cfg(feature = "postgres")]
        if waited_on_pg_listener {
            continue;
        }

        #[cfg(feature = "redb")]
        {
            let notified = server.journey_update_notify.notified();
            if tokio::time::timeout(wait_for, notified).await.is_ok() {
                return Ok(());
            }
        }

        #[cfg(not(any(feature = "postgres", feature = "redb")))]
        {
            tokio::time::sleep(wait_for).await;
        }
    }
}

#[cfg(any(feature = "postgres", feature = "redb"))]
async fn progress_due_timers(server: &Server) -> Result<bool> {
    let mut advanced = false;
    loop {
        let Some(next_due_at_unix_ms) =
            server.store.next_timer_due_at().await.map_err(|err| {
                crate::ServerError::Backend(BackendError::Message(err.to_string()))
            })?
        else {
            break;
        };

        if next_due_at_unix_ms > now_unix_ms() {
            break;
        }

        let polled =
            server.store.poll_timers().await.map_err(|err| {
                crate::ServerError::Backend(BackendError::Message(err.to_string()))
            })?;
        if polled.is_none() {
            break;
        }

        advanced = true;

        #[cfg(feature = "redb")]
        server.journey_update_notify.notify_waiters();
    }

    Ok(advanced)
}

#[cfg(any(feature = "postgres", feature = "redb"))]
async fn wait_duration_until_next_timer(server: &Server, max_wait: Duration) -> Result<Duration> {
    let Some(next_due_at_unix_ms) = server
        .store
        .next_timer_due_at()
        .await
        .map_err(|err| crate::ServerError::Backend(BackendError::Message(err.to_string())))?
    else {
        return Ok(max_wait);
    };

    let now_unix_ms = now_unix_ms();
    if next_due_at_unix_ms <= now_unix_ms {
        return Ok(Duration::from_millis(1));
    }

    let due_wait_ms =
        u64::try_from(next_due_at_unix_ms.saturating_sub(now_unix_ms)).unwrap_or(u64::MAX);
    Ok(std::cmp::min(max_wait, Duration::from_millis(due_wait_ms)))
}

#[cfg(any(feature = "postgres", feature = "redb"))]
fn now_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

fn wire_in_kind(input: &WireIn) -> &'static str {
    match input {
        WireIn::CreateJourney { .. } => "CreateJourney",
        WireIn::JourneyHistory(..) => "JourneyHistory",
        WireIn::JourneyStatus(..) => "JourneyStatus",
        WireIn::SubscribeJourneyUpdates { .. } => "SubscribeJourneyUpdates",
        WireIn::AnimalAppearance(..) => "AnimalAppearance",
        WireIn::PerturbAnimal { .. } => "PerturbAnimal",
        WireIn::ClaimPerturbable(..) => "ClaimPerturbable",
        WireIn::AckPerturbable { .. } => "AckPerturbable",
        WireIn::HeartbeatJourneyLease { .. } => "HeartbeatJourneyLease",
        WireIn::PollOwnerWake { .. } => "PollOwnerWake",
        WireIn::ScheduleSleep { .. } => "ScheduleSleep",
        WireIn::JourneyComplete(..) => "JourneyComplete",
        WireIn::JourneyDead(..) => "JourneyDead",
        WireIn::PollStep { .. } => "PollStep",
        WireIn::WaitForWorkerWake { .. } => "WaitForWorkerWake",
        WireIn::PollTimers => "PollTimers",
        WireIn::HistoryEvent { .. } => "HistoryEvent",
    }
}

#[cfg(feature = "postgres")]
async fn pg_listener_for_store(store: &dyn JungleStore) -> Option<PgListener> {
    let pool = store.postgres_pool()?;
    let mut listener = match PgListener::connect_with(&pool).await {
        Ok(listener) => listener,
        Err(err) => {
            warn!("failed to connect postgres listener: {err}");
            return None;
        }
    };

    if let Err(err) = listener.listen(PG_JOURNEY_EVENTS_CHANNEL).await {
        warn!("failed to listen on postgres channel {PG_JOURNEY_EVENTS_CHANNEL}: {err}");
        return None;
    }

    Some(listener)
}
