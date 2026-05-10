pub mod migrations;

use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use jungle_types::JourneyStatus;
use jungle_types::{ClaimedAnimalPerturbation, RunnerOut, RunnerStep};
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

    async fn create_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        let journey_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();
        let ordinal = i32::try_from(ordinal).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "journey ordinal exceeds i32 range for postgres: {ordinal}"
            ))
        })?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query(
            r#"
            INSERT INTO journeys (id, ordinal, status, seed)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(journey_id)
        .bind(ordinal)
        .bind(0_i16)
        .bind(seed)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query(
            r#"
            INSERT INTO work_items (id, journey_id, kind, status, expiry)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(work_item_id)
        .bind(journey_id)
        .bind(0_i16)
        .bind(0_i16)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(journey_id)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        let status = sqlx::query_scalar::<_, i16>(
            r#"
            SELECT status
            FROM journeys
            WHERE id = $1
            "#,
        )
        .bind(journey_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?
        .ok_or_else(|| {
            crate::PersistenceError::Message(format!("journey not found: {journey_id}"))
        })?;

        decode_journey_status(status)
    }

    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>> {
        let appearance = sqlx::query_scalar::<_, Vec<u8>>(
            r#"
            SELECT data
            FROM animal_appearances
            WHERE journey_id = $1
            "#,
        )
        .bind(journey_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(appearance)
    }

    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO animal_appearances (journey_id, data, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (journey_id)
            DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()
            "#,
        )
        .bind(journey_id)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        sqlx::query(
            r#"
            WITH next_sequence AS (
                SELECT COALESCE(MAX(sequence_id) + 1, 0) AS sequence_id
                FROM animal_perturbations
                WHERE journey_id = $1
            )
            INSERT INTO animal_perturbations (
                journey_id,
                sequence_id,
                data,
                status,
                claimed_at,
                lease_until
            )
            SELECT $1, next_sequence.sequence_id, $2, $3, NULL, NULL
            FROM next_sequence
            "#,
        )
        .bind(journey_id)
        .bind(data)
        .bind(0_i16)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>> {
        #[derive(Debug, sqlx::FromRow)]
        struct ClaimedRow {
            sequence_id: i64,
            data: Vec<u8>,
        }

        let row = sqlx::query_as::<_, ClaimedRow>(
            r#"
            WITH next_item AS (
                SELECT journey_id, sequence_id
                FROM animal_perturbations
                WHERE journey_id = $1
                  AND (status = $2 OR (status = $3 AND lease_until < NOW()))
                ORDER BY sequence_id
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            ),
            claimed AS (
                UPDATE animal_perturbations ap
                SET status = $3,
                    claimed_at = NOW(),
                    lease_until = NOW() + INTERVAL '30 seconds'
                FROM next_item ni
                WHERE ap.journey_id = ni.journey_id
                  AND ap.sequence_id = ni.sequence_id
                RETURNING ap.sequence_id, ap.data
            )
            SELECT sequence_id, data
            FROM claimed
            "#,
        )
        .bind(journey_id)
        .bind(0_i16)
        .bind(1_i16)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let id = u64::try_from(row.sequence_id).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "negative sequence_id claimed for perturbation: {}",
                row.sequence_id
            ))
        })?;

        Ok(Some(ClaimedAnimalPerturbation { id, data: row.data }))
    }

    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()> {
        let sequence_id = i64::try_from(perturbation_id).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "perturbation id exceeds i64 range for postgres: {perturbation_id}"
            ))
        })?;
        let result = sqlx::query(
            r#"
            DELETE FROM animal_perturbations
            WHERE journey_id = $1 AND sequence_id = $2
            "#,
        )
        .bind(journey_id)
        .bind(sequence_id)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        if result.rows_affected() == 0 {
            return Err(crate::PersistenceError::Message(format!(
                "animal perturbation not found for ack: {journey_id}:{perturbation_id}"
            )));
        }

        Ok(())
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE journeys
            SET status = $2
            WHERE id = $1
            "#,
        )
        .bind(journey_id)
        .bind(encode_journey_status(JourneyStatus::Completed))
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;
        if result.rows_affected() == 0 {
            return Err(crate::PersistenceError::Message(format!(
                "journey not found: {journey_id}"
            )));
        }

        Ok(())
    }

    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE journeys
            SET status = $2
            WHERE id = $1 AND status = $3
            "#,
        )
        .bind(journey_id)
        .bind(encode_journey_status(JourneyStatus::Alive))
        .bind(encode_journey_status(JourneyStatus::Created))
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn claim_work(&self) -> Result<Option<RunnerStep>> {
        #[derive(Debug, sqlx::FromRow)]
        struct ClaimedWorkRow {
            journey_id: Uuid,
            kind: i16,
            ordinal: i32,
            seed: Vec<u8>,
        }

        let row = sqlx::query_as::<_, ClaimedWorkRow>(
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
                RETURNING wi.journey_id, wi.kind
            )
            SELECT c.journey_id, c.kind, f.ordinal, f.seed
            FROM claimed c
            INNER JOIN journeys f ON f.id = c.journey_id
            "#,
        )
        .bind(0_i16)
        .bind(1_i16)
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
                        "invalid negative ordinal in postgres journey: {}",
                        row.ordinal
                    ))
                })?;
                RunnerStep::StartJourney {
                    journey_id: row.journey_id,
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
        let (journey_id, kind, data) = match history {
            RunnerOut::ActionInput { data, uuid } => (uuid, 0_i16, data),
            RunnerOut::ActionSuccessOutput { data, uuid } => (uuid, 1_i16, data),
            RunnerOut::ActionFailureOutput { data, uuid } => (uuid, 2_i16, data),
            RunnerOut::Appearance { .. } => {
                return Err(crate::PersistenceError::Message(
                    "appearance snapshots are not history events in postgres".to_string(),
                ))
            }
        };

        sqlx::query(
            r#"
            WITH next_sequence AS (
                SELECT COALESCE(MAX(sequence_id) + 1, 0) AS sequence_id
                FROM events
                WHERE journey_id = $1
            )
            INSERT INTO events (journey_id, sequence_id, kind, data)
            SELECT $1, next_sequence.sequence_id, $2, $3
            FROM next_sequence
            "#,
        )
        .bind(journey_id)
        .bind(kind)
        .bind(data)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let _ = &self.pool;
        todo!()
    }
}

fn encode_journey_status(status: JourneyStatus) -> i16 {
    match status {
        JourneyStatus::Created => 0,
        JourneyStatus::Alive => 1,
        JourneyStatus::Stopped => 2,
        JourneyStatus::Completed => 3,
        JourneyStatus::Dead => 4,
    }
}

fn decode_journey_status(status: i16) -> Result<JourneyStatus> {
    match status {
        0 => Ok(JourneyStatus::Created),
        1 => Ok(JourneyStatus::Alive),
        2 => Ok(JourneyStatus::Stopped),
        3 => Ok(JourneyStatus::Completed),
        4 => Ok(JourneyStatus::Dead),
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported journey status in postgres: {other}"
        ))),
    }
}
