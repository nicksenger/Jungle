use async_trait::async_trait;
use dyn_clone::DynClone;
use jungle_types::{
    ClaimedPerturbable, JourneyRecord, JourneyReplayPage, JourneyStatus, JourneyUpdateEvent,
    OwnerWake, RunnerOut, SupportedAnimal, Work,
};
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "fjall")]
pub mod fjall;
pub mod mock;
pub mod models;
#[cfg(feature = "postgres")]
pub mod pg;

pub type Error = PersistenceError;
pub type Result<T> = std::result::Result<T, Error>;
pub const DEFAULT_CLAIMED_WORK_TTL_MS: i64 = 30_000;

#[derive(Debug, Clone)]
pub enum Kind {
    #[cfg(feature = "postgres")]
    Postgres(pg::PgStoreBuilder),
    #[cfg(feature = "fjall")]
    Fjall(fjall::FjallStoreBuilder),
    #[cfg(feature = "fjall")]
    Memory,
}

#[derive(Debug, Clone)]
pub struct StoreBuilder {
    kind: Option<Kind>,
    claimed_work_ttl_ms: i64,
}

impl Default for StoreBuilder {
    fn default() -> Self {
        Self {
            kind: None,
            claimed_work_ttl_ms: DEFAULT_CLAIMED_WORK_TTL_MS,
        }
    }
}

impl StoreBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind(mut self, value: Kind) -> Self {
        self.kind = Some(value);
        self
    }

    pub fn has_kind(&self) -> bool {
        self.kind.is_some()
    }

    pub fn claimed_work_ttl_ms(mut self, value: i64) -> Self {
        self.claimed_work_ttl_ms = value.max(0);
        self
    }

    #[cfg(feature = "postgres")]
    pub fn postgres(self, builder: pg::PgStoreBuilder) -> Self {
        self.kind(Kind::Postgres(builder))
    }

    #[cfg(feature = "postgres")]
    pub fn postgres_connection_string(self, value: impl Into<String>) -> Self {
        self.postgres(pg::PgStore::builder().connection_string(value))
    }

    #[cfg(feature = "fjall")]
    pub fn fjall(self, builder: fjall::FjallStoreBuilder) -> Self {
        self.kind(Kind::Fjall(builder))
    }

    #[cfg(feature = "fjall")]
    pub fn fjall_path(self, value: impl Into<std::path::PathBuf>) -> Self {
        self.fjall(fjall::FjallStore::builder().path(value))
    }

    #[cfg(feature = "fjall")]
    /// Uses an auto-cleaned temporary Fjall directory rather than a RAM-only store.
    pub fn memory(self) -> Self {
        self.kind(Kind::Memory)
    }

    pub async fn build(self) -> Result<Box<dyn JungleStore>> {
        #[cfg(all(feature = "postgres", feature = "fjall"))]
        {
            match self.kind {
                Some(Kind::Postgres(builder)) => {
                    let store = builder
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()
                        .await?;
                    return Ok(Box::new(store));
                }
                Some(Kind::Fjall(builder)) => {
                    let store = builder
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()?;
                    return Ok(Box::new(store));
                }
                Some(Kind::Memory) => {
                    let store = fjall::FjallStore::in_memory_with_claimed_work_ttl_ms(
                        self.claimed_work_ttl_ms,
                    )?;
                    return Ok(Box::new(store));
                }
                None => {
                    let store = pg::PgStore::builder()
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()
                        .await?;
                    return Ok(Box::new(store));
                }
            }
        }

        #[cfg(all(feature = "postgres", not(feature = "fjall")))]
        {
            match self.kind {
                Some(Kind::Postgres(builder)) => {
                    let store = builder
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()
                        .await?;
                    return Ok(Box::new(store));
                }
                None => {
                    let store = pg::PgStore::builder()
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()
                        .await?;
                    return Ok(Box::new(store));
                }
            }
        }

        #[cfg(all(feature = "fjall", not(feature = "postgres")))]
        {
            match self.kind {
                Some(Kind::Fjall(builder)) => {
                    let store = builder
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()?;
                    return Ok(Box::new(store));
                }
                Some(Kind::Memory) => {
                    let store = fjall::FjallStore::in_memory_with_claimed_work_ttl_ms(
                        self.claimed_work_ttl_ms,
                    )?;
                    return Ok(Box::new(store));
                }
                None => {
                    let store = fjall::FjallStore::builder()
                        .claimed_work_ttl_ms(self.claimed_work_ttl_ms)
                        .build()?;
                    return Ok(Box::new(store));
                }
            }
        }

        #[allow(unreachable_code)]
        Err(PersistenceError::Message(
            "no persistence backend compiled; enable `postgres` or `fjall` feature".to_string(),
        ))
    }
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("{0}")]
    Message(String),
    #[cfg(feature = "postgres")]
    #[error("postgres connection string is required")]
    MissingPostgresConnectionString,
    #[cfg(feature = "postgres")]
    #[error("postgres connection failed: {0}")]
    PostgresConnect(#[source] sqlx::Error),
    #[cfg(feature = "postgres")]
    #[error("postgres query failed: {0}")]
    PostgresQuery(#[source] sqlx::Error),
    #[cfg(feature = "fjall")]
    #[error("fjall database path is required")]
    MissingFjallPath,
    #[cfg(feature = "fjall")]
    #[error("fjall open/create failed: {0}")]
    FjallOpen(#[source] ::fjall::Error),
}

/// Storage backend contract for persistence implementations.
#[async_trait]
pub trait JungleStore: DynClone + Send + Sync {
    async fn migrate(&self) -> Result<()>;
    async fn create_journey(
        &self,
        namespace: String,
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
    ) -> Result<Uuid>;
    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>>;
    async fn journey_replay_page(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
        snapshot_end_sequence_id: Option<u64>,
        limit: u32,
    ) -> Result<JourneyReplayPage>;
    async fn journey_update_events_since(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<Vec<JourneyUpdateEvent>>;
    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus>;
    async fn list_journeys(&self, namespace: String) -> Result<Vec<JourneyRecord>>;
    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>>;
    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()>;
    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()>;
    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>>;
    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()>;
    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<()>;
    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>>;
    async fn journey_complete(&self, journey_id: Uuid) -> Result<()>;
    async fn journey_dead(&self, journey_id: Uuid) -> Result<()>;
    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()>;
    async fn claim_work(
        &self,
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>>;
    async fn append_history(&self, history: RunnerOut, event_unix_ms: i64) -> Result<()>;
    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()>;
    async fn next_timer_due_at(&self) -> Result<Option<i64>>;
    async fn poll_timers(&self) -> Result<Option<()>>;

    #[cfg(feature = "postgres")]
    fn postgres_pool(&self) -> Option<sqlx::PgPool> {
        None
    }
}

dyn_clone::clone_trait_object!(JungleStore);

/// Panics when no concrete storage backend is compiled in.
///
/// Callers using `MockStore` should avoid invoking this.
pub fn ensure_store_backend_available() {
    ensure_store_backend_available_or_mock(false);
}

/// Panics when no concrete storage backend is compiled in unless `using_mock` is true.
pub fn ensure_store_backend_available_or_mock(using_mock: bool) {
    if using_mock {
        return;
    }

    if !cfg!(any(feature = "postgres", feature = "fjall")) {
        panic!("no persistence backend compiled; enable `postgres` or `fjall` feature");
    }
}
