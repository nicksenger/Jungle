use std::sync::Arc;

use async_trait::async_trait;
use jungle_types::{RunnerOut, Work};
use uuid::Uuid;

use crate::{Result, Store};

type ClaimWorkHandler = Arc<dyn Fn() -> Result<Option<Work>> + Send + Sync + 'static>;
type AppendHistoryHandler = Arc<dyn Fn(RunnerOut) -> Result<()> + Send + Sync + 'static>;
type PollTimersHandler = Arc<dyn Fn() -> Result<Option<()>> + Send + Sync + 'static>;
type DetailsHandler = Arc<dyn Fn(Uuid) -> Result<()> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockStore {
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
impl Store for MockStore {
    async fn migrate(&self) -> Result<()> {
        Ok(())
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        (self.on_claim_work)()
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        (self.on_append_history)(history)
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        (self.on_poll_timers)()
    }

    async fn details(&self, flow_id: Uuid) -> Result<()> {
        (self.on_details)(flow_id)
    }
}

#[derive(Default)]
pub struct MockStoreBuilder {
    on_claim_work: Option<ClaimWorkHandler>,
    on_append_history: Option<AppendHistoryHandler>,
    on_poll_timers: Option<PollTimersHandler>,
    on_details: Option<DetailsHandler>,
}

impl MockStoreBuilder {
    pub fn on_claim_work<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Result<Option<Work>> + Send + Sync + 'static,
    {
        self.on_claim_work = Some(Arc::new(f));
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
        let default_claim_work: ClaimWorkHandler = Arc::new(|| Ok(None));
        let default_append_history: AppendHistoryHandler = Arc::new(|_| Ok(()));
        let default_poll_timers: PollTimersHandler = Arc::new(|| Ok(None));
        let default_details: DetailsHandler = Arc::new(|_| Ok(()));

        MockStore {
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
