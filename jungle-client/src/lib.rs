//! Client contracts for the Jungle workspace.

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::channel::{mpsc, oneshot};
use jungle_types::{ExecutorError, FlowStatus, RunnerOut, Work};
use uuid::Uuid;

pub mod client;
pub mod mock;

pub use client::{Client, ClientBuilder, ClientError, ClientResult};
pub use mock::{MockClient, MockClientBuilder};

#[async_trait]
pub trait JungleClient: DynClone + Send + Sync {
    async fn create_flow(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid, ExecutorError>;
    async fn flow_status(&self, id: Uuid) -> Result<FlowStatus, ExecutorError>;
    async fn flow_complete(&self, id: Uuid) -> Result<(), ExecutorError>;
    async fn poll_work(&self) -> Result<Option<Work>, ExecutorError>;
    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError>;
}

dyn_clone::clone_trait_object!(JungleClient);

pub type RunnerChannelTx = mpsc::Sender<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;
pub type RunnerChannelRx = mpsc::Receiver<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;
