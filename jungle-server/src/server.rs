use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
#[cfg(any(feature = "postgres", feature = "redb"))]
use jungle_persist::{JungleStore, Kind, StoreBuilder};
use jungle_types::{BackendError, JourneyStatus, WireIn, WireOut};
#[cfg(feature = "postgres")]
use sqlx::postgres::PgListener;
#[cfg(any(feature = "postgres", feature = "redb"))]
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
#[cfg(feature = "postgres")]
use tracing::warn;

use crate::{JungleServer, Result, WireRx, WireTx};

#[cfg(feature = "postgres")]
const PG_JOURNEY_EVENTS_CHANNEL: &str = "jungle_journey_events";

#[derive(Clone)]
pub struct Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    store: Arc<dyn JungleStore>,
}

impl Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn from_store(store: Arc<dyn JungleStore>) -> Self {
        Self { store }
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
            info!("both `postgres` and `redb` features are enabled; defaulting to postgres");
        }
        let store = self.db.build().await?;
        store.migrate().await?;
        Ok(Server::from_store(Arc::from(store)))
    }
}

#[async_trait]
impl JungleServer for Server {
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request = rx.next().await;
        info!(has_request = request.is_some(), "received request");

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
                    #[cfg(feature = "postgres")]
                    let mut pg_listener = pg_listener_for_store(self.store.as_ref()).await;
                    loop {
                        let updates = self
                            .store
                            .journey_update_events_since(journey_id, cursor)
                            .await
                            .map_err(|err| {
                                crate::ServerError::Backend(BackendError::Message(err.to_string()))
                            })?;

                        if !updates.is_empty() {
                            for update in updates {
                                cursor = Some(update.sequence_id);
                                tx.send(Ok(WireOut::JourneyUpdate(update))).await?;
                            }
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
                            info!("complete");
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

                        sleep(Duration::from_millis(100)).await;
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
            Some(WireIn::ClaimAnimalPerturbation(journey_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let claimed = self
                        .store
                        .claim_animal_perturbation(journey_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    WireOut::ClaimedAnimalPerturbation(claimed)
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
            Some(WireIn::AckAnimalPerturbation {
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
                    match self
                        .store
                        .claim_work(namespace, supported_animals)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })? {
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
            Some(WireIn::PollTimers) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let _ = self.store.poll_timers().await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    WireOut::Ack
                }
            }
            Some(WireIn::HistoryEvent(history)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
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
                            self.store
                                .upsert_animal_appearance(uuid, data)
                                .await
                                .map_err(|err| {
                                    crate::ServerError::Backend(BackendError::Message(
                                        err.to_string(),
                                    ))
                                })?;
                        }
                        event => {
                            self.store.append_history(event).await.map_err(|err| {
                                crate::ServerError::Backend(BackendError::Message(err.to_string()))
                            })?;
                        }
                    }
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = history;
                    WireOut::Ack
                }
            }
            None => WireOut::NoAvailableSteps,
        };
        tx.send(Ok(response)).await?;
        tx.close().await?;
        info!("complete");
        Ok(())
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
