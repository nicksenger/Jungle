pub mod migrations;

use async_trait::async_trait;
use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use jungle_types::{RunnerOut, Work};
use sqlx::postgres::PgPoolOptions;
use tracing::warn;
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
        match SCHEMA_VERSION {
            SchemaVersion::V0 => self.migrate_v0().await,
        }
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

impl PgStore {
    async fn migrate_v0(&self) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jungle_schema_metadata (
                id SMALLINT PRIMARY KEY,
                version INTEGER NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let version_row = sqlx::query_scalar::<_, i32>(
            "SELECT version FROM jungle_schema_metadata WHERE id = 1",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        if let Some(version) = version_row {
            if version != 0 {
                warn!(
                    expected_schema_version = 0,
                    actual_schema_version = version,
                    "postgres schema version mismatch"
                );
            }
        } else {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS flows (
                    id UUID PRIMARY KEY,
                    ordinal INTEGER NOT NULL,
                    seed BYTEA NOT NULL
                )
                "#,
            )
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS events (
                    flow_id UUID NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
                    sequence_id BIGINT NOT NULL,
                    kind SMALLINT NOT NULL,
                    data BYTEA NOT NULL,
                    PRIMARY KEY (flow_id, sequence_id)
                )
                "#,
            )
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

            sqlx::query("INSERT INTO jungle_schema_metadata (id, version) VALUES (1, 0)")
                .execute(&mut *tx)
                .await
                .map_err(crate::PersistenceError::PostgresQuery)?;
        }

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }
}
