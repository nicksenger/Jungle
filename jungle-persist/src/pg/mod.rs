pub mod migrations;

use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
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
        match SCHEMA_VERSION {
            SchemaVersion::V0 => self.migrate_v0().await,
        }
    }

    async fn create_flow(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        let flow_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();
        let ordinal = i32::try_from(ordinal).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "flow ordinal exceeds i32 range for postgres: {ordinal}"
            ))
        })?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query(
            r#"
            INSERT INTO flows (id, ordinal, seed)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(flow_id)
        .bind(ordinal)
        .bind(seed)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query(
            r#"
            INSERT INTO work_items (id, flow_id, kind, status, expiry)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(work_item_id)
        .bind(flow_id)
        .bind(0_i16)
        .bind(0_i16)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(flow_id)
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        #[derive(Debug)]
        struct ClaimedWorkRow {
            flow_id: Uuid,
            kind: i16,
            ordinal: i32,
            seed: Vec<u8>,
        }

        let row = sqlx::query_as!(
            ClaimedWorkRow,
            r#"
            WITH candidate AS (
                SELECT id
                FROM work_items
                WHERE status = $1
                ORDER BY expiry, id
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            ),
            claimed AS (
                UPDATE work_items wi
                SET status = $2
                FROM candidate c
                WHERE wi.id = c.id
                RETURNING wi.flow_id, wi.kind
            )
            SELECT c.flow_id, c.kind, f.ordinal, f.seed
            FROM claimed c
            INNER JOIN flows f ON f.id = c.flow_id
            "#,
            0_i16,
            1_i16
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let work = match row.kind {
            0 => {
                let ordinal = u32::try_from(row.ordinal).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative ordinal in postgres flow: {}",
                        row.ordinal
                    ))
                })?;
                Work::StartFlow {
                    flow_id: row.flow_id,
                    ordinal,
                    seed: row.seed,
                }
            }
            kind => {
                return Err(crate::PersistenceError::Message(format!(
                    "unsupported work item kind in postgres: {kind}"
                )));
            }
        };

        Ok(Some(work))
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        let (flow_id, kind, data) = match history {
            RunnerOut::ActionInput { data, uuid } => (uuid, 0_i16, data),
            RunnerOut::ActionSuccessOutput { data, uuid } => (uuid, 1_i16, data),
            RunnerOut::ActionFailureOutput { data, uuid } => (uuid, 2_i16, data),
        };

        sqlx::query!(
            r#"
            WITH next_sequence AS (
                SELECT COALESCE(MAX(sequence_id) + 1, 0) AS sequence_id
                FROM events
                WHERE flow_id = $1
            )
            INSERT INTO events (flow_id, sequence_id, kind, data)
            SELECT $1, next_sequence.sequence_id, $2, $3
            FROM next_sequence
            "#,
            flow_id,
            kind,
            data
        )
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
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
