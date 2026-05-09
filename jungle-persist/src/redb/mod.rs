pub mod migrations;

use crate::models::{SchemaVersion, WorkItemKind, WorkItemStatus, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jungle_types::{FlowStatus, RunnerOut, Work};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const FLOWS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("flows");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const WORK_ITEMS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");

const WORK_ITEM_KIND_START_FLOW: u8 = 0;
const WORK_ITEM_STATUS_AVAILABLE: u8 = 0;
const WORK_ITEM_STATUS_CLAIMED: u8 = 1;
const FLOW_STATUS_CREATED: u8 = 0;
const FLOW_STATUS_ALIVE: u8 = 1;
const FLOW_STATUS_STOPPED: u8 = 2;
const FLOW_STATUS_COMPLETED: u8 = 3;
const FLOW_STATUS_DEAD: u8 = 4;

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

    fn update_flow_status(
        &self,
        flow_id: Uuid,
        new_status: FlowStatus,
        expected_current: Option<FlowStatus>,
    ) -> Result<()> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb update_flow_status begin failed: {err}"))
        })?;

        {
            let mut flows = write_tx.open_table(FLOWS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb update_flow_status open flows table failed: {err}"
                ))
            })?;
            let key = &flow_id.as_bytes()[..];
            let existing_raw = {
                let Some(existing) = flows.get(key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb update_flow_status read flow failed: {err}"
                    ))
                })?
                else {
                    return Err(crate::PersistenceError::Message(format!(
                        "flow not found: {flow_id}"
                    )));
                };
                existing.value().to_vec()
            };

            let flow = decode_flow(
                existing_raw.as_slice(),
                "redb update_flow_status decode flow value",
            )?;
            if expected_current.is_none_or(|expected| flow.status == expected) {
                let updated_value = encode_flow(flow.ordinal, new_status, &flow.seed);
                flows.insert(key, updated_value.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb update_flow_status write flow failed: {err}"
                    ))
                })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb update_flow_status commit failed: {err}"
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

    async fn create_flow(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid> {
        let flow_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();
        let expiry = Utc::now();

        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb create_flow begin failed: {err}"))
        })?;

        {
            let mut flows = write_tx.open_table(FLOWS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb create_flow open flows table failed: {err}"
                ))
            })?;
            let flow_value = encode_flow(ordinal, FlowStatus::Created, &seed);
            flows
                .insert(&flow_id.as_bytes()[..], flow_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_flow insert flow failed: {err}"
                    ))
                })?;
        }

        {
            let mut work_items = write_tx.open_table(WORK_ITEMS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb create_flow open work_items table failed: {err}"
                ))
            })?;

            let work_item_value = encode_work_item(
                flow_id,
                WorkItemKind::StartFlow,
                WorkItemStatus::Available,
                expiry,
            );

            work_items
                .insert(&work_item_id.as_bytes()[..], work_item_value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb create_flow insert work item failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb create_flow commit failed: {err}"))
        })?;

        Ok(flow_id)
    }

    async fn flow_status(&self, flow_id: Uuid) -> Result<FlowStatus> {
        let read_tx = self.db.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!("redb flow_status begin read failed: {err}"))
        })?;

        let flows = read_tx.open_table(FLOWS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "redb flow_status open flows table failed: {err}"
            ))
        })?;

        let flow_value = flows
            .get(&flow_id.as_bytes()[..])
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb flow_status read flow failed: {err}"
                ))
            })?
            .ok_or_else(|| {
                crate::PersistenceError::Message(format!("flow not found: {flow_id}"))
            })?;

        let flow = decode_flow(flow_value.value(), "redb flow_status decode flow value")?;
        Ok(flow.status)
    }

    async fn flow_complete(&self, flow_id: Uuid) -> Result<()> {
        self.update_flow_status(flow_id, FlowStatus::Completed, None)
    }

    async fn flow_alive_if_created(&self, flow_id: Uuid) -> Result<()> {
        self.update_flow_status(flow_id, FlowStatus::Alive, Some(FlowStatus::Created))
    }

    async fn claim_work(&self) -> Result<Option<Work>> {
        let write_tx = self.db.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_work begin failed: {err}"))
        })?;

        let mut selected: Option<(Uuid, Uuid, WorkItemKind, DateTime<Utc>)> = None;

        {
            let mut work_items = write_tx.open_table(WORK_ITEMS_TABLE).map_err(|err| {
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
                let (flow_id, kind, status, expiry) =
                    decode_work_item(value.value(), "redb claim_work decode work_item value")?;

                if status != WorkItemStatus::Available {
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
                    selected = Some((id, flow_id, kind, expiry));
                }
            }

            if let Some((selected_id, selected_flow_id, selected_kind, selected_expiry)) = selected
            {
                let claimed = encode_work_item(
                    selected_flow_id,
                    selected_kind,
                    WorkItemStatus::Claimed,
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

        let Some((selected_id, selected_flow_id, selected_kind, _)) = selected else {
            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!("redb claim_work commit failed: {err}"))
            })?;
            return Ok(None);
        };

        let flow = {
            let flows = write_tx.open_table(FLOWS_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "redb claim_work open flows table failed: {err}"
                ))
            })?;
            let flow_key = &selected_flow_id.as_bytes()[..];
            let flow_value = flows
                .get(flow_key)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_work read flow failed: {err}"
                    ))
                })?
                .ok_or_else(|| {
                    crate::PersistenceError::Message(format!(
                        "redb claim_work missing flow for work item {selected_id}"
                    ))
                })?;
            decode_flow(flow_value.value(), "redb claim_work decode flow value")?
        };

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("redb claim_work commit failed: {err}"))
        })?;

        let work = match selected_kind {
            WorkItemKind::StartFlow => Work::StartFlow {
                flow_id: selected_flow_id,
                ordinal: flow.ordinal,
                seed: flow.seed,
            },
        };

        Ok(Some(work))
    }

    async fn append_history(&self, history: RunnerOut) -> Result<()> {
        let (flow_id, kind, data) = match history {
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
                let (entry_flow_id, sequence_id) =
                    decode_event_key(key.value(), "redb append_history decode event key")?;
                if entry_flow_id == flow_id {
                    max_sequence =
                        Some(max_sequence.map_or(sequence_id, |max| max.max(sequence_id)));
                }
            }

            let sequence_id = max_sequence.map_or(0_u64, |max| max.saturating_add(1));
            let event_key = encode_event_key(flow_id, sequence_id);
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

    async fn details(&self, _flow_id: Uuid) -> Result<()> {
        let _ = &self.db;
        todo!()
    }
}

#[derive(Debug)]
struct FlowRow {
    ordinal: u32,
    status: FlowStatus,
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
    flow_id: Uuid,
    kind: WorkItemKind,
    status: WorkItemStatus,
    expiry: DateTime<Utc>,
) -> Vec<u8> {
    let kind = match kind {
        WorkItemKind::StartFlow => WORK_ITEM_KIND_START_FLOW,
    };
    let status = match status {
        WorkItemStatus::Available => WORK_ITEM_STATUS_AVAILABLE,
        WorkItemStatus::Claimed => WORK_ITEM_STATUS_CLAIMED,
    };

    let mut out = Vec::with_capacity(26);
    out.extend_from_slice(flow_id.as_bytes());
    out.push(kind);
    out.push(status);
    out.extend_from_slice(&expiry.timestamp_millis().to_be_bytes());
    out
}

fn decode_work_item(
    raw: &[u8],
    context: &str,
) -> Result<(Uuid, WorkItemKind, WorkItemStatus, DateTime<Utc>)> {
    if raw.len() < 26 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 26 bytes, got {}",
            raw.len()
        )));
    }

    let flow_id = decode_uuid(&raw[..16], context)?;
    let kind = match raw[16] {
        WORK_ITEM_KIND_START_FLOW => WorkItemKind::StartFlow,
        other => {
            return Err(crate::PersistenceError::Message(format!(
                "{context}: unknown work item kind {other}"
            )))
        }
    };
    let status = match raw[17] {
        WORK_ITEM_STATUS_AVAILABLE => WorkItemStatus::Available,
        WORK_ITEM_STATUS_CLAIMED => WorkItemStatus::Claimed,
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

    Ok((flow_id, kind, status, expiry))
}

fn decode_flow(raw: &[u8], context: &str) -> Result<FlowRow> {
    if raw.len() < 5 {
        return Err(crate::PersistenceError::Message(format!(
            "{context}: expected at least 5 bytes, got {}",
            raw.len()
        )));
    }

    let mut ordinal_bytes = [0_u8; 4];
    ordinal_bytes.copy_from_slice(&raw[..4]);
    let ordinal = u32::from_be_bytes(ordinal_bytes);
    let status = decode_flow_status(raw[4], context)?;
    let seed = raw[5..].to_vec();
    Ok(FlowRow {
        ordinal,
        status,
        seed,
    })
}

fn encode_flow(ordinal: u32, status: FlowStatus, seed: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + seed.len());
    out.extend_from_slice(&ordinal.to_be_bytes());
    out.push(encode_flow_status(status));
    out.extend_from_slice(seed);
    out
}

fn encode_flow_status(status: FlowStatus) -> u8 {
    match status {
        FlowStatus::Created => FLOW_STATUS_CREATED,
        FlowStatus::Alive => FLOW_STATUS_ALIVE,
        FlowStatus::Stopped => FLOW_STATUS_STOPPED,
        FlowStatus::Completed => FLOW_STATUS_COMPLETED,
        FlowStatus::Dead => FLOW_STATUS_DEAD,
    }
}

fn decode_flow_status(raw: u8, context: &str) -> Result<FlowStatus> {
    match raw {
        FLOW_STATUS_CREATED => Ok(FlowStatus::Created),
        FLOW_STATUS_ALIVE => Ok(FlowStatus::Alive),
        FLOW_STATUS_STOPPED => Ok(FlowStatus::Stopped),
        FLOW_STATUS_COMPLETED => Ok(FlowStatus::Completed),
        FLOW_STATUS_DEAD => Ok(FlowStatus::Dead),
        other => Err(crate::PersistenceError::Message(format!(
            "{context}: unknown flow status {other}"
        ))),
    }
}

fn encode_event_key(flow_id: Uuid, sequence_id: u64) -> [u8; 24] {
    let mut key = [0_u8; 24];
    key[..16].copy_from_slice(flow_id.as_bytes());
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

    let flow_id = decode_uuid(&raw[..16], context)?;
    let mut sequence_bytes = [0_u8; 8];
    sequence_bytes.copy_from_slice(&raw[16..24]);
    let sequence_id = u64::from_be_bytes(sequence_bytes);
    Ok((flow_id, sequence_id))
}

fn encode_event_value(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut value = Vec::with_capacity(1 + data.len());
    value.push(kind);
    value.extend_from_slice(data);
    value
}
