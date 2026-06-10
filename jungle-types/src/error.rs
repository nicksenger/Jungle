use serde::{Deserialize, Serialize};
use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum Failure {
    #[error("{0}")]
    Message(String),
}

impl From<String> for Failure {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for Failure {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

impl<E> From<Box<E>> for Failure
where
    E: StdError + Send + Sync + 'static + ?Sized,
{
    fn from(value: Box<E>) -> Self {
        Self::Message(value.to_string())
    }
}
