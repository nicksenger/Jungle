//! PostgreSQL persistence migrations.

use super::PgStore;
use crate::Result;
use tracing::warn;

impl PgStore {
    pub(super) async fn migrate_v0(&self) -> Result<()> {
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
        }

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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS work_items (
                id UUID PRIMARY KEY,
                flow_id UUID NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
                kind SMALLINT NOT NULL,
                expiry TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        if version_row.is_none() {
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
