pub mod migrations;

use crate::models::{SchemaVersion, StepKind, StepStatus, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jungle_types::{ClaimedAnimalPerturbation, JourneyStatus, RunnerOut, RunnerStep};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const JOURNEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journeys");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const STEPS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");
const TIMER_TASKS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("timer_tasks");
const APPEARANCES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("animal_appearances");
const PERTURBATIONS_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("animal_perturbations");

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
                let updated_value = encode_journey(flow.ordinal, new_status, &flow.seed);
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
            SchemaVersion::V0 => self.migrate_v0().await,
        }
    }

    async fn create_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        let journey_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();
        let expiry = Utc::now();

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb create_journey begin failed: {err}"))
        })?;

        {
            let mut journeys = write_tx.open_table(JOURNEYS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb create_journey open journeys table failed: {err}"
                ))
            })?;
            let flow_value = encode_journey(ordinal, JourneyStatus::Created, &seed);
            journeys
                .insert(&journey_id.as_bytes()[..], flow_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_journey insert journey failed: {err}"
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

    async fn claim_work(&self) -> Result<Option<RunnerStep>> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_work begin failed: {err}"))
        })?;

        let mut selected: Option<(Uuid, Uuid, StepKind, DateTime<Utc>)> = None;

        {
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

                if status != StepStatus::Available {
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

            if let Some((selected_id, selected_journey_id, selected_kind, selected_expiry)) =
                selected
            {
                let claimed = encode_work_item(
                    selected_journey_id,
                    selected_kind,
                    StepStatus::Claimed,
                    selected_expiry,
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
            StepKind::StartJourney => RunnerStep::StartJourney {
                journey_id: selected_journey_id,
                ordinal: flow.ordinal,
                seed: flow.seed,
            },
            StepKind::ResumeJourney => RunnerStep::ResumeJourney {
                journey_id: selected_journey_id,
            },
        };

        Ok(Some(work))
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        let (journey_id, kind, data) = match history {
            RunnerOut::ActionInput { data, uuid } => (uuid, EVENT_KIND_ACTION_INPUT, data),
            RunnerOut::ActionSuccessOutput { data, uuid } => {
                (uuid, EVENT_KIND_ACTION_SUCCESS_OUTPUT, data)
            }
            RunnerOut::ActionFailureOutput { data, uuid } => {
                (uuid, EVENT_KIND_ACTION_FAILURE_OUTPUT, data)
            }
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

            let mut max_sequence: Option<u64> = None;
            let iter = events.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb append_history iterate events failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, _) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb append_history read events entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "redb append_history decode event key")?;
                if entry_journey_id == journey_id {
                    max_sequence =
                        Some(max_sequence.map_or(sequence_id, |max| max.max(sequence_id)));
                }
            }

            let sequence_id = max_sequence.map_or(0_u64, |max| max.saturating_add(1));
            let event_key = encode_event_key(journey_id, sequence_id);
            let event_value = encode_event_value(kind, &data);
            events
                .insert(event_key.as_slice(), event_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb append_history insert event failed: {err}"
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
            let timer_value = encode_timer_task(journey_id, TIMER_STATUS_PENDING, wake_at, 0);
            timers
                .insert(&timer_id.as_bytes()[..], timer_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb schedule_sleep_timer insert timer task failed: {err}"
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

            let iter = timers.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers iterate timer_tasks failed: {err}"
                ))
            })?;

            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers read timer task entry failed: {err}"
                    ))
                })?;
                let timer_id = decode_uuid(key.value(), "redb poll_timers decode timer id")?;
                let timer = decode_timer_task(value.value(), "redb poll_timers decode timer")?;
                if timer.status != TIMER_STATUS_PENDING
                    || timer.visible_at.timestamp_millis() > now_millis
                {
                    continue;
                }

                let should_replace = selected
                    .as_ref()
                    .map(|(selected_timer_id, _, selected_visible_at)| {
                        timer.visible_at < *selected_visible_at
                            || (timer.visible_at == *selected_visible_at
                                && timer_id < *selected_timer_id)
                    })
                    .unwrap_or(true);

                if should_replace {
                    selected = Some((timer_id, timer.journey_id, timer.visible_at));
                }
            }

            if let Some((timer_id, journey_id, _)) = selected {
                let fired = encode_timer_task(journey_id, TIMER_STATUS_FIRED, now, now_millis);
                timers
                    .insert(&timer_id.as_bytes()[..], fired.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "redb poll_timers mark timer task fired failed: {err}"
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

        {
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
            let mut max_sequence: Option<u64> = None;
            let iter = events.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb poll_timers iterate events failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, _) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb poll_timers read events entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "redb poll_timers decode event key")?;
                if entry_journey_id == journey_id {
                    max_sequence =
                        Some(max_sequence.map_or(sequence_id, |max| max.max(sequence_id)));
                }
            }

            let sequence_id = max_sequence.map_or(0_u64, |max| max.saturating_add(1));
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
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb poll_timers commit failed: {err}"))
        })?;

        Ok(Some(()))
    }
}

#[derive(Debug)]
struct JourneyRow {
    ordinal: u32,
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

fn decode_journey(raw: &[u8], context: &str) -> Result<JourneyRow> {
    if raw.len() < 5 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 5 bytes, got {}",
            raw.len()
        )));
    }

    let mut ordinal_bytes = [0_u8; 4];
    ordinal_bytes.copy_from_slice(&raw[..4]);
    let ordinal = u32::from_be_bytes(ordinal_bytes);
    let status = decode_journey_status(raw[4], context)?;
    let seed = raw[5..].to_vec();
    Ok(JourneyRow {
        ordinal,
        status,
        seed,
    })
}

fn encode_journey(ordinal: u32, status: JourneyStatus, seed: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + seed.len());
    out.extend_from_slice(&ordinal.to_be_bytes());
    out.push(encode_journey_status(status));
    out.extend_from_slice(seed);
    out
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
