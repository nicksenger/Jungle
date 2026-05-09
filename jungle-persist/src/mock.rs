use std::sync::Arc;

use async_trait::async_trait;
use jungle_types::{JourneyStatus, RunnerOut, RunnerStep};
use uuid::Uuid;

use crate::{JungleStore, Result};

type ClaimWorkHandler = Arc<dyn Fn() -> Result<Option<RunnerStep>> + Send + Sync + 'static>;
type CreateFlowHandler = Arc<dyn Fn(u32, Vec<u8>) -> Result<Uuid> + Send + Sync + 'static>;
type FlowStatusHandler = Arc<dyn Fn(Uuid) -> Result<JourneyStatus> + Send + Sync + 'static>;
type FlowCompleteHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;
type FlowAliveIfCreatedHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;
type AppendHistoryHandler = Arc<dyn Fn(RunnerOut) -> Result<()> + Send + Sync + 'static>;
type PollTimersHandler = Arc<dyn Fn() -> Result<Option<()>> + Send + Sync + 'static>;
type DetailsHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockStore {
    on_create_flow: CreateFlowHandler,
    on_flow_status: FlowStatusHandler,
    on_flow_complete: FlowCompleteHandler,
    on_flow_alive_if_created: FlowAliveIfCreatedHandler,
    on_claim_work: ClaimWorkHandler,
    on_append_history: AppendHistoryHandler,
    on_poll_timers: PollTimersHandler,
    on_details: DetailsHandler,
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

    async fn create_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        (self.on_create_flow)(ordinal, seed)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        (self.on_flow_status)(journey_id)
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        (self.on_flow_complete)(journey_id)
    }

    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()> {
        (self.on_flow_alive_if_created)(journey_id)
    }

    async fn claim_work(&self) -> Result<Option<RunnerStep>> {
        (self.on_claim_work)()
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        (self.on_append_history)(history)
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        (self.on_poll_timers)()
    }

    async fn details(&self, journey_id: Uuid) -> Result<()> {
        (self.on_details)(journey_id)
    }
}

#[derive(Default)]
pub struct MockStoreBuilder {
    on_create_flow: Option<CreateFlowHandler>,
    on_flow_status: Option<FlowStatusHandler>,
    on_flow_complete: Option<FlowCompleteHandler>,
    on_flow_alive_if_created: Option<FlowAliveIfCreatedHandler>,
    on_claim_work: Option<ClaimWorkHandler>,
    on_append_history: Option<AppendHistoryHandler>,
    on_poll_timers: Option<PollTimersHandler>,
    on_details: Option<DetailsHandler>,
}

impl MockStoreBuilder {
    pub fn on_create_flow<F>(mut self, f: F) -> Self
    where
        F: Fn(u32, Vec<u8>) -> Result<Uuid> + Send + Sync + 'static,
    {
        self.on_create_flow = Some(Arc::new(f));
        self
    }

    pub fn on_claim_work<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<Option<RunnerStep>> + Send + Sync + 'static,
    {
        self.on_claim_work = Some(Arc::new(f));
        self
    }

    pub fn on_flow_status<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<JourneyStatus> + Send + Sync + 'static,
    {
        self.on_flow_status = Some(Arc::new(f));
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

    pub fn on_poll_timers<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<Option<()>> + Send + Sync + 'static,
    {
        self.on_poll_timers = Some(Arc::new(f));
        self
    }

    pub fn on_details<F>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Result<()> + Send + Sync + 'static,
    {
        self.on_details = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> MockStore {
        let default_create_flow: CreateFlowHandler = Arc::new(|_, _| Ok(Uuid::new_v4()));
        let default_flow_status: FlowStatusHandler = Arc::new(|_| Ok(JourneyStatus::Alive));
        let default_flow_complete: FlowCompleteHandler = Arc::new(|_| Ok(()));
        let default_flow_alive_if_created: FlowAliveIfCreatedHandler = Arc::new(|_| Ok(()));
        let default_claim_work: ClaimWorkHandler = Arc::new(|| Ok(None));
        let default_append_history: AppendHistoryHandler = Arc::new(|_| Ok(()));
        let default_poll_timers: PollTimersHandler = Arc::new(|| Ok(None));
        let default_details: DetailsHandler = Arc::new(|_| Ok(()));

        MockStore {
            on_create_flow: self
                .on_create_flow
                .unwrap_or_else(|| default_create_flow.clone()),
            on_flow_status: self
                .on_flow_status
                .unwrap_or_else(|| default_flow_status.clone()),
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
            on_poll_timers: self
                .on_poll_timers
                .unwrap_or_else(|| default_poll_timers.clone()),
            on_details: self.on_details.unwrap_or(default_details),
        }
    }
}
