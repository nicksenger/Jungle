use std::sync::Arc;

use async_trait::async_trait;
use jungle_types::{ClaimedAnimalPerturbation, JourneyStatus, OwnerWake, RunnerOut, RunnerStep};
use uuid::Uuid;

use crate::{JungleStore, Result};

type ClaimWorkHandler = Arc<dyn Fn(String) -> Result<Option<RunnerStep>> + Send + Sync + 'static>;
type CreateFlowHandler = Arc<dyn Fn(String, u32, Vec<u8>) -> Result<Uuid> + Send + Sync + 'static>;
type JourneyHistoryHandler = Arc<dyn Fn(Uuid) -> Result<Vec<RunnerOut>> + Send + Sync + 'static>;
type FlowStatusHandler = Arc<dyn Fn(Uuid) -> Result<JourneyStatus> + Send + Sync + 'static>;
type FlowAppearanceHandler = Arc<dyn Fn(Uuid) -> Result<Option<Vec<u8>>> + Send + Sync + 'static>;
type UpsertFlowAppearanceHandler = Arc<dyn Fn(Uuid, Vec<u8>) -> Result<()> + Send + Sync + 'static>;
type EnqueuePerturbationHandler = Arc<dyn Fn(Uuid, Vec<u8>) -> Result<()> + Send + Sync + 'static>;
type ClaimPerturbationHandler =
    Arc<dyn Fn(Uuid) -> Result<Option<ClaimedAnimalPerturbation>> + Send + Sync + 'static>;
type AckPerturbationHandler = Arc<dyn Fn(Uuid, u64) -> Result<()> + Send + Sync + 'static>;
type HeartbeatJourneyLeaseHandler =
    Arc<dyn Fn(Uuid, Uuid, i64) -> Result<()> + Send + Sync + 'static>;
type ClaimOwnerWakeHandler = Arc<dyn Fn(Uuid) -> Result<Option<OwnerWake>> + Send + Sync + 'static>;
type FlowCompleteHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;
type FlowAliveIfCreatedHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;
type AppendHistoryHandler = Arc<dyn Fn(RunnerOut) -> Result<()> + Send + Sync + 'static>;
type ScheduleSleepTimerHandler = Arc<dyn Fn(Uuid, Uuid, i64) -> Result<()> + Send + Sync + 'static>;
type PollTimersHandler = Arc<dyn Fn() -> Result<Option<()>> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockStore {
    on_create_flow: CreateFlowHandler,
    on_journey_history: JourneyHistoryHandler,
    on_flow_status: FlowStatusHandler,
    on_flow_appearance: FlowAppearanceHandler,
    on_upsert_flow_appearance: UpsertFlowAppearanceHandler,
    on_enqueue_perturbation: EnqueuePerturbationHandler,
    on_claim_perturbation: ClaimPerturbationHandler,
    on_ack_perturbation: AckPerturbationHandler,
    on_heartbeat_journey_lease: HeartbeatJourneyLeaseHandler,
    on_claim_owner_wake: ClaimOwnerWakeHandler,
    on_flow_complete: FlowCompleteHandler,
    on_flow_alive_if_created: FlowAliveIfCreatedHandler,
    on_claim_work: ClaimWorkHandler,
    on_append_history: AppendHistoryHandler,
    on_schedule_sleep_timer: ScheduleSleepTimerHandler,
    on_poll_timers: PollTimersHandler,
}

impl MockStore {
    pub fn builder() -> MockStoreBuilder {
        MockStoreBuilder::default()
    }
}

impl Default for MockStore {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[async_trait]
impl JungleStore for MockStore {
    async fn migrate(&self) -> Result<()> {
        Ok(())
    }

    async fn create_journey(&self, namespace: String, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        (self.on_create_flow)(namespace, ordinal, seed)
    }

    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>> {
        (self.on_journey_history)(journey_id)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        (self.on_flow_status)(journey_id)
    }

    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>> {
        (self.on_flow_appearance)(journey_id)
    }

    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        (self.on_upsert_flow_appearance)(journey_id, data)
    }

    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        (self.on_enqueue_perturbation)(journey_id, data)
    }

    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>> {
        (self.on_claim_perturbation)(journey_id)
    }

    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()> {
        (self.on_ack_perturbation)(journey_id, perturbation_id)
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<()> {
        (self.on_heartbeat_journey_lease)(journey_id, owner_id, lease_ttl_ms)
    }

    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>> {
        (self.on_claim_owner_wake)(owner_id)
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        (self.on_flow_complete)(journey_id)
    }

    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()> {
        (self.on_flow_alive_if_created)(journey_id)
    }

    async fn claim_work(&self, namespace: String) -> Result<Option<RunnerStep>> {
        (self.on_claim_work)(namespace)
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        (self.on_append_history)(history)
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()> {
        (self.on_schedule_sleep_timer)(journey_id, timer_id, wake_at_unix_ms)
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        (self.on_poll_timers)()
    }
}

#[derive(Default)]
pub struct MockStoreBuilder {
    on_create_flow: Option<CreateFlowHandler>,
    on_journey_history: Option<JourneyHistoryHandler>,
    on_flow_status: Option<FlowStatusHandler>,
    on_flow_appearance: Option<FlowAppearanceHandler>,
    on_upsert_flow_appearance: Option<UpsertFlowAppearanceHandler>,
    on_enqueue_perturbation: Option<EnqueuePerturbationHandler>,
    on_claim_perturbation: Option<ClaimPerturbationHandler>,
    on_ack_perturbation: Option<AckPerturbationHandler>,
    on_heartbeat_journey_lease: Option<HeartbeatJourneyLeaseHandler>,
    on_claim_owner_wake: Option<ClaimOwnerWakeHandler>,
    on_flow_complete: Option<FlowCompleteHandler>,
    on_flow_alive_if_created: Option<FlowAliveIfCreatedHandler>,
    on_claim_work: Option<ClaimWorkHandler>,
    on_append_history: Option<AppendHistoryHandler>,
    on_schedule_sleep_timer: Option<ScheduleSleepTimerHandler>,
    on_poll_timers: Option<PollTimersHandler>,
}

impl MockStoreBuilder {
    pub fn on_create_flow<F>(mut self, f: F) -> Self
    where
        F: Fn(String, u32, Vec<u8>) -> Result<Uuid> + Send + Sync + 'static,
    {
        self.on_create_flow = Some(Arc::new(f));
        self
    }

    pub fn on_claim_work<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> Result<Option<RunnerStep>> + Send + Sync + 'static,
    {
        self.on_claim_work = Some(Arc::new(f));
        self
    }

    pub fn on_journey_history<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<Vec<RunnerOut>> + Send + Sync + 'static,
    {
        self.on_journey_history = Some(Arc::new(f));
        self
    }

    pub fn on_flow_status<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<JourneyStatus> + Send + Sync + 'static,
    {
        self.on_flow_status = Some(Arc::new(f));
        self
    }

    pub fn on_flow_appearance<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<Option<Vec<u8>>> + Send + Sync + 'static,
    {
        self.on_flow_appearance = Some(Arc::new(f));
        self
    }

    pub fn on_upsert_flow_appearance<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Result<()> + Send + Sync + 'static,
    {
        self.on_upsert_flow_appearance = Some(Arc::new(f));
        self
    }

    pub fn on_enqueue_perturbation<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Result<()> + Send + Sync + 'static,
    {
        self.on_enqueue_perturbation = Some(Arc::new(f));
        self
    }

    pub fn on_claim_perturbation<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<Option<ClaimedAnimalPerturbation>> + Send + Sync + 'static,
    {
        self.on_claim_perturbation = Some(Arc::new(f));
        self
    }

    pub fn on_ack_perturbation<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, u64) -> Result<()> + Send + Sync + 'static,
    {
        self.on_ack_perturbation = Some(Arc::new(f));
        self
    }

    pub fn on_heartbeat_journey_lease<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Uuid, i64) -> Result<()> + Send + Sync + 'static,
    {
        self.on_heartbeat_journey_lease = Some(Arc::new(f));
        self
    }

    pub fn on_claim_owner_wake<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<Option<OwnerWake>> + Send + Sync + 'static,
    {
        self.on_claim_owner_wake = Some(Arc::new(f));
        self
    }

    pub fn on_flow_complete<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<()> + Send + Sync + 'static,
    {
        self.on_flow_complete = Some(Arc::new(f));
        self
    }

    pub fn on_flow_alive_if_created<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<()> + Send + Sync + 'static,
    {
        self.on_flow_alive_if_created = Some(Arc::new(f));
        self
    }

    pub fn on_append_history<F>(mut self, f: F) -> Self
    where
        F: Fn(RunnerOut) -> Result<()> + Send + Sync + 'static,
    {
        self.on_append_history = Some(Arc::new(f));
        self
    }

    pub fn on_schedule_sleep_timer<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Uuid, i64) -> Result<()> + Send + Sync + 'static,
    {
        self.on_schedule_sleep_timer = Some(Arc::new(f));
        self
    }

    pub fn on_poll_timers<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<Option<()>> + Send + Sync + 'static,
    {
        self.on_poll_timers = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> MockStore {
        let default_create_flow: CreateFlowHandler = Arc::new(|_, _, _| Ok(Uuid::new_v4()));
        let default_journey_history: JourneyHistoryHandler = Arc::new(|_| Ok(Vec::new()));
        let default_flow_status: FlowStatusHandler = Arc::new(|_| Ok(JourneyStatus::Alive));
        let default_flow_appearance: FlowAppearanceHandler = Arc::new(|_| Ok(None));
        let default_upsert_flow_appearance: UpsertFlowAppearanceHandler = Arc::new(|_, _| Ok(()));
        let default_enqueue_perturbation: EnqueuePerturbationHandler = Arc::new(|_, _| Ok(()));
        let default_claim_perturbation: ClaimPerturbationHandler = Arc::new(|_| Ok(None));
        let default_ack_perturbation: AckPerturbationHandler = Arc::new(|_, _| Ok(()));
        let default_heartbeat_journey_lease: HeartbeatJourneyLeaseHandler =
            Arc::new(|_, _, _| Ok(()));
        let default_claim_owner_wake: ClaimOwnerWakeHandler = Arc::new(|_| Ok(None));
        let default_flow_complete: FlowCompleteHandler = Arc::new(|_| Ok(()));
        let default_flow_alive_if_created: FlowAliveIfCreatedHandler = Arc::new(|_| Ok(()));
        let default_claim_work: ClaimWorkHandler = Arc::new(|_| Ok(None));
        let default_append_history: AppendHistoryHandler = Arc::new(|_| Ok(()));
        let default_schedule_sleep_timer: ScheduleSleepTimerHandler = Arc::new(|_, _, _| Ok(()));
        let default_poll_timers: PollTimersHandler = Arc::new(|| Ok(None));

        MockStore {
            on_create_flow: self
                .on_create_flow
                .unwrap_or_else(|| default_create_flow.clone()),
            on_journey_history: self
                .on_journey_history
                .unwrap_or_else(|| default_journey_history.clone()),
            on_flow_status: self
                .on_flow_status
                .unwrap_or_else(|| default_flow_status.clone()),
            on_flow_appearance: self
                .on_flow_appearance
                .unwrap_or_else(|| default_flow_appearance.clone()),
            on_upsert_flow_appearance: self
                .on_upsert_flow_appearance
                .unwrap_or_else(|| default_upsert_flow_appearance.clone()),
            on_enqueue_perturbation: self
                .on_enqueue_perturbation
                .unwrap_or_else(|| default_enqueue_perturbation.clone()),
            on_claim_perturbation: self
                .on_claim_perturbation
                .unwrap_or_else(|| default_claim_perturbation.clone()),
            on_ack_perturbation: self
                .on_ack_perturbation
                .unwrap_or_else(|| default_ack_perturbation.clone()),
            on_heartbeat_journey_lease: self
                .on_heartbeat_journey_lease
                .unwrap_or_else(|| default_heartbeat_journey_lease.clone()),
            on_claim_owner_wake: self
                .on_claim_owner_wake
                .unwrap_or_else(|| default_claim_owner_wake.clone()),
            on_flow_complete: self
                .on_flow_complete
                .unwrap_or_else(|| default_flow_complete.clone()),
            on_flow_alive_if_created: self
                .on_flow_alive_if_created
                .unwrap_or_else(|| default_flow_alive_if_created.clone()),
            on_claim_work: self
                .on_claim_work
                .unwrap_or_else(|| default_claim_work.clone()),
            on_append_history: self
                .on_append_history
                .unwrap_or_else(|| default_append_history.clone()),
            on_schedule_sleep_timer: self
                .on_schedule_sleep_timer
                .unwrap_or_else(|| default_schedule_sleep_timer.clone()),
            on_poll_timers: self
                .on_poll_timers
                .unwrap_or_else(|| default_poll_timers.clone()),
        }
    }
}
