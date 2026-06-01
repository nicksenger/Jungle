//! Client contracts for the Jungle workspace.

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use jungle_types::{
    Animal, AnimalIdValue, ClaimedPerturbable, ExecutorError, JourneyRecord, JourneyStatus,
    OwnerWake, RunnerOut, SupportedAnimal, Work,
};
use std::time::Duration;
use typosaurus::num::Unsigned;
use uuid::Uuid;

pub mod client;
pub mod mock;

pub use client::{
    Client, ClientBuilder, ClientError, ClientResult, JourneyUpdateSubscription, StepUpdate,
};
pub use mock::{MockClient, MockClientBuilder};

pub struct JourneyHandle {
    pub journey_id: Uuid,
    client: Box<dyn JungleClient>,
}

impl core::fmt::Debug for JourneyHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("JourneyHandle")
            .field("journey_id", &self.journey_id)
            .finish_non_exhaustive()
    }
}

impl JourneyHandle {
    pub fn new(journey_id: Uuid, client: Box<dyn JungleClient>) -> Self {
        Self { journey_id, client }
    }

    pub async fn journey_history(&self) -> Result<Vec<RunnerOut>, ExecutorError> {
        self.client.journey_history(self.journey_id).await
    }

    pub async fn subscribe_step_updates(
        &self,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        self.client
            .subscribe_step_updates(self.journey_id, after_sequence_id)
            .await
    }

    pub async fn journey_details(&self) -> Result<JourneyStatus, ExecutorError> {
        self.client.journey_details(self.journey_id).await
    }

    pub async fn cancel(&self) -> Result<(), ExecutorError> {
        self.client.dead_journey(self.journey_id).await
    }

    pub async fn complete(&self) -> Result<(), ExecutorError> {
        self.client.complete_journey(self.journey_id).await
    }

    pub async fn await_completion(&self) -> Result<JourneyStatus, ExecutorError> {
        match self.journey_details().await? {
            JourneyStatus::Completed | JourneyStatus::Dead | JourneyStatus::Stopped => {
                return Ok(self.journey_details().await?);
            }
            JourneyStatus::Created | JourneyStatus::Alive => {}
        }

        let mut subscription = self.subscribe_step_updates(None).await?;
        while let Some(update) = subscription.next().await {
            update?;
        }

        self.journey_details().await
    }
}

#[async_trait]
pub trait JungleClient: DynClone + Send + Sync {
    async fn spawn<A>(&self, seed: &A::Seed) -> Result<JourneyHandle, ExecutorError>
    where
        Self: Sized,
        A: Animal,
        A::Id: AnimalIdValue,
        A::Generation: Unsigned,
        A::Seed: Sync;
    async fn journey_history(&self, id: Uuid) -> Result<Vec<RunnerOut>, ExecutorError>;
    async fn list_journeys(&self, namespace: String) -> Result<Vec<JourneyRecord>, ExecutorError>;
    async fn subscribe_step_updates(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError>;
    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError>;
    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError>;
    async fn animal_appearance_update(&self, id: Uuid, data: Vec<u8>) -> Result<(), ExecutorError>;
    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError>;
    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>, ExecutorError>;
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
    async fn dead_journey(&self, id: Uuid) -> Result<(), ExecutorError>;
    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError>;
    async fn poll_work(
        &self,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>, ExecutorError>;
    async fn wait_for_worker_wake(
        &self,
        owner_id: Uuid,
        supported_animals: Vec<SupportedAnimal>,
        timeout: Duration,
    ) -> Result<(), ExecutorError>;
    async fn effect_input(
        &self,
        id: Uuid,
        node_id: u32,
        input: Vec<u8>,
    ) -> Result<(), ExecutorError>;
    async fn effect_success_output(
        &self,
        id: Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError>;
    async fn effect_failure_output(
        &self,
        id: Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError>;
}

dyn_clone::clone_trait_object!(JungleClient);

pub enum RunnerChannelMessage {
    History(RunnerOut),
    ClaimPerturbable {
        journey_id: Uuid,
    },
    AckPerturbable {
        journey_id: Uuid,
        perturbation_id: u64,
    },
}

pub enum RunnerChannelResponse {
    Ack,
    ClaimedPerturbation(Option<ClaimedPerturbable>),
}

pub type RunnerChannelTx = mpsc::Sender<(
    RunnerChannelMessage,
    oneshot::Sender<Result<RunnerChannelResponse, ExecutorError>>,
)>;
pub type RunnerChannelRx = mpsc::Receiver<(
    RunnerChannelMessage,
    oneshot::Sender<Result<RunnerChannelResponse, ExecutorError>>,
)>;
