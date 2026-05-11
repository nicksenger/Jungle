use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
#[cfg(any(feature = "postgres", feature = "redb"))]
use jungle_persist::JungleStore;
use jungle_types::{BackendError, WireIn, WireOut};
#[cfg(any(feature = "postgres", feature = "redb"))]
use std::sync::Arc;
use tracing::info;

use crate::{JungleServer, Result, WireRx, WireTx};

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
    #[cfg(feature = "postgres")]
    postgres: Option<jungle_persist::pg::PgStoreBuilder>,
    #[cfg(feature = "redb")]
    redb: Option<jungle_persist::redb::RedbStoreBuilder>,
}

#[cfg(any(feature = "postgres", feature = "redb"))]
impl ServerBuilder {
    #[cfg(feature = "postgres")]
    pub fn postgres(mut self, builder: jungle_persist::pg::PgStoreBuilder) -> Self {
        self.postgres = Some(builder);
        self
    }

    #[cfg(feature = "postgres")]
    pub fn postgres_connection_string(self, value: impl Into<String>) -> Self {
        self.postgres(jungle_persist::pg::PgStore::builder().connection_string(value))
    }

    #[cfg(feature = "redb")]
    pub fn redb(mut self, builder: jungle_persist::redb::RedbStoreBuilder) -> Self {
        self.redb = Some(builder);
        self
    }

    #[cfg(feature = "redb")]
    pub fn redb_path(self, value: impl Into<std::path::PathBuf>) -> Self {
        self.redb(jungle_persist::redb::RedbStore::builder().path(value))
    }

    pub async fn build(self) -> jungle_persist::Result<Server> {
        #[cfg(all(feature = "postgres", feature = "redb"))]
        {
            if let Some(builder) = self.postgres {
                let store = builder.build().await?;
                store.migrate().await?;
                return Ok(Server::from_store(Arc::new(store)));
            }

            if let Some(builder) = self.redb {
                let store = builder.build()?;
                store.migrate().await?;
                return Ok(Server::from_store(Arc::new(store)));
            }

            info!("both `postgres` and `redb` features are enabled; defaulting to postgres");
            let store = jungle_persist::pg::PgStore::builder().build().await?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }

        #[cfg(all(feature = "postgres", not(feature = "redb")))]
        {
            let store = self
                .postgres
                .unwrap_or_else(jungle_persist::pg::PgStore::builder)
                .build()
                .await?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }

        #[cfg(all(feature = "redb", not(feature = "postgres")))]
        {
            let store = self
                .redb
                .unwrap_or_else(jungle_persist::redb::RedbStore::builder)
                .build()?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }
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
                client_observed_generation,
                seed,
            }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    match self
                        .store
                        .create_journey(namespace, animal_id, client_observed_generation, seed)
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
                    let _ = (namespace, animal_id, client_observed_generation, seed);
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
                        jungle_types::RunnerOut::ActionInput { uuid, .. }
                        | jungle_types::RunnerOut::ActionSuccessOutput { uuid, .. }
                        | jungle_types::RunnerOut::ActionFailureOutput { uuid, .. }
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
