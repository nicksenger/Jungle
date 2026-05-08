use jungle_types::{RunnerOut, Work};
use thiserror::Error;
use uuid::Uuid;

pub type Error = PersistenceError;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("{0}")]
    Message(String),
}

/// Storage backend contract for persistence implementations.
pub trait Store {
    fn claim_work(&self) -> Result<Option<Work>>;
    fn append_history(&self, history: RunnerOut) -> Result<()>;
    fn poll_timers(&self) -> Result<Option<()>>;
    fn details(&self, flow_id: Uuid) -> Result<()>;
}
