use crate::{Result, Store};
use jungle_types::{RunnerOut, Work};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct RedbStore;

impl Store for RedbStore {
    fn claim_work(&self) -> Result<Option<Work>> {
        Ok(None)
    }

    fn append_history(&self, _history: RunnerOut) -> Result<()> {
        Ok(())
    }

    fn poll_timers(&self) -> Result<Option<()>> {
        Ok(None)
    }

    fn details(&self, _flow_id: Uuid) -> Result<()> {
        Ok(())
    }
}
