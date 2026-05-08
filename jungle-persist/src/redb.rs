use async_trait::async_trait;
use crate::{Result, Store};
use jungle_types::{RunnerOut, Work};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct RedbStore;

#[async_trait]
impl Store for RedbStore {
    async fn migrate(&self) -> Result<()> {
        Ok(())
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        Ok(None)
    }

    async fn append_history(&self, _history: RunnerOut) -> Result<()> {
        Ok(())
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        Ok(None)
    }

    async fn details(&self, _flow_id: Uuid) -> Result<()> {
        Ok(())
    }
}
