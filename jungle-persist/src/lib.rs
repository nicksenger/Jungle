use async_trait::async_trait;
use dyn_clone::DynClone;
use jungle_types::{
    ClaimedAnimalPerturbation, JourneyStatus, OwnerWake, RunnerOut, SupportedAnimal, Work,
};
use thiserror::Error;
use uuid::Uuid;

pub mod mock;
pub mod models;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "redb")]
pub mod redb;

pub type Error = PersistenceError;
pub type Result<T> = std::result::Result<T, Error>;

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
    #[cfg(feature = "redb")]
    #[error("redb database path is required")]
    MissingRedbPath,
    #[cfg(feature = "redb")]
    #[error("redb open/create failed: {0}")]
    RedbOpen(#[source] ::redb::DatabaseError),
}

/// Storage backend contract for persistence implementations.
#[async_trait]
pub trait JungleStore: DynClone + Send + Sync {
    async fn migrate(&self) -> Result<()>;
    async fn create_journey(
        &self,
        namespace: String,
        animal_id: u32,
        client_observed_generation: Option<u32>,
        seed: Vec<u8>,
    ) -> Result<Uuid>;
    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>>;
    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus>;
    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>>;
    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()>;
    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()>;
    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>>;
    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()>;
    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<()>;
    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>>;
    async fn journey_complete(&self, journey_id: Uuid) -> Result<()>;
    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()>;
    async fn claim_work(
        &self,
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>>;
    async fn append_history(&self, history: RunnerOut) -> Result<()>;
    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()>;
    async fn poll_timers(&self) -> Result<Option<()>>;
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

    if !cfg!(any(feature = "postgres", feature = "redb")) {
        panic!("no persistence backend compiled; enable `postgres` or `redb` feature");
    }
}
