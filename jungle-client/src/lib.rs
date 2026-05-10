//! Client contracts for the Jungle workspace.

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::channel::{mpsc, oneshot};
use jungle_types::{
    ClaimedAnimalPerturbation, ExecutorError, JourneyStatus, OwnerWake, RunnerOut, RunnerStep,
};
use uuid::Uuid;

pub mod client;
pub mod mock;

pub use client::{Client, ClientBuilder, ClientError, ClientResult};
pub use mock::{MockClient, MockClientBuilder};

#[async_trait]
pub trait JungleClient: DynClone + Send + Sync {
    async fn start_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid, ExecutorError>;
    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError>;
    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError>;
    async fn animal_appearance_update(&self, id: Uuid, data: Vec<u8>) -> Result<(), ExecutorError>;
    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError>;
    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>, ExecutorError>;
    async fn ack_animal_perturbation(
        &self,
        id: Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError>;
    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<(), ExecutorError>;
    async fn poll_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>, ExecutorError>;
    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<(), ExecutorError>;
    async fn complete_journey(&self, id: Uuid) -> Result<(), ExecutorError>;
    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError>;
    async fn poll_work(&self) -> Result<Option<RunnerStep>, ExecutorError>;
    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError>;
}

dyn_clone::clone_trait_object!(JungleClient);

pub enum RunnerChannelMessage {
    History(RunnerOut),
    ClaimAnimalPerturbation {
        journey_id: Uuid,
    },
    AckAnimalPerturbation {
        journey_id: Uuid,
        perturbation_id: u64,
    },
}

pub enum RunnerChannelResponse {
    Ack,
    ClaimedPerturbation(Option<ClaimedAnimalPerturbation>),
}

pub type RunnerChannelTx = mpsc::Sender<(
    RunnerChannelMessage,
    oneshot::Sender<Result<RunnerChannelResponse, ExecutorError>>,
)>;
pub type RunnerChannelRx = mpsc::Receiver<(
    RunnerChannelMessage,
    oneshot::Sender<Result<RunnerChannelResponse, ExecutorError>>,
)>;
