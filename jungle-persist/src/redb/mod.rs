pub mod migrations;

use crate::models::{SchemaVersion, StepKind, StepStatus, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jungle_types::{JourneyStatus, RunnerOut, RunnerStep};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const JOURNEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journeys");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const STEPS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");

const STEP_KIND_START_JOURNEY: u8 = 0;
const STEP_STATUS_AVAILABLE: u8 = 0;
const STEP_STATUS_CLAIMED: u8 = 1;
const JOURNEY_STATUS_CREATED: u8 = 0;
const JOURNEY_STATUS_ALIVE: u8 = 1;
const JOURNEY_STATUS_STOPPED: u8 = 2;
const JOURNEY_STATUS_COMPLETED: u8 = 3;
const JOURNEY_STATUS_DEAD: u8 = 4;

const EVENT_KIND_ACTION_INPUT: u8 = 0;
const EVENT_KIND_ACTION_SUCCESS_OUTPUT: u8 = 1;
const EVENT_KIND_ACTION_FAILURE_OUTPUT: u8 = 2;

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
            crate::PersistenceError::Message(format!("redb update_journey_status begin failed: {err}"))
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
                journeys.insert(key, updated_value.as_slice()).map_err(|err| {
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
            crate::PersistenceError::Message(format!("redb journey_status begin read failed: {err}"))
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

        let flow = decode_journey(flow_value.value(), "redb journey_status decode journey value")?;
        Ok(flow.status)
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(journey_id, JourneyStatus::Completed, None)
    }

    async fn journey_alive_if_created(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(journey_id, JourneyStatus::Alive, Some(JourneyStatus::Created))
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

            if let Some((selected_id, selected_journey_id, selected_kind, selected_expiry)) = selected
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

    async fn poll_timers(&self) -> Result<Option<()>> {
        let _ = &self.db;
        todo!()
    }

    async fn details(&self, _journey_id: Uuid) -> Result<()> {
        let _ = &self.db;
        todo!()
    }
}

#[derive(Debug)]
struct JourneyRow {
    ordinal: u32,
    status: JourneyStatus,
    seed: Vec<u8>,
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
