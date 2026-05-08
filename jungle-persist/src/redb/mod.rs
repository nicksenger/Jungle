pub mod migrations;

use async_trait::async_trait;
use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use jungle_types::{RunnerOut, Work};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RedbStore {
    db: Arc<redb::Database>,
}

impl RedbStore {
    pub fn builder() -> RedbStoreBuilder {
        RedbStoreBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RedbStoreBuilder {
    path: Option<PathBuf>,
}

impl RedbStoreBuilder {
    pub fn path(mut self, value: impl Into<PathBuf>) -> Self {
        self.path = Some(value.into());
        self
    }

    pub fn build(self) -> Result<RedbStore> {
        let path = self.path.ok_or(crate::PersistenceError::MissingRedbPath)?;
        let db = redb::Database::create(path).map_err(crate::PersistenceError::RedbOpen)?;
        Ok(RedbStore { db: Arc::new(db) })
    }
}

#[async_trait]
impl JungleStore for RedbStore {
    async fn migrate(&self) -> Result<()> {
        match SCHEMA_VERSION {
            SchemaVersion::V0 => self.migrate_v0().await,
        }
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        let _ = &self.db;
        todo!()
    }

    async fn append_history(&self, _history: RunnerOut) -> Result<()> {
        let _ = &self.db;
        todo!()
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let _ = &self.db;
        todo!()
    }

    async fn details(&self, _flow_id: Uuid) -> Result<()> {
        let _ = &self.db;
        todo!()
    }
}
