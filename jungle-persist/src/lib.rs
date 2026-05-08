use async_trait::async_trait;
use dyn_clone::DynClone;
use jungle_types::{RunnerOut, Work};
use thiserror::Error;
use uuid::Uuid;

pub mod models;
pub mod mock;
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
    async fn claim_work(&self) -> Result<Option<Work>>;
    async fn append_history(&self, history: RunnerOut) -> Result<()>;
    async fn poll_timers(&self) -> Result<Option<()>>;
    async fn details(&self, flow_id: Uuid) -> Result<()>;
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
