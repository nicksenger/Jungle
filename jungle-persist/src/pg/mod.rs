use crate::models::{SchemaVersion, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use jungle_types::{
    ClaimedPerturbable, JourneyUpdateEvent, OwnerWake, RunnerOut, RunnerUpdateOut, SupportedAnimal,
    Work,
};
use jungle_types::{JourneyRecord, JourneyStatus};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

const PG_JOURNEY_EVENTS_CHANNEL: &str = "jungle_journey_events";
const PG_UPDATES_LOG_INTERVAL: usize = 256;
const PG_SLOW_UPDATES_FETCH_WARN_THRESHOLD_MS: u128 = 50;
const PG_STALE_EVENT_WARN_MS: i64 = 1_000;

static PG_UPDATES_FETCH_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct PgStore {
    pool: sqlx::PgPool,
}

impl PgStore {
    pub fn builder() -> PgStoreBuilder {
        PgStoreBuilder::default()
    }

    async fn notify_journey_event(&self, journey_id: Uuid) -> Result<()> {
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(PG_JOURNEY_EVENTS_CHANNEL)
            .bind(journey_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        Ok(())
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
            SchemaVersion::V0 => jungle_migrate::migrate_postgres_v0(&self.pool)
                .await
                .map_err(crate::PersistenceError::PostgresQuery),
        }
    }

    async fn create_journey(
        &self,
        namespace: String,
        animal_id: u32,
        generation: u32,
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

        let latest_generation = sqlx::query_scalar!(
            r#"
            SELECT generation
            FROM animal_generations
            WHERE namespace = $1 AND animal_id = $2
            "#,
            namespace.as_str(),
            animal_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?
        .unwrap_or(0);

        let generation = i32::try_from(generation).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "client generation exceeds i32 range for postgres: {generation}"
            ))
        })?;
        if generation > latest_generation {
            return Err(crate::PersistenceError::Message(format!(
                "client generation {generation} exceeds latest server generation {latest_generation} for namespace {namespace} animal {animal_id}"
            )));
        }

        sqlx::query!(
            r#"
            INSERT INTO journeys (id, namespace, animal_id, generation, status, seed)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            journey_id,
            namespace,
            animal_id,
            latest_generation,
            0_i16,
            seed
        )
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        sqlx::query!(
            r#"
            INSERT INTO work_items (id, journey_id, kind, status, expiry)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
            work_item_id,
            journey_id,
            0_i16,
            0_i16
        )
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        self.notify_journey_event(journey_id).await?;

        Ok(journey_id)
    }

    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>> {
        let rows = sqlx::query!(
            r#"
            SELECT kind, data
            FROM events
            WHERE journey_id = $1
            ORDER BY sequence_id
            "#,
            journey_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let mut history = Vec::with_capacity(rows.len());
        for row in rows {
            history.push(decode_history_row(journey_id, row.kind, row.data)?);
        }
        Ok(history)
    }

    async fn journey_update_events_since(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<Vec<JourneyUpdateEvent>> {
        let fetch_started_at = Instant::now();
        let after_sequence_id = after_sequence_id
            .map(|seq| {
                i64::try_from(seq).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "sequence id exceeds i64 range for postgres: {seq}"
                    ))
                })
            })
            .transpose()?;

        let rows = sqlx::query(
            r#"
            SELECT
                sequence_id,
                event_unix_ms,
                kind,
                node_id,
                CASE
                    WHEN kind IN (3, 4) THEN data
                    WHEN kind IN (0, 1, 2) AND node_id IS NULL THEN data
                    ELSE NULL
                END AS data
            FROM events
            WHERE journey_id = $1
              AND sequence_id > COALESCE($2, -1)
            ORDER BY sequence_id
            "#,
        )
        .bind(journey_id)
        .bind(after_sequence_id)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let mut updates = Vec::with_capacity(rows.len());
        for row in rows {
            let sequence_id_i64 = row
                .try_get::<i64, _>("sequence_id")
                .map_err(crate::PersistenceError::PostgresQuery)?;
            let kind = row
                .try_get::<i16, _>("kind")
                .map_err(crate::PersistenceError::PostgresQuery)?;
            let event_unix_ms = row
                .try_get::<Option<i64>, _>("event_unix_ms")
                .map_err(crate::PersistenceError::PostgresQuery)?
                .unwrap_or(0);
            let node_id = row
                .try_get::<Option<i32>, _>("node_id")
                .map_err(crate::PersistenceError::PostgresQuery)?;
            let data = row
                .try_get::<Option<Vec<u8>>, _>("data")
                .map_err(crate::PersistenceError::PostgresQuery)?;

            let sequence_id = u64::try_from(sequence_id_i64).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "negative sequence_id in postgres events: {}",
                    sequence_id_i64
                ))
            })?;
            updates.push(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms,
                event: decode_journey_update_row(journey_id, kind, node_id, data)?,
            });
        }
        let fetch_elapsed_ms = fetch_started_at.elapsed().as_millis();
        let fetch_count = PG_UPDATES_FETCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        let now_ms = now_unix_ms();
        let mut max_event_age_ms = 0_i64;
        for update in updates.iter() {
            max_event_age_ms = max_event_age_ms.max(now_ms.saturating_sub(update.event_unix_ms));
        }
        if fetch_elapsed_ms > PG_SLOW_UPDATES_FETCH_WARN_THRESHOLD_MS {
            warn!(
                journey_id = %journey_id,
                fetch_count,
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "slow postgres journey_update_events_since query"
            );
        } else if max_event_age_ms > PG_STALE_EVENT_WARN_MS {
            warn!(
                journey_id = %journey_id,
                fetch_count,
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "postgres journey_update_events_since returned stale events"
            );
        } else if fetch_count % PG_UPDATES_LOG_INTERVAL == 0 {
            debug!(
                journey_id = %journey_id,
                fetch_count,
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "postgres journey_update_events_since heartbeat"
            );
        }
        Ok(updates)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        let status = sqlx::query_scalar!(
            r#"
            SELECT status
            FROM journeys
            WHERE id = $1
            "#,
            journey_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?
        .ok_or_else(|| {
            crate::PersistenceError::Message(format!("journey not found: {journey_id}"))
        })?;

        decode_journey_status(status)
    }

    async fn list_journeys(&self, namespace: String) -> Result<Vec<JourneyRecord>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, namespace, animal_id, generation, status, seed
            FROM journeys
            WHERE namespace = $1
            ORDER BY id
            "#,
            namespace
        )
        .fetch_all(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let mut journeys = Vec::with_capacity(rows.len());
        for row in rows {
            let animal_id = u32::try_from(row.animal_id).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "negative animal_id in postgres journeys: {}",
                    row.animal_id
                ))
            })?;
            let generation = u32::try_from(row.generation).map_err(|_| {
                crate::PersistenceError::Message(format!(
                    "negative generation in postgres journeys: {}",
                    row.generation
                ))
            })?;
            journeys.push(JourneyRecord {
                journey_id: row.id,
                namespace: row.namespace,
                animal_id,
                generation,
                status: decode_journey_status(row.status)?,
                seed: row.seed,
            });
        }

        Ok(journeys)
    }

    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>> {
        let appearance = sqlx::query_scalar!(
            r#"
            SELECT data
            FROM animal_appearances
            WHERE journey_id = $1
            "#,
            journey_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(appearance)
    }

    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO animal_appearances (journey_id, data, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (journey_id)
            DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()
            "#,
            journey_id,
            data
        )
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        sqlx::query!(
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
            journey_id,
            data,
            0_i16
        )
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(())
    }

    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>> {
        let row = sqlx::query!(
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
            journey_id,
            0_i16,
            1_i16
        )
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

        Ok(Some(ClaimedPerturbable { id, data: row.data }))
    }

    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()> {
        let sequence_id = i64::try_from(perturbation_id).map_err(|_| {
            crate::PersistenceError::Message(format!(
                "perturbation id exceeds i64 range for postgres: {perturbation_id}"
            ))
        })?;
        let result = sqlx::query!(
            r#"
            DELETE FROM animal_perturbations
            WHERE journey_id = $1 AND sequence_id = $2
            "#,
            journey_id,
            sequence_id
        )
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
        sqlx::query!(
            r#"
            INSERT INTO journey_leases (journey_id, owner_id, lease_until, heartbeat_at)
            VALUES ($1, $2, NOW() + ($3::BIGINT * INTERVAL '1 millisecond'), NOW())
            ON CONFLICT (journey_id)
            DO UPDATE SET owner_id = EXCLUDED.owner_id,
                          lease_until = EXCLUDED.lease_until,
                          heartbeat_at = EXCLUDED.heartbeat_at
            "#,
            journey_id,
            owner_id,
            lease_ttl_ms
        )
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;
        Ok(())
    }

    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>> {
        let wake = sqlx::query!(
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
            owner_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        Ok(wake.map(|row| OwnerWake {
            journey_id: row.journey_id,
            timer_id: row.timer_id,
        }))
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE journeys
            SET status = $2
            WHERE id = $1
            "#,
            journey_id,
            encode_journey_status(JourneyStatus::Completed)
        )
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

    async fn journey_dead(&self, journey_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE journeys
            SET status = $2
            WHERE id = $1
            "#,
            journey_id,
            encode_journey_status(JourneyStatus::Dead)
        )
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
        sqlx::query!(
            r#"
            UPDATE journeys
            SET status = $2
            WHERE id = $1 AND status = $3
            "#,
            journey_id,
            encode_journey_status(JourneyStatus::Alive),
            encode_journey_status(JourneyStatus::Created)
        )
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
            sqlx::query!(
                r#"
                INSERT INTO animal_generations (namespace, animal_id, generation, updated_at)
                VALUES ($1, $2, $3, NOW())
                ON CONFLICT (namespace, animal_id)
                DO UPDATE SET
                    generation = EXCLUDED.generation,
                    updated_at = NOW()
                WHERE animal_generations.generation < EXCLUDED.generation
                "#,
                namespace.as_str(),
                animal_id,
                generation
            )
            .execute(&self.pool)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        }

        let row = sqlx::query(
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
                  AND j.status IN ($6, $7)
                  AND (
                        wi.status = $4
                        OR (
                            wi.status = $5
                            AND wi.expiry < NOW()
                            AND NOT EXISTS (
                                SELECT 1
                                FROM journey_leases jl
                                WHERE jl.journey_id = wi.journey_id
                                  AND jl.lease_until > NOW()
                            )
                        )
                  )
                ORDER BY wi.expiry, wi.id
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            ),
            claimed AS (
                UPDATE work_items wi
                SET status = $5,
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
        .bind(&supported_ids)
        .bind(&supported_generations)
        .bind(0_i16)
        .bind(1_i16)
        .bind(encode_journey_status(JourneyStatus::Created))
        .bind(encode_journey_status(JourneyStatus::Alive))
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let kind = row
            .try_get::<i16, _>("kind")
            .map_err(crate::PersistenceError::PostgresQuery)?;
        let journey_id = row
            .try_get::<Uuid, _>("journey_id")
            .map_err(crate::PersistenceError::PostgresQuery)?;
        let animal_id_i32 = row
            .try_get::<i32, _>("animal_id")
            .map_err(crate::PersistenceError::PostgresQuery)?;
        let generation_i32 = row
            .try_get::<i32, _>("generation")
            .map_err(crate::PersistenceError::PostgresQuery)?;
        let seed = row
            .try_get::<Vec<u8>, _>("seed")
            .map_err(crate::PersistenceError::PostgresQuery)?;

        let work = match kind {
            0 => {
                let animal_id = u32::try_from(animal_id_i32).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative animal_id in postgres journey: {}",
                        animal_id_i32
                    ))
                })?;
                let generation = u32::try_from(generation_i32).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative generation in postgres journey: {}",
                        generation_i32
                    ))
                })?;
                Work::StartJourney {
                    journey_id,
                    animal_id,
                    generation,
                    seed,
                }
            }
            1 => {
                let animal_id = u32::try_from(animal_id_i32).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative animal_id in postgres journey: {}",
                        animal_id_i32
                    ))
                })?;
                let generation = u32::try_from(generation_i32).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative generation in postgres journey: {}",
                        generation_i32
                    ))
                })?;
                Work::ResumeJourney {
                    journey_id,
                    animal_id,
                    generation,
                    seed,
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

    async fn append_history(&self, history: RunnerOut, event_unix_ms: i64) -> Result<()> {
        let (journey_id, kind, node_id, data) = match history {
            RunnerOut::EffectInput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                0_i16,
                Some(i32::try_from(node_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "node_id exceeds i32 range for postgres: {node_id}"
                    ))
                })?),
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::EffectSuccessOutput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                1_i16,
                Some(i32::try_from(node_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "node_id exceeds i32 range for postgres: {node_id}"
                    ))
                })?),
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::EffectFailureOutput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                2_i16,
                Some(i32::try_from(node_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "node_id exceeds i32 range for postgres: {node_id}"
                    ))
                })?),
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::SleepScheduled {
                uuid,
                timer_id,
                wake_at_unix_ms,
            } => (
                uuid,
                3_i16,
                None,
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
                None,
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
            INSERT INTO events (journey_id, sequence_id, kind, node_id, data, event_unix_ms)
            SELECT $1, next_sequence.sequence_id, $2, $3, $4, $5
            FROM next_sequence
            "#,
        )
        .bind(journey_id)
        .bind(kind)
        .bind(node_id)
        .bind(data)
        .bind(event_unix_ms)
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        self.notify_journey_event(journey_id).await?;

        Ok(())
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO timer_tasks (id, journey_id, status, visible_at, fired_at)
            VALUES ($1, $2, $3, to_timestamp($4::BIGINT::double precision / 1000.0), NULL)
            ON CONFLICT (id) DO NOTHING
            "#,
            timer_id,
            journey_id,
            0_i16,
            wake_at_unix_ms
        )
        .execute(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        self.append_history(
            RunnerOut::SleepScheduled {
                uuid: journey_id,
                timer_id,
                wake_at_unix_ms,
            },
            chrono::Utc::now().timestamp_millis(),
        )
        .await?;

        Ok(())
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        let due = sqlx::query!(
            r#"
            SELECT id, journey_id
            FROM timer_tasks
            WHERE status = $1 AND visible_at <= NOW()
            ORDER BY visible_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#,
            0_i16
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let Some(due) = due else {
            tx.commit()
                .await
                .map_err(crate::PersistenceError::PostgresQuery)?;
            return Ok(None);
        };

        sqlx::query!(
            r#"
            UPDATE timer_tasks
            SET status = $2, fired_at = NOW()
            WHERE id = $1 AND status = $3
            "#,
            due.id,
            1_i16,
            0_i16
        )
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
            INSERT INTO events (journey_id, sequence_id, kind, node_id, data, event_unix_ms)
            SELECT $1, next_sequence.sequence_id, $2, $3, $4, $5
            FROM next_sequence
            "#,
        )
        .bind(due.journey_id)
        .bind(4_i16)
        .bind(Option::<i32>::None)
        .bind(sleep_fired_data)
        .bind(fired_at_unix_ms)
        .execute(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        let owner_id = sqlx::query_scalar!(
            r#"
            SELECT owner_id
            FROM journey_leases
            WHERE journey_id = $1 AND lease_until > NOW()
            LIMIT 1
            "#,
            due.journey_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;

        if let Some(owner_id) = owner_id {
            sqlx::query!(
                r#"
                INSERT INTO owner_wakes (id, owner_id, journey_id, timer_id, created_at)
                VALUES ($1, $2, $3, $4, NOW())
                "#,
                Uuid::new_v4(),
                owner_id,
                due.journey_id,
                due.id
            )
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        } else {
            sqlx::query!(
                r#"
                INSERT INTO work_items (id, journey_id, kind, status, expiry)
                VALUES ($1, $2, $3, $4, NOW())
                "#,
                Uuid::new_v4(),
                due.journey_id,
                1_i16,
                0_i16
            )
            .execute(&mut *tx)
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;
        }

        tx.commit()
            .await
            .map_err(crate::PersistenceError::PostgresQuery)?;

        self.notify_journey_event(due.journey_id).await?;

        Ok(Some(()))
    }

    async fn next_timer_due_at(&self) -> Result<Option<i64>> {
        let due = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT FLOOR(EXTRACT(EPOCH FROM visible_at) * 1000)::BIGINT
            FROM timer_tasks
            WHERE status = $1
            ORDER BY visible_at, id
            LIMIT 1
            "#,
        )
        .bind(0_i16)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::PersistenceError::PostgresQuery)?;
        Ok(due)
    }

    fn postgres_pool(&self) -> Option<sqlx::PgPool> {
        Some(self.pool.clone())
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
struct EffectEventData {
    node_id: u32,
    data: Vec<u8>,
}

fn encode_effect_event(node_id: u32, data: Vec<u8>) -> Result<Vec<u8>> {
    let mut payload = Vec::with_capacity(1 + data.len() + 8);
    payload.push(ACTION_EVENT_ENVELOPE_V1);
    let encoded = postcard::to_allocvec(&EffectEventData { node_id, data })
        .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
    payload.extend_from_slice(&encoded);
    Ok(payload)
}

fn decode_effect_event(data: Vec<u8>) -> Result<(u32, Vec<u8>)> {
    if data.first().copied() != Some(ACTION_EVENT_ENVELOPE_V1) {
        return Ok((0, data));
    }
    let envelope: EffectEventData = postcard::from_bytes(&data[1..])
        .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
    Ok((envelope.node_id, envelope.data))
}

fn decode_history_row(journey_id: Uuid, kind: i16, data: Vec<u8>) -> Result<RunnerOut> {
    match kind {
        0 => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectInput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        1 => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectSuccessOutput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        2 => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectFailureOutput {
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

fn decode_journey_update_row(
    journey_id: Uuid,
    kind: i16,
    node_id: Option<i32>,
    data: Option<Vec<u8>>,
) -> Result<RunnerUpdateOut> {
    match kind {
        0..=2 => {
            let node_id = if let Some(node_id) = node_id {
                u32::try_from(node_id).map_err(|_| {
                    crate::PersistenceError::Message(format!(
                        "invalid negative node_id in postgres events: {node_id}"
                    ))
                })?
            } else {
                let fallback_data = data.ok_or_else(|| {
                    crate::PersistenceError::Message(
                        "missing effect event data and node_id in postgres events".to_string(),
                    )
                })?;
                decode_effect_event(fallback_data)?.0
            };

            match kind {
                0 => Ok(RunnerUpdateOut::EffectInput {
                    node_id,
                    uuid: journey_id,
                }),
                1 => Ok(RunnerUpdateOut::EffectSuccessOutput {
                    node_id,
                    uuid: journey_id,
                }),
                2 => Ok(RunnerUpdateOut::EffectFailureOutput {
                    node_id,
                    uuid: journey_id,
                }),
                _ => unreachable!(),
            }
        }
        3 => {
            let sleep_data = data.ok_or_else(|| {
                crate::PersistenceError::Message(
                    "missing sleep-scheduled payload in postgres events".to_string(),
                )
            })?;
            let event: SleepScheduledEvent = postcard::from_bytes(&sleep_data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerUpdateOut::SleepScheduled {
                uuid: journey_id,
                timer_id: event.timer_id,
                wake_at_unix_ms: event.wake_at_unix_ms,
            })
        }
        4 => {
            let sleep_data = data.ok_or_else(|| {
                crate::PersistenceError::Message(
                    "missing sleep-fired payload in postgres events".to_string(),
                )
            })?;
            let event: SleepFiredEvent = postcard::from_bytes(&sleep_data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerUpdateOut::SleepFired {
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

fn now_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
