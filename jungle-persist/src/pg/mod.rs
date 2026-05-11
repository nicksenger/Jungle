pub mod migrations;

use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use jungle_types::JourneyStatus;
use jungle_types::{ClaimedAnimalPerturbation, OwnerWake, RunnerOut, SupportedAnimal, Work};
use serde::{Deserialize, Serialize};
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

    async fn create_journey(
        &self,
        namespace: String,
        animal_id: u32,
        client_observed_generation: Option<u32>,
        seed: Vec<u8>,
    ) -> Result<Uuid> {
        let journey_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();
        let animal_id = i32::try_from(animal_id).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "animal id exceeds i32 range for postgres: {animal_id}"
            ))
        })?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        let generation = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT generation
            FROM animal_generations
            WHERE namespace = $1 AND animal_id = $2
            "#,
        )
        .bind(namespace.as_str())
        .bind(animal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?
        .unwrap_or(0);

        if let Some(observed_generation) = client_observed_generation {
            let observed_generation = i32::try_from(observed_generation).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "client observed generation exceeds i32 range for postgres: {observed_generation}"
                ))
            })?;
            if observed_generation > generation {
                return Err(crate::PersistenceError::Message(format!(
                    "client observed generation {observed_generation} exceeds latest server generation {generation} for namespace {namespace} animal {animal_id}"
                )));
            }
        }

        sqlx::query(
            r#"
            INSERT INTO journeys (id, namespace, animal_id, generation, status, seed)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(journey_id)
        .bind(namespace)
        .bind(animal_id)
        .bind(generation)
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

    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>> {
        #[derive(Debug, sqlx::FromRow)]
        struct HistoryRow {
            kind: i16,
            data: Vec<u8>,
        }

        let rows = sqlx::query_as::<_, HistoryRow>(
            r#"
            SELECT kind, data
            FROM events
            WHERE journey_id = $1
            ORDER BY sequence_id
            "#,
        )
        .bind(journey_id)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let mut history = Vec::with_capacity(rows.len());
        for row in rows {
            history.push(decode_history_row(journey_id, row.kind, row.data)?);
        }
        Ok(history)
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

    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<()> {
        let lease_ttl_ms = lease_ttl_ms.max(0);
        sqlx::query(
            r#"
            INSERT INTO journey_leases (journey_id, owner_id, lease_until, heartbeat_at)
            VALUES ($1, $2, NOW() + ($3 * INTERVAL '1 millisecond'), NOW())
            ON CONFLICT (journey_id)
            DO UPDATE SET owner_id = EXCLUDED.owner_id,
                          lease_until = EXCLUDED.lease_until,
                          heartbeat_at = EXCLUDED.heartbeat_at
            "#,
        )
        .bind(journey_id)
        .bind(owner_id)
        .bind(lease_ttl_ms)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;
        Ok(())
    }

    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>> {
        #[derive(Debug, sqlx::FromRow)]
        struct WakeRow {
            journey_id: Uuid,
            timer_id: Uuid,
        }

        let wake = sqlx::query_as::<_, WakeRow>(
            r#"
            WITH next_wake AS (
                SELECT id
                FROM owner_wakes
                WHERE owner_id = $1
                ORDER BY created_at, id
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            DELETE FROM owner_wakes ow
            USING next_wake nw
            WHERE ow.id = nw.id
            RETURNING ow.journey_id, ow.timer_id
            "#,
        )
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(wake.map(|row| OwnerWake {
            journey_id: row.journey_id,
            timer_id: row.timer_id,
        }))
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

    async fn claim_work(
        &self,
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>> {
        if supported_animals.is_empty() {
            return Ok(None);
        }

        let mut supported_ids = Vec::<i32>::with_capacity(supported_animals.len());
        let mut supported_generations = Vec::<i32>::with_capacity(supported_animals.len());
        for supported in &supported_animals {
            supported_ids.push(i32::try_from(supported.animal_id).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "supported animal id exceeds i32 range for postgres: {}",
                    supported.animal_id
                ))
            })?);
            supported_generations.push(i32::try_from(supported.generation).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "supported animal generation exceeds i32 range for postgres: {}",
                    supported.generation
                ))
            })?);
        }

        for supported in supported_animals {
            let animal_id = i32::try_from(supported.animal_id).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "supported animal id exceeds i32 range for postgres: {}",
                    supported.animal_id
                ))
            })?;
            let generation = i32::try_from(supported.generation).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "supported animal generation exceeds i32 range for postgres: {}",
                    supported.generation
                ))
            })?;
            sqlx::query(
                r#"
                INSERT INTO animal_generations (namespace, animal_id, generation, updated_at)
                VALUES ($1, $2, $3, NOW())
                ON CONFLICT (namespace, animal_id)
                DO UPDATE SET
                    generation = EXCLUDED.generation,
                    updated_at = NOW()
                WHERE animal_generations.generation < EXCLUDED.generation
                "#,
            )
            .bind(namespace.as_str())
            .bind(animal_id)
            .bind(generation)
            .execute(&self.pool)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        }

        #[derive(Debug, sqlx::FromRow)]
        struct ClaimedWorkRow {
            journey_id: Uuid,
            kind: i16,
            animal_id: i32,
            generation: i32,
            seed: Vec<u8>,
        }

        let row = sqlx::query_as::<_, ClaimedWorkRow>(
            r#"
            WITH supported AS (
                SELECT * FROM UNNEST($2::INT4[], $3::INT4[]) AS s(animal_id, generation)
            ),
            candidate AS (
                SELECT wi.id
                FROM work_items wi
                INNER JOIN journeys j ON j.id = wi.journey_id
                INNER JOIN supported s
                    ON s.animal_id = j.animal_id
                   AND s.generation = j.generation
                WHERE j.namespace = $1
                  AND (wi.status = $2 OR (wi.status = $3 AND wi.expiry < NOW()))
                ORDER BY wi.expiry, wi.id
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            ),
            claimed AS (
                UPDATE work_items wi
                SET status = $3,
                    expiry = NOW() + INTERVAL '30 seconds'
                FROM candidate c
                WHERE wi.id = c.id
                RETURNING wi.journey_id, wi.kind
            )
            SELECT c.journey_id, c.kind, f.animal_id, f.generation, f.seed
            FROM claimed c
            INNER JOIN journeys f ON f.id = c.journey_id
            "#,
        )
        .bind(namespace)
        .bind(supported_ids)
        .bind(supported_generations)
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
                let animal_id = u32::try_from(row.animal_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative animal_id in postgres journey: {}",
                        row.animal_id
                    ))
                })?;
                let generation = u32::try_from(row.generation).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative generation in postgres journey: {}",
                        row.generation
                    ))
                })?;
                Work::StartJourney {
                    journey_id: row.journey_id,
                    animal_id,
                    generation,
                    seed: row.seed,
                }
            }
            1 => {
                let animal_id = u32::try_from(row.animal_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative animal_id in postgres journey: {}",
                        row.animal_id
                    ))
                })?;
                let generation = u32::try_from(row.generation).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative generation in postgres journey: {}",
                        row.generation
                    ))
                })?;
                Work::ResumeJourney {
                    journey_id: row.journey_id,
                    animal_id,
                    generation,
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
            RunnerOut::ActionInput {
                node_id,
                data,
                uuid,
            } => (uuid, 0_i16, encode_action_event(node_id, data)?),
            RunnerOut::ActionSuccessOutput {
                node_id,
                data,
                uuid,
            } => (uuid, 1_i16, encode_action_event(node_id, data)?),
            RunnerOut::ActionFailureOutput {
                node_id,
                data,
                uuid,
            } => (uuid, 2_i16, encode_action_event(node_id, data)?),
            RunnerOut::SleepScheduled {
                uuid,
                timer_id,
                wake_at_unix_ms,
            } => (
                uuid,
                3_i16,
                postcard::to_allocvec(&SleepScheduledEvent {
                    timer_id,
                    wake_at_unix_ms,
                })
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?,
            ),
            RunnerOut::SleepFired {
                uuid,
                timer_id,
                fired_at_unix_ms,
            } => (
                uuid,
                4_i16,
                postcard::to_allocvec(&SleepFiredEvent {
                    timer_id,
                    fired_at_unix_ms,
                })
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?,
            ),
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

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO timer_tasks (id, journey_id, status, visible_at, fired_at)
            VALUES ($1, $2, $3, to_timestamp($4::double precision / 1000.0), NULL)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(timer_id)
        .bind(journey_id)
        .bind(0_i16)
        .bind(wake_at_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        self.append_history(RunnerOut::SleepScheduled {
            uuid: journey_id,
            timer_id,
            wake_at_unix_ms,
        })
        .await?;

        Ok(())
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        #[derive(Debug, sqlx::FromRow)]
        struct DueTimerRow {
            id: Uuid,
            journey_id: Uuid,
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        let due = sqlx::query_as::<_, DueTimerRow>(
            r#"
            SELECT id, journey_id
            FROM timer_tasks
            WHERE status = $1 AND visible_at <= NOW()
            ORDER BY visible_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
        )
        .bind(0_i16)
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let Some(due) = due else {
            tx.commit()
                .await
                .map_err(crate::PersistenceError::PostgresQuery)?;
            return Ok(None);
        };

        sqlx::query(
            r#"
            UPDATE timer_tasks
            SET status = $2, fired_at = NOW()
            WHERE id = $1 AND status = $3
            "#,
        )
        .bind(due.id)
        .bind(1_i16)
        .bind(0_i16)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let fired_at_unix_ms = chrono::Utc::now().timestamp_millis();
        let sleep_fired_data = postcard::to_allocvec(&SleepFiredEvent {
            timer_id: due.id,
            fired_at_unix_ms,
        })
        .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;

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
        .bind(due.journey_id)
        .bind(4_i16)
        .bind(sleep_fired_data)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let owner_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT owner_id
            FROM journey_leases
            WHERE journey_id = $1 AND lease_until > NOW()
            LIMIT 1
            "#,
        )
        .bind(due.journey_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        if let Some(owner_id) = owner_id {
            sqlx::query(
                r#"
                INSERT INTO owner_wakes (id, owner_id, journey_id, timer_id, created_at)
                VALUES ($1, $2, $3, $4, NOW())
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(owner_id)
            .bind(due.journey_id)
            .bind(due.id)
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO work_items (id, journey_id, kind, status, expiry)
                VALUES ($1, $2, $3, $4, NOW())
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(due.journey_id)
            .bind(1_i16)
            .bind(0_i16)
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        }

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(Some(()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SleepScheduledEvent {
    timer_id: Uuid,
    wake_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SleepFiredEvent {
    timer_id: Uuid,
    fired_at_unix_ms: i64,
}

const ACTION_EVENT_ENVELOPE_V1: u8 = 0xA1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionEventData {
    node_id: u32,
    data: Vec<u8>,
}

fn encode_action_event(node_id: u32, data: Vec<u8>) -> Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(1 + data.len() + 8);
    payload.push(ACTION_EVENT_ENVELOPE_V1);
    let encoded = postcard::to_allocvec(&ActionEventData { node_id, data })
        .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
    payload.extend_from_slice(&encoded);
    Ok(payload)
}

fn decode_action_event(data: Vec<u8>) -> Result<(u32, Vec<u8>)> {
    if data.first().copied() != Some(ACTION_EVENT_ENVELOPE_V1) {
        return Ok((0, data));
    }
    let envelope: ActionEventData = postcard::from_bytes(&data[1..])
        .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
    Ok((envelope.node_id, envelope.data))
}

fn decode_history_row(journey_id: Uuid, kind: i16, data: Vec<u8>) -> Result<RunnerOut> {
    match kind {
        0 => {
            let (node_id, data) = decode_action_event(data)?;
            Ok(RunnerOut::ActionInput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        1 => {
            let (node_id, data) = decode_action_event(data)?;
            Ok(RunnerOut::ActionSuccessOutput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        2 => {
            let (node_id, data) = decode_action_event(data)?;
            Ok(RunnerOut::ActionFailureOutput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        3 => {
            let event: SleepScheduledEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerOut::SleepScheduled {
                uuid: journey_id,
                timer_id: event.timer_id,
                wake_at_unix_ms: event.wake_at_unix_ms,
            })
        }
        4 => {
            let event: SleepFiredEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerOut::SleepFired {
                uuid: journey_id,
                timer_id: event.timer_id,
                fired_at_unix_ms: event.fired_at_unix_ms,
            })
        }
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported event kind in postgres: {other}"
        ))),
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
