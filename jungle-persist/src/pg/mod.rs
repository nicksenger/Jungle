pub mod migrations;

use async_trait::async_trait;
use crate::{JungleStore, Result};
use jungle_types::{RunnerOut, Work};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PgStore {
    pool: sqlx::PgPool,
}

impl PgStore {
    pub fn builder() -> PgStoreBuilder {
        PgStoreBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct PgStoreBuilder {
    connection_string: Option<String>,
    max_connections: u32,
}

impl Default for PgStoreBuilder {
    fn default() -> Self {
        Self {
            connection_string: None,
            max_connections: 10,
        }
    }
}

impl PgStoreBuilder {
    pub fn connection_string(mut self, value: impl Into<String>) -> Self {
        self.connection_string = Some(value.into());
        self
    }

    pub fn max_connections(mut self, value: u32) -> Self {
        self.max_connections = value;
        self
    }

    pub async fn build(self) -> Result<PgStore> {
        let connection_string = self
            .connection_string
            .ok_or(crate::PersistenceError::MissingPostgresConnectionString)?;
        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .connect(&connection_string)
            .await
            .map_err(crate::PersistenceError::PostgresConnect)?;
        Ok(PgStore { pool })
    }
}

#[async_trait]
impl JungleStore for PgStore {
    async fn migrate(&self) -> Result<()> {
        let _ = &self.pool;
        todo!()
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        let _ = &self.pool;
        todo!()
    }

    async fn append_history(&self, _history: RunnerOut) -> Result<()> {
        let _ = &self.pool;
        todo!()
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let _ = &self.pool;
        todo!()
    }

    async fn details(&self, _flow_id: Uuid) -> Result<()> {
        let _ = &self.pool;
        todo!()
    }
}
