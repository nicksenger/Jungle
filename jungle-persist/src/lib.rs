use async_trait::async_trait;
use dyn_clone::DynClone;
use jungle_types::{RunnerOut, Work};
use thiserror::Error;
use uuid::Uuid;

pub mod models;
pub mod mock;
pub mod pg;
pub mod redb;

pub type Error = PersistenceError;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("{0}")]
    Message(String),
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
