use crate::models::{SchemaVersion, StepKind, StepStatus, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jungle_types::{
    ClaimedAnimalPerturbation, JourneyStatus, JourneyUpdateEvent, OwnerWake, RunnerOut,
    RunnerUpdateOut, SupportedAnimal, Work,
};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const JOURNEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journeys");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const STEPS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");
const TIMER_TASKS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("timer_tasks");
const TIMER_DUE_INDEX_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("timer_due_index");
const JOURNEY_LEASES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journey_leases");
const OWNER_WAKES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("owner_wakes");
const APPEARANCES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("animal_appearances");
const PERTURBATIONS_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("animal_perturbations");
const ANIMAL_GENERATIONS_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("animal_generations");
const JOURNEY_EVENT_SEQUENCE_TABLE: TableDefinition<&[u8], u64> =
    TableDefinition::new("journey_event_sequences");

const STEP_KIND_START_JOURNEY: u8 = 0;
const STEP_KIND_RESUME_JOURNEY: u8 = 1;
const STEP_STATUS_AVAILABLE: u8 = 0;
const STEP_STATUS_CLAIMED: u8 = 1;
const TIMER_STATUS_PENDING: u8 = 0;
const TIMER_STATUS_FIRED: u8 = 1;
const JOURNEY_STATUS_CREATED: u8 = 0;
const JOURNEY_STATUS_ALIVE: u8 = 1;
const JOURNEY_STATUS_STOPPED: u8 = 2;
const JOURNEY_STATUS_COMPLETED: u8 = 3;
const JOURNEY_STATUS_DEAD: u8 = 4;

const EVENT_KIND_ACTION_INPUT: u8 = 0;
const EVENT_KIND_ACTION_SUCCESS_OUTPUT: u8 = 1;
const EVENT_KIND_ACTION_FAILURE_OUTPUT: u8 = 2;
const EVENT_KIND_SLEEP_SCHEDULED: u8 = 3;
const EVENT_KIND_SLEEP_FIRED: u8 = 4;

#[derive(Debug, Clone)]
pub struct RedbStore {
    db: Arc<redb::Database>,
}

impl RedbStore {
    pub fn builder() -> RedbStoreBuilder {
        RedbStoreBuilder::default()
    }

    pub fn in_memory() -> Result<RedbStore> {
        let db = redb::Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .map_err(crate::PersistenceError::RedbOpen)?;
        Ok(RedbStore { db: Arc::new(db) })
    }

    fn update_journey_status(
        &self,
        journey_id: Uuid,
        new_status: JourneyStatus,
        expected_current: Option<JourneyStatus>,
    ) -> Result<()> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb update_journey_status begin failed: {err}"
            ))
        })?;

        {
            let mut journeys = write_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb update_journey_status open journeys table failed: {err}"
                ))
            })?;
            let key = &journey_id.as_bytes()[..];
            let existing_raw = {
                let Some(existing) = journeys.get(key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb update_journey_status read journey failed: {err}"
                    ))
                })?
                else {
                    return Err(crate::PersistenceError::Message(format!(
                        "journey not found: {journey_id}"
                    )));
                };
                existing.value().to_vec()
            };

            let flow = decode_journey(
                existing_raw.as_slice(),
                "redb update_journey_status decode journey value",
            )?;
            if expected_current.is_none_or(|expected| flow.status == expected) {
                let updated_value = encode_journey(
                    flow.namespace.as_str(),
                    flow.animal_id,
                    flow.generation,
                    new_status,
                    &flow.seed,
                );
                journeys
                    .insert(key, updated_value.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb update_journey_status write journey failed: {err}"
                        ))
                    })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb update_journey_status commit failed: {err}"
            ))
        })?;
        Ok(())
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
            SchemaVersion::V0 => {
                jungle_migrate::migrate_redb_v0(&self.db).map_err(crate::PersistenceError::Message)
            }
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
        let expiry = Utc::now();

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb create_journey begin failed: {err}"))
        })?;

        {
            let generations = write_tx
                .open_table(ANIMAL_GENERATIONS_TABLE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey open animal_generations table failed: {err}"
                    ))
                })?;
            let generation_key = encode_animal_generation_key(namespace.as_str(), animal_id);
            let latest_generation = generations
                .get(generation_key.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey read animal generation failed: {err}"
                    ))
                })?
                .map(|value| {
                    decode_generation(value.value(), "redb create_journey decode generation")
                })
                .transpose()?
                .unwrap_or(0);

            if generation > latest_generation {
                return Err(crate::PersistenceError::Message(format!(
                    "client generation {generation} exceeds latest server generation {latest_generation} for namespace {namespace} animal {animal_id}"
                )));
            }

            let mut journeys = write_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb create_journey open journeys table failed: {err}"
                ))
            })?;
            let flow_value = encode_journey(
                namespace.as_str(),
                animal_id,
                latest_generation,
                JourneyStatus::Created,
                &seed,
            );
            journeys
                .insert(&journey_id.as_bytes()[..], flow_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey insert journey failed: {err}"
                    ))
                })?;

            let mut sequences =
                write_tx
                    .open_table(JOURNEY_EVENT_SEQUENCE_TABLE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb create_journey open journey_event_sequences table failed: {err}"
                        ))
                    })?;
            sequences
                .insert(&journey_id.as_bytes()[..], 0_u64)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey initialize journey event sequence failed: {err}"
                    ))
                })?;
        }

        {
            let mut work_items = write_tx.open_table(STEPS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb create_journey open work_items table failed: {err}"
                ))
            })?;

            let work_item_value = encode_work_item(
                journey_id,
                StepKind::StartJourney,
                StepStatus::Available,
                expiry,
            );

            work_items
                .insert(&work_item_id.as_bytes()[..], work_item_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey insert work item failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb create_journey commit failed: {err}"))
        })?;

        Ok(journey_id)
    }

    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>> {
        let read_tx = self.db.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_history begin read failed: {err}"
            ))
        })?;
        let events = read_tx.open_table(EVENTS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_history open events table failed: {err}"
            ))
        })?;
        let start_key = encode_event_key(journey_id, 0);
        let end_key = encode_event_key(journey_id, u64::MAX);
        let iter = events
            .range(start_key.as_slice()..=end_key.as_slice())
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb journey_history range events failed: {err}"
                ))
            })?;

        let mut rows: Vec<(u64, u8, Vec<u8>)> = Vec::new();
        for entry in iter {
            let (key, value) = entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb journey_history read events entry failed: {err}"
                ))
            })?;
            let (_, sequence_id) =
                decode_event_key(key.value(), "redb journey_history decode event key")?;
            let (kind, data) =
                decode_event_value(value.value(), "redb journey_history decode event value")?;
            rows.push((sequence_id, kind, data));
        }

        let mut history = Vec::with_capacity(rows.len());
        for (_, kind, data) in rows {
            history.push(decode_runner_out(journey_id, kind, data)?);
        }
        Ok(history)
    }

    async fn journey_update_events_since(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<Vec<JourneyUpdateEvent>> {
        if after_sequence_id == Some(u64::MAX) {
            return Ok(Vec::new());
        }

        let read_tx = self.db.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_events_since begin read failed: {err}"
            ))
        })?;
        let events = read_tx.open_table(EVENTS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_events_since open events table failed: {err}"
            ))
        })?;
        let start_sequence_id = after_sequence_id.map_or(0_u64, |after| after + 1);
        let start_key = encode_event_key(journey_id, start_sequence_id);
        let end_key = encode_event_key(journey_id, u64::MAX);
        let iter = events
            .range(start_key.as_slice()..=end_key.as_slice())
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb journey_events_since range events failed: {err}"
                ))
            })?;

        let mut rows: Vec<(u64, u8, Vec<u8>)> = Vec::new();
        for entry in iter {
            let (key, value) = entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb journey_events_since read events entry failed: {err}"
                ))
            })?;
            let (_, sequence_id) =
                decode_event_key(key.value(), "redb journey_events_since decode event key")?;
            let (kind, data) = decode_event_value(
                value.value(),
                "redb journey_events_since decode event value",
            )?;
            rows.push((sequence_id, kind, data));
        }

        let mut updates = Vec::with_capacity(rows.len());
        for (sequence_id, kind, data) in rows {
            updates.push(JourneyUpdateEvent {
                sequence_id,
                event: decode_runner_update_out(journey_id, kind, data)?,
            });
        }
        Ok(updates)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        let read_tx = self.db.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_status begin read failed: {err}"
            ))
        })?;

        let journeys = read_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb journey_status open journeys table failed: {err}"
            ))
        })?;

        let flow_value = journeys
            .get(&journey_id.as_bytes()[..])
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb journey_status read journey failed: {err}"
                ))
            })?
            .ok_or_else(|| {
                crate::PersistenceError::Message(format!("journey not found: {journey_id}"))
            })?;

        let flow = decode_journey(
            flow_value.value(),
            "redb journey_status decode journey value",
        )?;
        Ok(flow.status)
    }

    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>> {
        let read_tx = self.db.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb animal_appearance begin read failed: {err}"
            ))
        })?;

        let appearances = read_tx.open_table(APPEARANCES_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb animal_appearance open animal_appearances table failed: {err}"
            ))
        })?;

        let key = &journey_id.as_bytes()[..];
        let value = appearances.get(key).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb animal_appearance read appearance failed: {err}"
            ))
        })?;

        Ok(value.map(|entry| entry.value().to_vec()))
    }

    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb upsert_animal_appearance begin failed: {err}"
            ))
        })?;

        {
            let mut appearances = write_tx.open_table(APPEARANCES_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb upsert_animal_appearance open animal_appearances table failed: {err}"
                ))
            })?;
            appearances
                .insert(&journey_id.as_bytes()[..], data.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb upsert_animal_appearance write failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb upsert_animal_appearance commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb enqueue_animal_perturbation begin failed: {err}"
            ))
        })?;

        {
            let mut perturbations = write_tx.open_table(PERTURBATIONS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb enqueue_animal_perturbation open animal_perturbations table failed: {err}"
                ))
            })?;

            let mut max_sequence: Option<u64> = None;
            let iter = perturbations.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb enqueue_animal_perturbation iterate table failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, _) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb enqueue_animal_perturbation read entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "redb enqueue_animal_perturbation decode key")?;
                if entry_journey_id == journey_id {
                    max_sequence =
                        Some(max_sequence.map_or(sequence_id, |max| max.max(sequence_id)));
                }
            }

            let sequence_id = max_sequence.map_or(0_u64, |max| max.saturating_add(1));
            let key = encode_event_key(journey_id, sequence_id);
            let value = encode_perturbation_value(0_i64, &data);
            perturbations
                .insert(key.as_slice(), value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb enqueue_animal_perturbation insert failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb enqueue_animal_perturbation commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb claim_animal_perturbation begin failed: {err}"
            ))
        })?;

        let now = Utc::now().timestamp_millis();
        let lease_until = now.saturating_add(30_000);
        let mut selected: Option<(u64, Vec<u8>)> = None;

        {
            let mut perturbations = write_tx.open_table(PERTURBATIONS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_animal_perturbation open animal_perturbations table failed: {err}"
                ))
            })?;
            let iter = perturbations.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_animal_perturbation iterate table failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_animal_perturbation read entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "redb claim_animal_perturbation decode key")?;
                if entry_journey_id != journey_id {
                    continue;
                }
                let (entry_lease_until, payload) = decode_perturbation_value(
                    value.value(),
                    "redb claim_animal_perturbation decode value",
                )?;
                if entry_lease_until != 0 && entry_lease_until >= now {
                    continue;
                }
                let replace = selected
                    .as_ref()
                    .map(|(best, _)| sequence_id < *best)
                    .unwrap_or(true);
                if replace {
                    selected = Some((sequence_id, payload));
                }
            }

            if let Some((sequence_id, payload)) = selected.as_ref() {
                let key = encode_event_key(journey_id, *sequence_id);
                let value = encode_perturbation_value(lease_until, payload);
                perturbations
                    .insert(key.as_slice(), value.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_animal_perturbation write claim failed: {err}"
                        ))
                    })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb claim_animal_perturbation commit failed: {err}"
            ))
        })?;

        let Some((sequence_id, data)) = selected else {
            return Ok(None);
        };
        Ok(Some(ClaimedAnimalPerturbation {
            id: sequence_id,
            data,
        }))
    }

    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb ack_animal_perturbation begin failed: {err}"
            ))
        })?;
        {
            let mut perturbations = write_tx.open_table(PERTURBATIONS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb ack_animal_perturbation open animal_perturbations table failed: {err}"
                ))
            })?;
            let key = encode_event_key(journey_id, perturbation_id);
            let removed = perturbations.remove(key.as_slice()).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb ack_animal_perturbation remove failed: {err}"
                ))
            })?;
            if removed.is_none() {
                return Err(crate::PersistenceError::Message(format!(
                    "animal perturbation not found for ack: {journey_id}:{perturbation_id}"
                )));
            }
        }
        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb ack_animal_perturbation commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<()> {
        let now_millis = Utc::now().timestamp_millis();
        let lease_ttl_ms = lease_ttl_ms.max(0);
        let lease_until_millis = now_millis.saturating_add(lease_ttl_ms);

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb heartbeat_journey_lease begin failed: {err}"
            ))
        })?;
        {
            let mut leases = write_tx.open_table(JOURNEY_LEASES_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb heartbeat_journey_lease open journey_leases table failed: {err}"
                ))
            })?;
            let key = &journey_id.as_bytes()[..];
            let value = encode_journey_lease(owner_id, lease_until_millis, now_millis);
            leases.insert(key, value.as_slice()).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb heartbeat_journey_lease write lease failed: {err}"
                ))
            })?;
        }
        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb heartbeat_journey_lease commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_owner_wake begin failed: {err}"))
        })?;

        let mut selected_key: Option<Vec<u8>> = None;
        let mut selected_value: Option<Vec<u8>> = None;

        {
            let mut owner_wakes = write_tx.open_table(OWNER_WAKES_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_owner_wake open owner_wakes table failed: {err}"
                ))
            })?;

            let iter = owner_wakes.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_owner_wake iterate owner_wakes failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_owner_wake read owner_wakes entry failed: {err}"
                    ))
                })?;
                let (entry_owner_id, _, _) =
                    decode_owner_wake_key(key.value(), "redb claim_owner_wake decode key")?;
                if entry_owner_id == owner_id {
                    selected_key = Some(key.value().to_vec());
                    selected_value = Some(value.value().to_vec());
                    break;
                }
            }

            if let Some(key) = selected_key.as_ref() {
                owner_wakes.remove(key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_owner_wake remove wake failed: {err}"
                    ))
                })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_owner_wake commit failed: {err}"))
        })?;

        let Some(value) = selected_value else {
            return Ok(None);
        };
        let wake = decode_owner_wake_value(value.as_slice(), "redb claim_owner_wake decode value")?;
        Ok(Some(wake))
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(journey_id, JourneyStatus::Completed, None)
    }

    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(
            journey_id,
            JourneyStatus::Alive,
            Some(JourneyStatus::Created),
        )
    }

    async fn claim_work(
        &self,
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>> {
        if supported_animals.is_empty() {
            return Ok(None);
        }

        let supported_set: HashSet<(u32, u32)> = supported_animals
            .iter()
            .map(|animal| (animal.animal_id, animal.generation))
            .collect();

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_work begin failed: {err}"))
        })?;
        let now = Utc::now();
        let lease_until = now + chrono::Duration::seconds(30);

        let mut selected: Option<(Uuid, Uuid, StepKind, DateTime<Utc>)> = None;

        {
            let mut generation_table =
                write_tx
                    .open_table(ANIMAL_GENERATIONS_TABLE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_work open animal_generations table failed: {err}"
                        ))
                    })?;
            for supported in supported_animals {
                let key = encode_animal_generation_key(namespace.as_str(), supported.animal_id);
                let existing = generation_table
                    .get(key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_work read animal generation failed: {err}"
                        ))
                    })?
                    .map(|value| {
                        decode_generation(value.value(), "redb claim_work decode generation")
                    })
                    .transpose()?
                    .unwrap_or(0);
                if supported.generation > existing {
                    generation_table
                        .insert(
                            key.as_slice(),
                            supported.generation.to_be_bytes().as_slice(),
                        )
                        .map_err(|err| {
                            crate::PersistenceError::Message(format!(
                                "redb claim_work write animal generation failed: {err}"
                            ))
                        })?;
                }
            }

            let journeys = write_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_work open journeys table failed: {err}"
                ))
            })?;
            let mut work_items = write_tx.open_table(STEPS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_work open work_items table failed: {err}"
                ))
            })?;

            let iter = work_items.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_work iterate work_items failed: {err}"
                ))
            })?;

            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_work read work_items entry failed: {err}"
                    ))
                })?;
                let id = decode_uuid(key.value(), "redb claim_work decode work_item id")?;
                let (journey_id, kind, status, expiry) =
                    decode_work_item(value.value(), "redb claim_work decode work_item value")?;

                let claimable = match status {
                    StepStatus::Available => true,
                    StepStatus::Claimed => expiry <= now,
                };
                if !claimable {
                    continue;
                }

                let journey_raw = journeys
                    .get(&journey_id.as_bytes()[..])
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_work read journey for namespace filter failed: {err}"
                        ))
                    })?
                    .ok_or_else(|| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_work missing journey for work item {id}"
                        ))
                    })?;
                let journey = decode_journey(
                    journey_raw.value(),
                    "redb claim_work decode journey for namespace filter",
                )?;
                if journey.namespace != namespace.as_str() {
                    continue;
                }
                if !supported_set.contains(&(journey.animal_id, journey.generation)) {
                    continue;
                }

                let replace = selected
                    .as_ref()
                    .map(|(selected_id, _, _, selected_expiry)| {
                        expiry < *selected_expiry
                            || (expiry == *selected_expiry && id < *selected_id)
                    })
                    .unwrap_or(true);

                if replace {
                    selected = Some((id, journey_id, kind, expiry));
                }
            }

            if let Some((selected_id, selected_journey_id, selected_kind, _selected_expiry)) =
                selected
            {
                let claimed = encode_work_item(
                    selected_journey_id,
                    selected_kind,
                    StepStatus::Claimed,
                    lease_until,
                );
                let work_item_id_key = &selected_id.as_bytes()[..];
                work_items
                    .insert(work_item_id_key, claimed.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb claim_work update work_items status failed: {err}"
                        ))
                    })?;
            }
        }

        let Some((selected_id, selected_journey_id, selected_kind, _)) = selected else {
            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!("redb claim_work commit failed: {err}"))
            })?;
            return Ok(None);
        };

        let flow = {
            let journeys = write_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_work open journeys table failed: {err}"
                ))
            })?;
            let flow_key = &selected_journey_id.as_bytes()[..];
            let flow_value = journeys
                .get(flow_key)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_work read journey failed: {err}"
                    ))
                })?
                .ok_or_else(|| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_work missing journey for work item {selected_id}"
                    ))
                })?;
            decode_journey(flow_value.value(), "redb claim_work decode journey value")?
        };

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_work commit failed: {err}"))
        })?;

        let work = match selected_kind {
            StepKind::StartJourney => Work::StartJourney {
                journey_id: selected_journey_id,
                animal_id: flow.animal_id,
                generation: flow.generation,
                seed: flow.seed,
            },
            StepKind::ResumeJourney => Work::ResumeJourney {
                journey_id: selected_journey_id,
                animal_id: flow.animal_id,
                generation: flow.generation,
                seed: flow.seed,
            },
        };

        Ok(Some(work))
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        let (journey_id, kind, data) = match history {
            RunnerOut::EffectInput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                EVENT_KIND_ACTION_INPUT,
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::EffectSuccessOutput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                EVENT_KIND_ACTION_SUCCESS_OUTPUT,
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::EffectFailureOutput {
                node_id,
                data,
                uuid,
            } => (
                uuid,
                EVENT_KIND_ACTION_FAILURE_OUTPUT,
                encode_effect_event(node_id, data)?,
            ),
            RunnerOut::SleepScheduled {
                uuid,
                timer_id,
                wake_at_unix_ms,
            } => (
                uuid,
                EVENT_KIND_SLEEP_SCHEDULED,
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
                EVENT_KIND_SLEEP_FIRED,
                postcard::to_allocvec(&SleepFiredEvent {
                    timer_id,
                    fired_at_unix_ms,
                })
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?,
            ),
            RunnerOut::Appearance { .. } => {
                return Err(crate::PersistenceError::Message(
                    "appearance snapshots are not history events in redb".to_string(),
                ))
            }
        };

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb append_history begin failed: {err}"))
        })?;

        {
            let mut events = write_tx.open_table(EVENTS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb append_history open events table failed: {err}"
                ))
            })?;
            let mut sequences =
                write_tx
                    .open_table(JOURNEY_EVENT_SEQUENCE_TABLE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb append_history open journey_event_sequences table failed: {err}"
                        ))
                    })?;

            let key = &journey_id.as_bytes()[..];
            let sequence_id = if let Some(next) = sequences.get(key).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb append_history read journey_event_sequences failed: {err}"
                ))
            })? {
                next.value()
            } else {
                let start_key = encode_event_key(journey_id, 0);
                let end_key = encode_event_key(journey_id, u64::MAX);
                let iter = events
                    .range(start_key.as_slice()..=end_key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb append_history range events failed: {err}"
                        ))
                    })?;
                let mut next_sequence = 0_u64;
                for entry in iter {
                    let (event_key, _) = entry.map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb append_history read events entry failed: {err}"
                        ))
                    })?;
                    let (_, existing_sequence_id) = decode_event_key(
                        event_key.value(),
                        "redb append_history decode event key",
                    )?;
                    next_sequence = next_sequence.max(existing_sequence_id.saturating_add(1));
                }
                next_sequence
            };

            let event_key = encode_event_key(journey_id, sequence_id);
            let event_value = encode_event_value(kind, &data);
            events
                .insert(event_key.as_slice(), event_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb append_history insert event failed: {err}"
                    ))
                })?;
            sequences
                .insert(key, sequence_id.saturating_add(1))
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb append_history write journey_event_sequences failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb append_history commit failed: {err}"))
        })?;

        Ok(())
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<()> {
        let wake_at = DateTime::from_timestamp_millis(wake_at_unix_ms).ok_or_else(|| {
            crate::PersistenceError::Message(format!(
                "invalid timestamp millis for wake_at: {wake_at_unix_ms}"
            ))
        })?;

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb schedule_sleep_timer begin failed: {err}"
            ))
        })?;

        {
            let mut timers = write_tx.open_table(TIMER_TASKS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb schedule_sleep_timer open timer_tasks table failed: {err}"
                ))
            })?;
            let mut due_index = write_tx.open_table(TIMER_DUE_INDEX_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb schedule_sleep_timer open timer_due_index table failed: {err}"
                ))
            })?;
            let timer_key = &timer_id.as_bytes()[..];
            if let Some(existing) = timers.get(timer_key).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb schedule_sleep_timer read existing timer task failed: {err}"
                ))
            })? {
                let existing_timer = decode_timer_task(
                    existing.value(),
                    "redb schedule_sleep_timer decode existing timer task",
                )?;
                let stale_due_key = encode_timer_due_index_key(
                    existing_timer.visible_at.timestamp_millis(),
                    timer_id,
                );
                let _ = due_index.remove(stale_due_key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb schedule_sleep_timer remove stale timer_due_index entry failed: {err}"
                    ))
                })?;
            }
            let timer_value = encode_timer_task(journey_id, TIMER_STATUS_PENDING, wake_at, 0);
            timers
                .insert(timer_key, timer_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb schedule_sleep_timer insert timer task failed: {err}"
                    ))
                })?;
            let due_key = encode_timer_due_index_key(wake_at.timestamp_millis(), timer_id);
            due_index
                .insert(due_key.as_slice(), &[] as &[u8])
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb schedule_sleep_timer insert timer_due_index entry failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb schedule_sleep_timer commit failed: {err}"
            ))
        })?;

        self.append_history(RunnerOut::SleepScheduled {
            uuid: journey_id,
            timer_id,
            wake_at_unix_ms,
        })
        .await
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let now = Utc::now();
        let now_millis = now.timestamp_millis();

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb poll_timers begin failed: {err}"))
        })?;

        let mut selected: Option<(Uuid, Uuid, DateTime<Utc>)> = None;
        {
            let mut timers = write_tx.open_table(TIMER_TASKS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open timer_tasks table failed: {err}"
                ))
            })?;
            let mut due_index = write_tx.open_table(TIMER_DUE_INDEX_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open timer_due_index table failed: {err}"
                ))
            })?;
            let due_start = encode_timer_due_index_key(i64::MIN, Uuid::nil());
            let due_end = encode_timer_due_index_bound_key(now_millis, true);
            let due_iter = due_index
                .range(due_start.as_slice()..=due_end.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers range timer_due_index failed: {err}"
                    ))
                })?;
            let mut stale_due_keys: Vec<Vec<u8>> = Vec::new();
            let mut selected_due_key: Option<Vec<u8>> = None;
            for due_entry in due_iter {
                let (due_key, _) = due_entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers read timer_due_index entry failed: {err}"
                    ))
                })?;
                let (indexed_visible_at_unix_ms, timer_id) = decode_timer_due_index_key(
                    due_key.value(),
                    "redb poll_timers decode timer_due_index key",
                )?;

                let timer_key = &timer_id.as_bytes()[..];
                let Some(timer_value) = timers.get(timer_key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers read timer task by due index failed: {err}"
                    ))
                })?
                else {
                    stale_due_keys.push(due_key.value().to_vec());
                    continue;
                };

                let timer = decode_timer_task(
                    timer_value.value(),
                    "redb poll_timers decode timer task by due index",
                )?;
                let timer_visible_at_unix_ms = timer.visible_at.timestamp_millis();
                if timer.status != TIMER_STATUS_PENDING
                    || timer_visible_at_unix_ms != indexed_visible_at_unix_ms
                    || timer_visible_at_unix_ms > now_millis
                {
                    stale_due_keys.push(due_key.value().to_vec());
                    continue;
                }

                selected = Some((timer_id, timer.journey_id, timer.visible_at));
                selected_due_key = Some(due_key.value().to_vec());
                break;
            }

            for stale_due_key in stale_due_keys {
                let _ = due_index.remove(stale_due_key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers remove stale timer_due_index entry failed: {err}"
                    ))
                })?;
            }

            if let Some((timer_id, journey_id, visible_at)) = selected {
                let fired = encode_timer_task(journey_id, TIMER_STATUS_FIRED, now, now_millis);
                timers
                    .insert(&timer_id.as_bytes()[..], fired.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb poll_timers mark timer task fired failed: {err}"
                        ))
                    })?;
                let due_key = selected_due_key.unwrap_or_else(|| {
                    encode_timer_due_index_key(visible_at.timestamp_millis(), timer_id).to_vec()
                });
                let _ = due_index.remove(due_key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers remove fired timer_due_index entry failed: {err}"
                    ))
                })?;
            }
        }

        let Some((timer_id, journey_id, _)) = selected else {
            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!("redb poll_timers commit failed: {err}"))
            })?;
            return Ok(None);
        };

        let mut valid_owner: Option<Uuid> = None;
        {
            let leases = write_tx.open_table(JOURNEY_LEASES_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open journey_leases table failed: {err}"
                ))
            })?;
            let lease_entry = leases.get(&journey_id.as_bytes()[..]).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers read journey lease failed: {err}"
                ))
            })?;
            if let Some(raw) = lease_entry {
                let lease =
                    decode_journey_lease(raw.value(), "redb poll_timers decode journey lease")?;
                if lease.lease_until_unix_ms > now_millis {
                    valid_owner = Some(lease.owner_id);
                }
            }
        }

        if let Some(owner_id) = valid_owner {
            let mut owner_wakes = write_tx.open_table(OWNER_WAKES_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open owner_wakes table failed: {err}"
                ))
            })?;
            let wake_id = Uuid::new_v4();
            let key = encode_owner_wake_key(owner_id, now_millis, wake_id);
            let value = encode_owner_wake_value(journey_id, timer_id);
            owner_wakes
                .insert(key.as_slice(), value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers enqueue owner wake failed: {err}"
                    ))
                })?;
        } else {
            let mut work_items = write_tx.open_table(STEPS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open work_items table failed: {err}"
                ))
            })?;
            let work_item_id = Uuid::new_v4();
            let value = encode_work_item(
                journey_id,
                StepKind::ResumeJourney,
                StepStatus::Available,
                now,
            );
            work_items
                .insert(&work_item_id.as_bytes()[..], value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers enqueue resume work item failed: {err}"
                    ))
                })?;
        }

        {
            let mut events = write_tx.open_table(EVENTS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers open events table failed: {err}"
                ))
            })?;
            let mut sequences =
                write_tx
                    .open_table(JOURNEY_EVENT_SEQUENCE_TABLE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb poll_timers open journey_event_sequences table failed: {err}"
                        ))
                    })?;
            let key = &journey_id.as_bytes()[..];
            let sequence_id = if let Some(next) = sequences.get(key).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers read journey_event_sequences failed: {err}"
                ))
            })? {
                next.value()
            } else {
                let start_key = encode_event_key(journey_id, 0);
                let end_key = encode_event_key(journey_id, u64::MAX);
                let iter = events
                    .range(start_key.as_slice()..=end_key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb poll_timers range events failed: {err}"
                        ))
                    })?;
                let mut next_sequence = 0_u64;
                for entry in iter {
                    let (event_key, _) = entry.map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb poll_timers read events entry failed: {err}"
                        ))
                    })?;
                    let (_, existing_sequence_id) =
                        decode_event_key(event_key.value(), "redb poll_timers decode event key")?;
                    next_sequence = next_sequence.max(existing_sequence_id.saturating_add(1));
                }
                next_sequence
            };

            let event_key = encode_event_key(journey_id, sequence_id);
            let payload = postcard::to_allocvec(&SleepFiredEvent {
                timer_id,
                fired_at_unix_ms: now_millis,
            })
            .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            let event_value = encode_event_value(EVENT_KIND_SLEEP_FIRED, &payload);
            events
                .insert(event_key.as_slice(), event_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers insert sleep fired event failed: {err}"
                    ))
                })?;
            sequences
                .insert(key, sequence_id.saturating_add(1))
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers write journey_event_sequences failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb poll_timers commit failed: {err}"))
        })?;

        Ok(Some(()))
    }
}

#[derive(Debug)]
struct JourneyRow {
    namespace: String,
    animal_id: u32,
    generation: u32,
    status: JourneyStatus,
    seed: Vec<u8>,
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

#[derive(Debug)]
struct TimerTaskRow {
    journey_id: Uuid,
    status: u8,
    visible_at: DateTime<Utc>,
}

#[derive(Debug)]
struct JourneyLeaseRow {
    owner_id: Uuid,
    lease_until_unix_ms: i64,
}

fn decode_uuid(raw: &[u8], context: &str) -> Result<Uuid> {
    if raw.len() != 16 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected 16-byte uuid key, got {}",
            raw.len()
        )));
    }

    let mut id_bytes = [0_u8; 16];
    id_bytes.copy_from_slice(raw);
    Ok(Uuid::from_bytes(id_bytes))
}

fn encode_work_item(
    journey_id: Uuid,
    kind: StepKind,
    status: StepStatus,
    expiry: DateTime<Utc>,
) -> Vec<u8> {
    let kind = match kind {
        StepKind::StartJourney => STEP_KIND_START_JOURNEY,
        StepKind::ResumeJourney => STEP_KIND_RESUME_JOURNEY,
    };
    let status = match status {
        StepStatus::Available => STEP_STATUS_AVAILABLE,
        StepStatus::Claimed => STEP_STATUS_CLAIMED,
    };

    let mut out = Vec::with_capacity(26);
    out.extend_from_slice(journey_id.as_bytes());
    out.push(kind);
    out.push(status);
    out.extend_from_slice(&expiry.timestamp_millis().to_be_bytes());
    out
}

fn decode_work_item(
    raw: &[u8],
    context: &str,
) -> Result<(Uuid, StepKind, StepStatus, DateTime<Utc>)> {
    if raw.len() < 26 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 26 bytes, got {}",
            raw.len()
        )));
    }

    let journey_id = decode_uuid(&raw[..16], context)?;
    let kind = match raw[16] {
        STEP_KIND_START_JOURNEY => StepKind::StartJourney,
        STEP_KIND_RESUME_JOURNEY => StepKind::ResumeJourney,
        other => {
            return Err(crate::PersistenceError::Message(format!(
                "{context}: unknown work item kind {other}"
            )))
        }
    };
    let status = match raw[17] {
        STEP_STATUS_AVAILABLE => StepStatus::Available,
        STEP_STATUS_CLAIMED => StepStatus::Claimed,
        other => {
            return Err(crate::PersistenceError::Message(format!(
                "{context}: unknown work item status {other}"
            )))
        }
    };

    let mut millis_bytes = [0_u8; 8];
    millis_bytes.copy_from_slice(&raw[18..26]);
    let millis = i64::from_be_bytes(millis_bytes);
    let expiry = DateTime::from_timestamp_millis(millis).ok_or_else(|| {
        crate::PersistenceError::Message(format!(
            "{context}: invalid timestamp millis for expiry: {millis}"
        ))
    })?;

    Ok((journey_id, kind, status, expiry))
}

fn encode_timer_task(
    journey_id: Uuid,
    status: u8,
    visible_at: DateTime<Utc>,
    fired_at_unix_ms: i64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(33);
    out.extend_from_slice(journey_id.as_bytes());
    out.push(status);
    out.extend_from_slice(&visible_at.timestamp_millis().to_be_bytes());
    out.extend_from_slice(&fired_at_unix_ms.to_be_bytes());
    out
}

fn decode_timer_task(raw: &[u8], context: &str) -> Result<TimerTaskRow> {
    if raw.len() < 33 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 33 bytes, got {}",
            raw.len()
        )));
    }

    let journey_id = decode_uuid(&raw[..16], context)?;
    let status = raw[16];

    let mut visible_at_bytes = [0_u8; 8];
    visible_at_bytes.copy_from_slice(&raw[17..25]);
    let visible_at_millis = i64::from_be_bytes(visible_at_bytes);
    let visible_at = DateTime::from_timestamp_millis(visible_at_millis).ok_or_else(|| {
        crate::PersistenceError::Message(format!(
            "{context}: invalid timestamp millis for visible_at: {visible_at_millis}"
        ))
    })?;

    Ok(TimerTaskRow {
        journey_id,
        status,
        visible_at,
    })
}

fn encode_timer_due_index_key(visible_at_unix_ms: i64, timer_id: Uuid) -> [u8; 24] {
    let mut out = [0_u8; 24];
    out[..8].copy_from_slice(&encode_sortable_i64(visible_at_unix_ms).to_be_bytes());
    out[8..24].copy_from_slice(timer_id.as_bytes());
    out
}

fn encode_timer_due_index_bound_key(visible_at_unix_ms: i64, upper: bool) -> [u8; 24] {
    let mut out = [0_u8; 24];
    out[..8].copy_from_slice(&encode_sortable_i64(visible_at_unix_ms).to_be_bytes());
    let fill = if upper { 0xFF } else { 0x00 };
    out[8..24].fill(fill);
    out
}

fn decode_timer_due_index_key(raw: &[u8], context: &str) -> Result<(i64, Uuid)> {
    if raw.len() != 24 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected 24-byte timer due index key, got {}",
            raw.len()
        )));
    }

    let mut sortable_millis_bytes = [0_u8; 8];
    sortable_millis_bytes.copy_from_slice(&raw[..8]);
    let sortable_millis = u64::from_be_bytes(sortable_millis_bytes);
    let visible_at_unix_ms = decode_sortable_i64(sortable_millis);
    let timer_id = decode_uuid(&raw[8..24], context)?;
    Ok((visible_at_unix_ms, timer_id))
}

fn encode_sortable_i64(value: i64) -> u64 {
    (value as u64) ^ (1_u64 << 63)
}

fn decode_sortable_i64(value: u64) -> i64 {
    (value ^ (1_u64 << 63)) as i64
}

fn encode_journey_lease(
    owner_id: Uuid,
    lease_until_unix_ms: i64,
    heartbeat_unix_ms: i64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(owner_id.as_bytes());
    out.extend_from_slice(&lease_until_unix_ms.to_be_bytes());
    out.extend_from_slice(&heartbeat_unix_ms.to_be_bytes());
    out
}

fn decode_journey_lease(raw: &[u8], context: &str) -> Result<JourneyLeaseRow> {
    if raw.len() < 32 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 32 bytes, got {}",
            raw.len()
        )));
    }
    let owner_id = decode_uuid(&raw[..16], context)?;
    let mut lease_until_bytes = [0_u8; 8];
    lease_until_bytes.copy_from_slice(&raw[16..24]);
    let lease_until_unix_ms = i64::from_be_bytes(lease_until_bytes);
    Ok(JourneyLeaseRow {
        owner_id,
        lease_until_unix_ms,
    })
}

fn encode_owner_wake_key(owner_id: Uuid, created_at_unix_ms: i64, wake_id: Uuid) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    out.extend_from_slice(owner_id.as_bytes());
    out.extend_from_slice(&created_at_unix_ms.to_be_bytes());
    out.extend_from_slice(wake_id.as_bytes());
    out
}

fn decode_owner_wake_key(raw: &[u8], context: &str) -> Result<(Uuid, i64, Uuid)> {
    if raw.len() < 40 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 40 bytes, got {}",
            raw.len()
        )));
    }
    let owner_id = decode_uuid(&raw[..16], context)?;
    let mut created_at_bytes = [0_u8; 8];
    created_at_bytes.copy_from_slice(&raw[16..24]);
    let created_at_unix_ms = i64::from_be_bytes(created_at_bytes);
    let wake_id = decode_uuid(&raw[24..40], context)?;
    Ok((owner_id, created_at_unix_ms, wake_id))
}

fn encode_owner_wake_value(journey_id: Uuid, timer_id: Uuid) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(journey_id.as_bytes());
    out.extend_from_slice(timer_id.as_bytes());
    out
}

fn decode_owner_wake_value(raw: &[u8], context: &str) -> Result<OwnerWake> {
    if raw.len() < 32 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 32 bytes, got {}",
            raw.len()
        )));
    }
    let journey_id = decode_uuid(&raw[..16], context)?;
    let timer_id = decode_uuid(&raw[16..32], context)?;
    Ok(OwnerWake {
        journey_id,
        timer_id,
    })
}

fn decode_journey(raw: &[u8], context: &str) -> Result<JourneyRow> {
    if raw.len() < 5 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 5 bytes, got {}",
            raw.len()
        )));
    }

    if raw.len() >= 12 && raw[9] == 0xFE {
        let mut animal_id_bytes = [0_u8; 4];
        animal_id_bytes.copy_from_slice(&raw[..4]);
        let animal_id = u32::from_be_bytes(animal_id_bytes);

        let mut generation_bytes = [0_u8; 4];
        generation_bytes.copy_from_slice(&raw[4..8]);
        let generation = u32::from_be_bytes(generation_bytes);

        let status = decode_journey_status(raw[8], context)?;
        let mut ns_len_bytes = [0_u8; 2];
        ns_len_bytes.copy_from_slice(&raw[10..12]);
        let ns_len = usize::from(u16::from_be_bytes(ns_len_bytes));
        let ns_start: usize = 12;
        let ns_end = ns_start.saturating_add(ns_len);
        if ns_end > raw.len() {
            return Err(crate::PersistenceError::Message(format!(
                "{context}: invalid namespace length for journey row"
            )));
        }
        let namespace = std::str::from_utf8(&raw[ns_start..ns_end])
            .map_err(|err| crate::PersistenceError::Message(format!("{context}: {err}")))?;
        let seed = raw[ns_end..].to_vec();
        return Ok(JourneyRow {
            namespace: namespace.to_string(),
            animal_id,
            generation,
            status,
            seed,
        });
    }

    let mut animal_id_bytes = [0_u8; 4];
    animal_id_bytes.copy_from_slice(&raw[..4]);
    let animal_id = u32::from_be_bytes(animal_id_bytes);
    let status = decode_journey_status(raw[4], context)?;
    if raw.len() >= 8 && raw[5] == 0xFF {
        let mut ns_len_bytes = [0_u8; 2];
        ns_len_bytes.copy_from_slice(&raw[6..8]);
        let ns_len = usize::from(u16::from_be_bytes(ns_len_bytes));
        let ns_start: usize = 8;
        let ns_end = ns_start.saturating_add(ns_len);
        if ns_end <= raw.len() {
            if let Ok(namespace) = std::str::from_utf8(&raw[ns_start..ns_end]) {
                let seed = raw[ns_end..].to_vec();
                return Ok(JourneyRow {
                    namespace: namespace.to_string(),
                    animal_id,
                    generation: 0,
                    status,
                    seed,
                });
            }
        }
    }

    // Legacy rows without explicit namespace default to "default".
    let seed = raw[5..].to_vec();
    Ok(JourneyRow {
        namespace: "default".to_string(),
        animal_id,
        generation: 0,
        status,
        seed,
    })
}

fn encode_journey(
    namespace: &str,
    animal_id: u32,
    generation: u32,
    status: JourneyStatus,
    seed: &[u8],
) -> Vec<u8> {
    let namespace_bytes = namespace.as_bytes();
    let namespace_len = u16::try_from(namespace_bytes.len()).unwrap_or(u16::MAX);
    let namespace_bytes = &namespace_bytes[..usize::from(namespace_len)];
    let mut out = Vec::with_capacity(12 + namespace_bytes.len() + seed.len());
    out.extend_from_slice(&animal_id.to_be_bytes());
    out.extend_from_slice(&generation.to_be_bytes());
    out.push(encode_journey_status(status));
    out.push(0xFE);
    out.extend_from_slice(&namespace_len.to_be_bytes());
    out.extend_from_slice(namespace_bytes);
    out.extend_from_slice(seed);
    out
}

fn encode_animal_generation_key(namespace: &str, animal_id: u32) -> Vec<u8> {
    let namespace_bytes = namespace.as_bytes();
    let namespace_len = u16::try_from(namespace_bytes.len()).unwrap_or(u16::MAX);
    let namespace_bytes = &namespace_bytes[..usize::from(namespace_len)];
    let mut out = Vec::with_capacity(6 + namespace_bytes.len());
    out.extend_from_slice(&namespace_len.to_be_bytes());
    out.extend_from_slice(namespace_bytes);
    out.extend_from_slice(&animal_id.to_be_bytes());
    out
}

fn decode_generation(raw: &[u8], context: &str) -> Result<u32> {
    if raw.len() != 4 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected 4-byte generation value, got {}",
            raw.len()
        )));
    }
    let mut generation_bytes = [0_u8; 4];
    generation_bytes.copy_from_slice(raw);
    Ok(u32::from_be_bytes(generation_bytes))
}

fn encode_journey_status(status: JourneyStatus) -> u8 {
    match status {
        JourneyStatus::Created => JOURNEY_STATUS_CREATED,
        JourneyStatus::Alive => JOURNEY_STATUS_ALIVE,
        JourneyStatus::Stopped => JOURNEY_STATUS_STOPPED,
        JourneyStatus::Completed => JOURNEY_STATUS_COMPLETED,
        JourneyStatus::Dead => JOURNEY_STATUS_DEAD,
    }
}

fn decode_journey_status(raw: u8, context: &str) -> Result<JourneyStatus> {
    match raw {
        JOURNEY_STATUS_CREATED => Ok(JourneyStatus::Created),
        JOURNEY_STATUS_ALIVE => Ok(JourneyStatus::Alive),
        JOURNEY_STATUS_STOPPED => Ok(JourneyStatus::Stopped),
        JOURNEY_STATUS_COMPLETED => Ok(JourneyStatus::Completed),
        JOURNEY_STATUS_DEAD => Ok(JourneyStatus::Dead),
        other => Err(crate::PersistenceError::Message(format!(
            "{context}: unknown journey status {other}"
        ))),
    }
}

fn encode_event_key(journey_id: Uuid, sequence_id: u64) -> [u8; 24] {
    let mut key = [0_u8; 24];
    key[..16].copy_from_slice(journey_id.as_bytes());
    key[16..].copy_from_slice(&sequence_id.to_be_bytes());
    key
}

fn decode_event_key(raw: &[u8], context: &str) -> Result<(Uuid, u64)> {
    if raw.len() != 24 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected 24-byte event key, got {}",
            raw.len()
        )));
    }

    let journey_id = decode_uuid(&raw[..16], context)?;
    let mut sequence_bytes = [0_u8; 8];
    sequence_bytes.copy_from_slice(&raw[16..24]);
    let sequence_id = u64::from_be_bytes(sequence_bytes);
    Ok((journey_id, sequence_id))
}

fn encode_event_value(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut value = Vec::with_capacity(1 + data.len());
    value.push(kind);
    value.extend_from_slice(data);
    value
}

fn decode_event_value(raw: &[u8], context: &str) -> Result<(u8, Vec<u8>)> {
    if raw.is_empty() {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 1 byte for event value kind"
        )));
    }
    Ok((raw[0], raw[1..].to_vec()))
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

fn decode_runner_out(journey_id: Uuid, kind: u8, data: Vec<u8>) -> Result<RunnerOut> {
    match kind {
        EVENT_KIND_ACTION_INPUT => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectInput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        EVENT_KIND_ACTION_SUCCESS_OUTPUT => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectSuccessOutput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        EVENT_KIND_ACTION_FAILURE_OUTPUT => {
            let (node_id, data) = decode_effect_event(data)?;
            Ok(RunnerOut::EffectFailureOutput {
                node_id,
                uuid: journey_id,
                data,
            })
        }
        EVENT_KIND_SLEEP_SCHEDULED => {
            let event: SleepScheduledEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerOut::SleepScheduled {
                uuid: journey_id,
                timer_id: event.timer_id,
                wake_at_unix_ms: event.wake_at_unix_ms,
            })
        }
        EVENT_KIND_SLEEP_FIRED => {
            let event: SleepFiredEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerOut::SleepFired {
                uuid: journey_id,
                timer_id: event.timer_id,
                fired_at_unix_ms: event.fired_at_unix_ms,
            })
        }
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported event kind in redb: {other}"
        ))),
    }
}

fn decode_runner_update_out(journey_id: Uuid, kind: u8, data: Vec<u8>) -> Result<RunnerUpdateOut> {
    match kind {
        EVENT_KIND_ACTION_INPUT => {
            let (node_id, _) = decode_effect_event(data)?;
            Ok(RunnerUpdateOut::EffectInput {
                node_id,
                uuid: journey_id,
            })
        }
        EVENT_KIND_ACTION_SUCCESS_OUTPUT => {
            let (node_id, _) = decode_effect_event(data)?;
            Ok(RunnerUpdateOut::EffectSuccessOutput {
                node_id,
                uuid: journey_id,
            })
        }
        EVENT_KIND_ACTION_FAILURE_OUTPUT => {
            let (node_id, _) = decode_effect_event(data)?;
            Ok(RunnerUpdateOut::EffectFailureOutput {
                node_id,
                uuid: journey_id,
            })
        }
        EVENT_KIND_SLEEP_SCHEDULED => {
            let event: SleepScheduledEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerUpdateOut::SleepScheduled {
                uuid: journey_id,
                timer_id: event.timer_id,
                wake_at_unix_ms: event.wake_at_unix_ms,
            })
        }
        EVENT_KIND_SLEEP_FIRED => {
            let event: SleepFiredEvent = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            Ok(RunnerUpdateOut::SleepFired {
                uuid: journey_id,
                timer_id: event.timer_id,
                fired_at_unix_ms: event.fired_at_unix_ms,
            })
        }
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported event kind in redb: {other}"
        ))),
    }
}

fn encode_perturbation_value(lease_until_millis: i64, data: &[u8]) -> Vec<u8> {
    let mut value = Vec::with_capacity(8 + data.len());
    value.extend_from_slice(&lease_until_millis.to_be_bytes());
    value.extend_from_slice(data);
    value
}

fn decode_perturbation_value(raw: &[u8], context: &str) -> Result<(i64, Vec<u8>)> {
    if raw.len() < 8 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 8 bytes, got {}",
            raw.len()
        )));
    }
    let mut lease_bytes = [0_u8; 8];
    lease_bytes.copy_from_slice(&raw[..8]);
    let lease_until = i64::from_be_bytes(lease_bytes);
    Ok((lease_until, raw[8..].to_vec()))
}
