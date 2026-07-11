use crate::models::{SchemaVersion, StepKind, StepStatus, SCHEMA_VERSION};
use crate::{JungleStore, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fjall::{
    KeyspaceCreateOptions, PersistMode, Readable, SingleWriterTxDatabase, SingleWriterTxKeyspace,
    SingleWriterWriteTx,
};
use jungle_types::{
    ClaimedPerturbable, JourneyEvent, JourneyRecord, JourneyReplayPage, JourneyStatus,
    JourneyUpdateEvent, NodeLifecycle, OwnerWake, RunnerOut, RunnerUpdateOut, SupportedAnimal,
    Work,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct KeyspaceDefinition(&'static str);

impl KeyspaceDefinition {
    const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

const JOURNEYS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("journeys");
const EVENTS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("events");
const EVENT_TIMESTAMPS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("event_timestamps");
const STEPS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("work_items");
const TIMER_TASKS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("timer_tasks");
const TIMER_DUE_INDEX_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("timer_due_index");
const JOURNEY_LEASES_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("journey_leases");
const OWNER_WAKES_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("owner_wakes");
const APPEARANCES_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("animal_appearances");
const PERTURBATIONS_KEYSPACE: KeyspaceDefinition = KeyspaceDefinition::new("animal_perturbations");
const ANIMAL_GENERATIONS_KEYSPACE: KeyspaceDefinition =
    KeyspaceDefinition::new("animal_generations");
const JOURNEY_EVENT_SEQUENCE_KEYSPACE: KeyspaceDefinition =
    KeyspaceDefinition::new("journey_event_sequences");

const STORE_KEYSPACES: [KeyspaceDefinition; 12] = [
    JOURNEYS_KEYSPACE,
    EVENTS_KEYSPACE,
    EVENT_TIMESTAMPS_KEYSPACE,
    STEPS_KEYSPACE,
    TIMER_TASKS_KEYSPACE,
    TIMER_DUE_INDEX_KEYSPACE,
    JOURNEY_LEASES_KEYSPACE,
    OWNER_WAKES_KEYSPACE,
    APPEARANCES_KEYSPACE,
    PERTURBATIONS_KEYSPACE,
    ANIMAL_GENERATIONS_KEYSPACE,
    JOURNEY_EVENT_SEQUENCE_KEYSPACE,
];

struct Keyspaces {
    by_name: HashMap<&'static str, SingleWriterTxKeyspace>,
}

impl Keyspaces {
    fn open(db: &SingleWriterTxDatabase) -> Result<Self> {
        let mut by_name = HashMap::with_capacity(STORE_KEYSPACES.len());
        for definition in STORE_KEYSPACES {
            let keyspace = db
                .keyspace(definition.0, KeyspaceCreateOptions::default)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall open {} keyspace failed: {err}",
                        definition.0
                    ))
                })?;
            by_name.insert(definition.0, keyspace);
        }
        Ok(Self { by_name })
    }

    fn get(&self, definition: KeyspaceDefinition) -> Result<SingleWriterTxKeyspace> {
        self.by_name.get(definition.0).cloned().ok_or_else(|| {
            crate::PersistenceError::Message(format!(
                "fjall keyspace is not initialized: {}",
                definition.0
            ))
        })
    }
}

struct ByteGuard(Vec<u8>);

impl ByteGuard {
    fn value(&self) -> &[u8] {
        &self.0
    }
}

struct KeyspaceIter {
    inner: fjall::Iter,
}

impl Iterator for KeyspaceIter {
    type Item = fjall::Result<(ByteGuard, ByteGuard)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|entry| {
            entry
                .into_inner()
                .map(|(key, value)| (ByteGuard(key.to_vec()), ByteGuard(value.to_vec())))
        })
    }
}

struct ReadTransaction {
    inner: fjall::Snapshot,
    keyspaces: Arc<Keyspaces>,
}

impl ReadTransaction {
    fn open_keyspace(&self, definition: KeyspaceDefinition) -> Result<ReadKeyspace<'_>> {
        Ok(ReadKeyspace {
            tx: &self.inner,
            keyspace: self.keyspaces.get(definition)?,
        })
    }
}

struct ReadKeyspace<'a> {
    tx: &'a fjall::Snapshot,
    keyspace: SingleWriterTxKeyspace,
}

impl ReadKeyspace<'_> {
    fn get(&self, key: &[u8]) -> fjall::Result<Option<ByteGuard>> {
        self.tx
            .get(&self.keyspace, key)
            .map(|value| value.map(|value| ByteGuard(value.to_vec())))
    }

    fn iter(&self) -> fjall::Result<KeyspaceIter> {
        Ok(KeyspaceIter {
            inner: self.tx.iter(&self.keyspace),
        })
    }

    fn range<K, R>(&self, range: R) -> fjall::Result<KeyspaceIter>
    where
        K: AsRef<[u8]>,
        R: std::ops::RangeBounds<K>,
    {
        Ok(KeyspaceIter {
            inner: self.tx.range(&self.keyspace, range),
        })
    }
}

struct WriteTransaction<'a> {
    inner: RefCell<Option<SingleWriterWriteTx<'a>>>,
    keyspaces: Arc<Keyspaces>,
}

impl<'a> WriteTransaction<'a> {
    fn open_keyspace(&self, definition: KeyspaceDefinition) -> Result<WriteKeyspace<'_, 'a>> {
        Ok(WriteKeyspace {
            tx: self,
            keyspace: self.keyspaces.get(definition)?,
        })
    }

    fn commit(self) -> fjall::Result<()> {
        self.inner
            .into_inner()
            .expect("fjall write transaction must exist until commit")
            .commit()
    }
}

struct WriteKeyspace<'tx, 'db> {
    tx: &'tx WriteTransaction<'db>,
    keyspace: SingleWriterTxKeyspace,
}

impl WriteKeyspace<'_, '_> {
    fn get(&self, key: &[u8]) -> fjall::Result<Option<ByteGuard>> {
        self.tx
            .inner
            .borrow()
            .as_ref()
            .expect("fjall write transaction must exist")
            .get(&self.keyspace, key)
            .map(|value| value.map(|value| ByteGuard(value.to_vec())))
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) -> fjall::Result<()> {
        self.tx
            .inner
            .borrow_mut()
            .as_mut()
            .expect("fjall write transaction must exist")
            .insert(&self.keyspace, key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> fjall::Result<Option<ByteGuard>> {
        self.tx
            .inner
            .borrow_mut()
            .as_mut()
            .expect("fjall write transaction must exist")
            .take(&self.keyspace, key.to_vec())
            .map(|value| value.map(|value| ByteGuard(value.to_vec())))
    }

    fn iter(&self) -> fjall::Result<KeyspaceIter> {
        Ok(KeyspaceIter {
            inner: self
                .tx
                .inner
                .borrow()
                .as_ref()
                .expect("fjall write transaction must exist")
                .iter(&self.keyspace),
        })
    }

    fn range<K, R>(&self, range: R) -> fjall::Result<KeyspaceIter>
    where
        K: AsRef<[u8]>,
        R: std::ops::RangeBounds<K>,
    {
        Ok(KeyspaceIter {
            inner: self
                .tx
                .inner
                .borrow()
                .as_ref()
                .expect("fjall write transaction must exist")
                .range(&self.keyspace, range),
        })
    }
}

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
const EVENT_KIND_NODE_LIFECYCLE: u8 = 5;
const FJALL_UPDATES_LOG_INTERVAL: usize = 256;
const FJALL_SLOW_UPDATES_FETCH_WARN_THRESHOLD_MS: u128 = 50;
const FJALL_STALE_EVENT_WARN_MS: i64 = 1_000;

static FJALL_UPDATES_FETCH_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct FjallStore {
    db: Arc<SingleWriterTxDatabase>,
    keyspaces: Arc<Keyspaces>,
    durability: PersistMode,
    claimed_work_ttl_ms: i64,
}

impl std::fmt::Debug for FjallStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FjallStore")
            .field("claimed_work_ttl_ms", &self.claimed_work_ttl_ms)
            .finish_non_exhaustive()
    }
}

impl FjallStore {
    pub fn builder() -> FjallStoreBuilder {
        FjallStoreBuilder::default()
    }

    /// Creates an ephemeral store backed by an auto-cleaned temporary directory.
    pub fn in_memory() -> Result<FjallStore> {
        Self::in_memory_with_claimed_work_ttl_ms(crate::DEFAULT_CLAIMED_WORK_TTL_MS)
    }

    /// Creates an ephemeral store backed by an auto-cleaned temporary directory.
    pub fn in_memory_with_claimed_work_ttl_ms(claimed_work_ttl_ms: i64) -> Result<FjallStore> {
        let path =
            std::env::temp_dir().join(format!("jungle-fjall-{}", Uuid::new_v4().as_hyphenated()));
        Self::open(path, true, claimed_work_ttl_ms)
    }

    fn open(path: PathBuf, temporary: bool, claimed_work_ttl_ms: i64) -> Result<Self> {
        let db = SingleWriterTxDatabase::builder(path)
            .temporary(temporary)
            .open()
            .map_err(crate::PersistenceError::FjallOpen)?;
        let keyspaces = Keyspaces::open(&db)?;
        Ok(Self {
            db: Arc::new(db),
            keyspaces: Arc::new(keyspaces),
            durability: if temporary {
                PersistMode::Buffer
            } else {
                PersistMode::SyncAll
            },
            claimed_work_ttl_ms: claimed_work_ttl_ms.max(0),
        })
    }

    fn begin_write(&self) -> Result<WriteTransaction<'_>> {
        Ok(WriteTransaction {
            inner: RefCell::new(Some(self.db.write_tx().durability(Some(self.durability)))),
            keyspaces: Arc::clone(&self.keyspaces),
        })
    }

    fn begin_read(&self) -> Result<ReadTransaction> {
        Ok(ReadTransaction {
            inner: self.db.read_tx(),
            keyspaces: Arc::clone(&self.keyspaces),
        })
    }

    fn update_journey_status(
        &self,
        journey_id: Uuid,
        new_status: JourneyStatus,
        expected_current: Option<JourneyStatus>,
    ) -> Result<()> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall update_journey_status begin failed: {err}"
            ))
        })?;

        {
            let mut journeys = write_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall update_journey_status open journeys keyspace failed: {err}"
                ))
            })?;
            let key = &journey_id.as_bytes()[..];
            let existing_raw = {
                let Some(existing) = journeys.get(key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall update_journey_status read journey failed: {err}"
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
                "fjall update_journey_status decode journey value",
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
                            "fjall update_journey_status write journey failed: {err}"
                        ))
                    })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall update_journey_status commit failed: {err}"
            ))
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FjallStoreBuilder {
    path: Option<PathBuf>,
    claimed_work_ttl_ms: i64,
}

impl Default for FjallStoreBuilder {
    fn default() -> Self {
        Self {
            path: None,
            claimed_work_ttl_ms: crate::DEFAULT_CLAIMED_WORK_TTL_MS,
        }
    }
}

impl FjallStoreBuilder {
    pub fn path(mut self, value: impl Into<PathBuf>) -> Self {
        self.path = Some(value.into());
        self
    }

    pub fn claimed_work_ttl_ms(mut self, value: i64) -> Self {
        self.claimed_work_ttl_ms = value.max(0);
        self
    }

    pub fn build(self) -> Result<FjallStore> {
        let path = self.path.ok_or(crate::PersistenceError::MissingFjallPath)?;
        FjallStore::open(path, false, self.claimed_work_ttl_ms)
    }
}

#[async_trait]
impl JungleStore for FjallStore {
    async fn migrate(&self) -> Result<()> {
        match SCHEMA_VERSION {
            SchemaVersion::V0 => {
                jungle_migrate::migrate_fjall_v0(&self.db).map_err(crate::PersistenceError::Message)
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

        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall create_journey begin failed: {err}"))
        })?;

        {
            let generations = write_tx
                .open_keyspace(ANIMAL_GENERATIONS_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall create_journey open animal_generations keyspace failed: {err}"
                    ))
                })?;
            let generation_key = encode_animal_generation_key(namespace.as_str(), animal_id);
            let latest_generation = generations
                .get(generation_key.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall create_journey read animal generation failed: {err}"
                    ))
                })?
                .map(|value| {
                    decode_generation(value.value(), "fjall create_journey decode generation")
                })
                .transpose()?
                .unwrap_or(0);

            if generation > latest_generation {
                return Err(crate::PersistenceError::Message(format!(
                    "client generation {generation} exceeds latest server generation {latest_generation} for namespace {namespace} animal {animal_id}"
                )));
            }

            let mut journeys = write_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall create_journey open journeys keyspace failed: {err}"
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
                        "fjall create_journey insert journey failed: {err}"
                    ))
                })?;

            let mut sequences = write_tx
                .open_keyspace(JOURNEY_EVENT_SEQUENCE_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall create_journey open journey_event_sequences keyspace failed: {err}"
                    ))
                })?;
            sequences
                .insert(&journey_id.as_bytes()[..], &0_u64.to_be_bytes())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall create_journey initialize journey event sequence failed: {err}"
                    ))
                })?;
        }

        {
            let mut work_items = write_tx.open_keyspace(STEPS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall create_journey open work_items keyspace failed: {err}"
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
                        "fjall create_journey insert work item failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall create_journey commit failed: {err}"))
        })?;

        Ok(journey_id)
    }

    async fn journey_history(&self, journey_id: Uuid) -> Result<Vec<RunnerOut>> {
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_history begin read failed: {err}"
            ))
        })?;
        let events = read_tx.open_keyspace(EVENTS_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_history open events keyspace failed: {err}"
            ))
        })?;
        let start_key = encode_event_key(journey_id, 0);
        let end_key = encode_event_key(journey_id, u64::MAX);
        let iter = events
            .range(start_key.as_slice()..=end_key.as_slice())
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall journey_history range events failed: {err}"
                ))
            })?;

        let mut rows: Vec<(u64, u8, Vec<u8>)> = Vec::new();
        for entry in iter {
            let (key, value) = entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall journey_history read events entry failed: {err}"
                ))
            })?;
            let (_, sequence_id) =
                decode_event_key(key.value(), "fjall journey_history decode event key")?;
            let (kind, data) =
                decode_event_value(value.value(), "fjall journey_history decode event value")?;
            rows.push((sequence_id, kind, data));
        }

        let mut history = Vec::with_capacity(rows.len());
        for (_, kind, data) in rows {
            history.push(decode_runner_out(journey_id, kind, data)?);
        }
        Ok(history)
    }

    async fn journey_replay_page(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
        snapshot_end_sequence_id: Option<u64>,
        limit: u32,
    ) -> Result<JourneyReplayPage> {
        let fetch_started_at = Instant::now();
        let limit = limit.max(1) as usize;
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_replay_page begin read failed: {err}"
            ))
        })?;
        let snapshot_end_sequence_id = match snapshot_end_sequence_id {
            Some(sequence_id) => Some(sequence_id),
            None => {
                let sequences = read_tx
                    .open_keyspace(JOURNEY_EVENT_SEQUENCE_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall journey_replay_page open sequence keyspace failed: {err}"
                        ))
                    })?;
                let key = &journey_id.as_bytes()[..];
                sequences
                    .get(key)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall journey_replay_page read sequence failed: {err}"
                        ))
                    })?
                    .map(|value| {
                        decode_u64(
                            value.value(),
                            "fjall journey_replay_page decode next sequence",
                        )
                    })
                    .transpose()?
                    .and_then(|next_sequence| next_sequence.checked_sub(1))
            }
        };

        let events = if let Some(snapshot_end_sequence_id) = snapshot_end_sequence_id {
            if after_sequence_id.is_some_and(|after| after >= snapshot_end_sequence_id) {
                Vec::new()
            } else {
                let events = read_tx.open_keyspace(EVENTS_KEYSPACE).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall journey_replay_page open events keyspace failed: {err}"
                    ))
                })?;
                let start_sequence_id = after_sequence_id.map_or(0_u64, |after| after + 1);
                let start_key = encode_event_key(journey_id, start_sequence_id);
                let end_key = encode_event_key(journey_id, snapshot_end_sequence_id);
                let iter = events
                    .range(start_key.as_slice()..=end_key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall journey_replay_page range events failed: {err}"
                        ))
                    })?;

                let mut out = Vec::with_capacity(limit);
                for entry in iter.take(limit) {
                    let (key, value) = entry.map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall journey_replay_page read events entry failed: {err}"
                        ))
                    })?;
                    let (_, sequence_id) = decode_event_key(
                        key.value(),
                        "fjall journey_replay_page decode event key",
                    )?;
                    let (kind, data) = decode_event_value(
                        value.value(),
                        "fjall journey_replay_page decode event value",
                    )?;
                    out.push(JourneyEvent {
                        sequence_id,
                        event: decode_runner_out(journey_id, kind, data)?,
                    });
                }
                out
            }
        } else {
            Vec::new()
        };

        let fetch_elapsed_ms = fetch_started_at.elapsed().as_millis();
        if fetch_elapsed_ms > FJALL_SLOW_UPDATES_FETCH_WARN_THRESHOLD_MS {
            warn!(
                journey_id = %journey_id,
                after_sequence_id = after_sequence_id.unwrap_or(0),
                snapshot_end_sequence_id = snapshot_end_sequence_id.unwrap_or(0),
                limit,
                events_len = events.len(),
                fetch_elapsed_ms,
                "slow fjall journey_replay_page query"
            );
        }

        Ok(JourneyReplayPage {
            snapshot_end_sequence_id,
            events,
        })
    }

    async fn journey_update_events_since(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<Vec<JourneyUpdateEvent>> {
        let fetch_started_at = Instant::now();
        if after_sequence_id == Some(u64::MAX) {
            return Ok(Vec::new());
        }

        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_events_since begin read failed: {err}"
            ))
        })?;
        let events = read_tx.open_keyspace(EVENTS_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_events_since open events keyspace failed: {err}"
            ))
        })?;
        let event_timestamps = read_tx.open_keyspace(EVENT_TIMESTAMPS_KEYSPACE).ok();
        let start_sequence_id = after_sequence_id.map_or(0_u64, |after| after + 1);
        let start_key = encode_event_key(journey_id, start_sequence_id);
        let end_key = encode_event_key(journey_id, u64::MAX);
        let iter = events
            .range(start_key.as_slice()..=end_key.as_slice())
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall journey_events_since range events failed: {err}"
                ))
            })?;

        let mut rows: Vec<(u64, i64, u8, Vec<u8>)> = Vec::new();
        for entry in iter {
            let (key, value) = entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall journey_events_since read events entry failed: {err}"
                ))
            })?;
            let (_, sequence_id) =
                decode_event_key(key.value(), "fjall journey_events_since decode event key")?;
            let event_unix_ms = event_timestamps
                .as_ref()
                .and_then(|timestamps| timestamps.get(key.value()).ok().flatten())
                .map(|value| {
                    decode_i64(
                        value.value(),
                        "fjall journey_events_since decode event timestamp",
                    )
                })
                .transpose()?
                .unwrap_or(0);
            let (kind, data) = decode_event_value(
                value.value(),
                "fjall journey_events_since decode event value",
            )?;
            rows.push((sequence_id, event_unix_ms, kind, data));
        }

        let mut updates = Vec::with_capacity(rows.len());
        for (sequence_id, event_unix_ms, kind, data) in rows {
            updates.push(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms,
                event: decode_runner_update_out(journey_id, kind, data)?,
            });
        }
        let fetch_elapsed_ms = fetch_started_at.elapsed().as_millis();
        let fetch_count = FJALL_UPDATES_FETCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        let now_ms = now_unix_ms();
        let mut max_event_age_ms = 0_i64;
        for update in updates.iter() {
            max_event_age_ms = max_event_age_ms.max(now_ms.saturating_sub(update.event_unix_ms));
        }
        if fetch_elapsed_ms > FJALL_SLOW_UPDATES_FETCH_WARN_THRESHOLD_MS {
            warn!(
                journey_id = %journey_id,
                fetch_count,
                after_sequence_id = after_sequence_id.unwrap_or(0),
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "slow fjall journey_update_events_since query"
            );
        } else if max_event_age_ms > FJALL_STALE_EVENT_WARN_MS {
            warn!(
                journey_id = %journey_id,
                fetch_count,
                after_sequence_id = after_sequence_id.unwrap_or(0),
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "fjall journey_update_events_since returned stale events"
            );
        } else if fetch_count.is_multiple_of(FJALL_UPDATES_LOG_INTERVAL) {
            debug!(
                journey_id = %journey_id,
                fetch_count,
                after_sequence_id = after_sequence_id.unwrap_or(0),
                updates_len = updates.len(),
                fetch_elapsed_ms,
                max_event_age_ms,
                "fjall journey_update_events_since heartbeat"
            );
        }
        Ok(updates)
    }

    async fn journey_status(&self, journey_id: Uuid) -> Result<JourneyStatus> {
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_status begin read failed: {err}"
            ))
        })?;

        let journeys = read_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall journey_status open journeys keyspace failed: {err}"
            ))
        })?;

        let flow_value = journeys
            .get(&journey_id.as_bytes()[..])
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall journey_status read journey failed: {err}"
                ))
            })?
            .ok_or_else(|| {
                crate::PersistenceError::Message(format!("journey not found: {journey_id}"))
            })?;

        let flow = decode_journey(
            flow_value.value(),
            "fjall journey_status decode journey value",
        )?;
        Ok(flow.status)
    }

    async fn list_journeys(&self, namespace: String) -> Result<Vec<JourneyRecord>> {
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall list_journeys begin read failed: {err}"
            ))
        })?;
        let journeys = read_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall list_journeys open journeys keyspace failed: {err}"
            ))
        })?;
        let iter = journeys.iter().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall list_journeys iterate journeys failed: {err}"
            ))
        })?;

        let mut out = Vec::new();
        for entry in iter {
            let (raw_id, raw_journey) = entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall list_journeys read journeys entry failed: {err}"
                ))
            })?;
            let journey_id = decode_uuid(raw_id.value(), "fjall list_journeys decode journey id")?;
            let journey = decode_journey(
                raw_journey.value(),
                "fjall list_journeys decode journey value",
            )?;
            if journey.namespace != namespace {
                continue;
            }
            out.push(JourneyRecord {
                journey_id,
                namespace: journey.namespace,
                animal_id: journey.animal_id,
                generation: journey.generation,
                status: journey.status,
                seed: journey.seed,
            });
        }

        Ok(out)
    }

    async fn animal_appearance(&self, journey_id: Uuid) -> Result<Option<Vec<u8>>> {
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall animal_appearance begin read failed: {err}"
            ))
        })?;

        let appearances = read_tx.open_keyspace(APPEARANCES_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall animal_appearance open animal_appearances keyspace failed: {err}"
            ))
        })?;

        let key = &journey_id.as_bytes()[..];
        let value = appearances.get(key).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall animal_appearance read appearance failed: {err}"
            ))
        })?;

        Ok(value.map(|entry| entry.value().to_vec()))
    }

    async fn upsert_animal_appearance(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall upsert_animal_appearance begin failed: {err}"
            ))
        })?;

        {
            let mut appearances = write_tx
                .open_keyspace(APPEARANCES_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                    "fjall upsert_animal_appearance open animal_appearances keyspace failed: {err}"
                ))
                })?;
            appearances
                .insert(&journey_id.as_bytes()[..], data.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall upsert_animal_appearance write failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall upsert_animal_appearance commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn enqueue_animal_perturbation(&self, journey_id: Uuid, data: Vec<u8>) -> Result<()> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall enqueue_animal_perturbation begin failed: {err}"
            ))
        })?;

        {
            let mut perturbations = write_tx.open_keyspace(PERTURBATIONS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall enqueue_animal_perturbation open animal_perturbations keyspace failed: {err}"
                ))
            })?;

            let mut max_sequence: Option<u64> = None;
            let iter = perturbations.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall enqueue_animal_perturbation iterate keyspace failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, _) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall enqueue_animal_perturbation read entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "fjall enqueue_animal_perturbation decode key")?;
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
                        "fjall enqueue_animal_perturbation insert failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall enqueue_animal_perturbation commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn claim_animal_perturbation(
        &self,
        journey_id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall claim_animal_perturbation begin failed: {err}"
            ))
        })?;

        let now = Utc::now().timestamp_millis();
        let lease_until = now.saturating_add(30_000);
        let mut selected: Option<(u64, Vec<u8>)> = None;

        {
            let mut perturbations = write_tx.open_keyspace(PERTURBATIONS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_animal_perturbation open animal_perturbations keyspace failed: {err}"
                ))
            })?;
            let iter = perturbations.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_animal_perturbation iterate keyspace failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_animal_perturbation read entry failed: {err}"
                    ))
                })?;
                let (entry_journey_id, sequence_id) =
                    decode_event_key(key.value(), "fjall claim_animal_perturbation decode key")?;
                if entry_journey_id != journey_id {
                    continue;
                }
                let (entry_lease_until, payload) = decode_perturbation_value(
                    value.value(),
                    "fjall claim_animal_perturbation decode value",
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
                            "fjall claim_animal_perturbation write claim failed: {err}"
                        ))
                    })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall claim_animal_perturbation commit failed: {err}"
            ))
        })?;

        let Some((sequence_id, data)) = selected else {
            return Ok(None);
        };
        Ok(Some(ClaimedPerturbable {
            id: sequence_id,
            data,
        }))
    }

    async fn ack_animal_perturbation(&self, journey_id: Uuid, perturbation_id: u64) -> Result<()> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall ack_animal_perturbation begin failed: {err}"
            ))
        })?;
        {
            let mut perturbations =
                write_tx
                    .open_keyspace(PERTURBATIONS_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                    "fjall ack_animal_perturbation open animal_perturbations keyspace failed: {err}"
                ))
                    })?;
            let key = encode_event_key(journey_id, perturbation_id);
            let removed = perturbations.remove(key.as_slice()).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall ack_animal_perturbation remove failed: {err}"
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
                "fjall ack_animal_perturbation commit failed: {err}"
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

        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall heartbeat_journey_lease begin failed: {err}"
            ))
        })?;
        {
            let mut leases = write_tx
                .open_keyspace(JOURNEY_LEASES_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall heartbeat_journey_lease open journey_leases keyspace failed: {err}"
                    ))
                })?;
            let key = &journey_id.as_bytes()[..];
            let value = encode_journey_lease(owner_id, lease_until_millis, now_millis);
            leases.insert(key, value.as_slice()).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall heartbeat_journey_lease write lease failed: {err}"
                ))
            })?;
        }
        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall heartbeat_journey_lease commit failed: {err}"
            ))
        })?;
        Ok(())
    }

    async fn claim_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>> {
        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall claim_owner_wake begin failed: {err}"))
        })?;

        let mut selected_key: Option<Vec<u8>> = None;
        let mut selected_value: Option<Vec<u8>> = None;

        {
            let mut owner_wakes = write_tx
                .open_keyspace(OWNER_WAKES_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_owner_wake open owner_wakes keyspace failed: {err}"
                    ))
                })?;

            let iter = owner_wakes.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_owner_wake iterate owner_wakes failed: {err}"
                ))
            })?;
            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_owner_wake read owner_wakes entry failed: {err}"
                    ))
                })?;
                let (entry_owner_id, _, _) =
                    decode_owner_wake_key(key.value(), "fjall claim_owner_wake decode key")?;
                if entry_owner_id == owner_id {
                    selected_key = Some(key.value().to_vec());
                    selected_value = Some(value.value().to_vec());
                    break;
                }
            }

            if let Some(key) = selected_key.as_ref() {
                owner_wakes.remove(key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_owner_wake remove wake failed: {err}"
                    ))
                })?;
            }
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall claim_owner_wake commit failed: {err}"))
        })?;

        let Some(value) = selected_value else {
            return Ok(None);
        };
        let wake =
            decode_owner_wake_value(value.as_slice(), "fjall claim_owner_wake decode value")?;
        Ok(Some(wake))
    }

    async fn journey_complete(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(journey_id, JourneyStatus::Completed, None)
    }

    async fn journey_dead(&self, journey_id: Uuid) -> Result<()> {
        self.update_journey_status(journey_id, JourneyStatus::Dead, None)
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

        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall claim_work begin failed: {err}"))
        })?;
        let now = Utc::now();
        let now_millis = now_unix_ms();
        let lease_until = now + chrono::Duration::milliseconds(self.claimed_work_ttl_ms);

        let mut selected: Option<(Uuid, Uuid, StepKind, DateTime<Utc>)> = None;

        {
            let mut generation_keyspace = write_tx
                .open_keyspace(ANIMAL_GENERATIONS_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_work open animal_generations keyspace failed: {err}"
                    ))
                })?;
            for supported in supported_animals {
                let key = encode_animal_generation_key(namespace.as_str(), supported.animal_id);
                let existing = generation_keyspace
                    .get(key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall claim_work read animal generation failed: {err}"
                        ))
                    })?
                    .map(|value| {
                        decode_generation(value.value(), "fjall claim_work decode generation")
                    })
                    .transpose()?
                    .unwrap_or(0);
                if supported.generation > existing {
                    generation_keyspace
                        .insert(
                            key.as_slice(),
                            supported.generation.to_be_bytes().as_slice(),
                        )
                        .map_err(|err| {
                            crate::PersistenceError::Message(format!(
                                "fjall claim_work write animal generation failed: {err}"
                            ))
                        })?;
                }
            }

            let journeys = write_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_work open journeys keyspace failed: {err}"
                ))
            })?;
            let journey_leases =
                write_tx
                    .open_keyspace(JOURNEY_LEASES_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall claim_work open journey_leases keyspace failed: {err}"
                        ))
                    })?;
            let mut work_items = write_tx.open_keyspace(STEPS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_work open work_items keyspace failed: {err}"
                ))
            })?;

            let iter = work_items.iter().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_work iterate work_items failed: {err}"
                ))
            })?;

            for entry in iter {
                let (key, value) = entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_work read work_items entry failed: {err}"
                    ))
                })?;
                let id = decode_uuid(key.value(), "fjall claim_work decode work_item id")?;
                let (journey_id, kind, status, expiry) =
                    decode_work_item(value.value(), "fjall claim_work decode work_item value")?;

                let has_active_lease = match journey_leases
                    .get(&journey_id.as_bytes()[..])
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall claim_work read journey lease failed: {err}"
                        ))
                    })? {
                    Some(raw_lease) => {
                        let lease = decode_journey_lease(
                            raw_lease.value(),
                            "fjall claim_work decode journey lease",
                        )?;
                        lease.lease_until_unix_ms > now_millis
                    }
                    None => false,
                };

                let claimable = match status {
                    StepStatus::Available => true,
                    StepStatus::Claimed => expiry <= now && !has_active_lease,
                };
                if !claimable {
                    continue;
                }

                let journey_raw = journeys
                    .get(&journey_id.as_bytes()[..])
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall claim_work read journey for namespace filter failed: {err}"
                        ))
                    })?
                    .ok_or_else(|| {
                        crate::PersistenceError::Message(format!(
                            "fjall claim_work missing journey for work item {id}"
                        ))
                    })?;
                let journey = decode_journey(
                    journey_raw.value(),
                    "fjall claim_work decode journey for namespace filter",
                )?;
                if journey.namespace != namespace.as_str() {
                    continue;
                }
                if !matches!(
                    journey.status,
                    JourneyStatus::Created | JourneyStatus::Alive
                ) {
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
                            "fjall claim_work update work_items status failed: {err}"
                        ))
                    })?;
            }
        }

        let Some((selected_id, selected_journey_id, selected_kind, _)) = selected else {
            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!("fjall claim_work commit failed: {err}"))
            })?;
            return Ok(None);
        };

        let flow = {
            let journeys = write_tx.open_keyspace(JOURNEYS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall claim_work open journeys keyspace failed: {err}"
                ))
            })?;
            let flow_key = &selected_journey_id.as_bytes()[..];
            let flow_value = journeys
                .get(flow_key)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_work read journey failed: {err}"
                    ))
                })?
                .ok_or_else(|| {
                    crate::PersistenceError::Message(format!(
                        "fjall claim_work missing journey for work item {selected_id}"
                    ))
                })?;
            decode_journey(flow_value.value(), "fjall claim_work decode journey value")?
        };

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall claim_work commit failed: {err}"))
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

    async fn append_history(&self, history: RunnerOut, event_unix_ms: i64) -> Result<()> {
        let (journey_id, kind, data) = match history {
            RunnerOut::NodeLifecycle(node) => (
                node.uuid,
                EVENT_KIND_NODE_LIFECYCLE,
                postcard::to_allocvec(&node)
                    .map_err(|err| crate::PersistenceError::Message(err.to_string()))?,
            ),
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
                    "appearance snapshots are not history events in fjall".to_string(),
                ))
            }
        };

        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall append_history begin failed: {err}"))
        })?;

        {
            let mut events = write_tx.open_keyspace(EVENTS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall append_history open events keyspace failed: {err}"
                ))
            })?;
            let mut event_timestamps =
                write_tx
                    .open_keyspace(EVENT_TIMESTAMPS_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall append_history open event_timestamps keyspace failed: {err}"
                        ))
                    })?;
            let mut sequences = write_tx
                .open_keyspace(JOURNEY_EVENT_SEQUENCE_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall append_history open journey_event_sequences keyspace failed: {err}"
                    ))
                })?;

            let key = &journey_id.as_bytes()[..];
            let sequence_id = if let Some(next) = sequences.get(key).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall append_history read journey_event_sequences failed: {err}"
                ))
            })? {
                decode_u64(
                    next.value(),
                    "fjall append_history decode next event sequence",
                )?
            } else {
                let start_key = encode_event_key(journey_id, 0);
                let end_key = encode_event_key(journey_id, u64::MAX);
                let iter = events
                    .range(start_key.as_slice()..=end_key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall append_history range events failed: {err}"
                        ))
                    })?;
                let mut next_sequence = 0_u64;
                for entry in iter {
                    let (event_key, _) = entry.map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall append_history read events entry failed: {err}"
                        ))
                    })?;
                    let (_, existing_sequence_id) = decode_event_key(
                        event_key.value(),
                        "fjall append_history decode event key",
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
                        "fjall append_history insert event failed: {err}"
                    ))
                })?;
            event_timestamps
                .insert(event_key.as_slice(), &event_unix_ms.to_be_bytes())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall append_history insert event timestamp failed: {err}"
                    ))
                })?;
            sequences
                .insert(key, &sequence_id.saturating_add(1).to_be_bytes())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall append_history write journey_event_sequences failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall append_history commit failed: {err}"))
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

        {
            let write_tx = self.begin_write().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall schedule_sleep_timer begin failed: {err}"
                ))
            })?;

            {
                let mut timers = write_tx
                    .open_keyspace(TIMER_TASKS_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall schedule_sleep_timer open timer_tasks keyspace failed: {err}"
                        ))
                    })?;
                let mut due_index =
                    write_tx
                        .open_keyspace(TIMER_DUE_INDEX_KEYSPACE)
                        .map_err(|err| {
                            crate::PersistenceError::Message(format!(
                            "fjall schedule_sleep_timer open timer_due_index keyspace failed: {err}"
                        ))
                        })?;
                let timer_key = &timer_id.as_bytes()[..];
                if let Some(existing) = timers.get(timer_key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall schedule_sleep_timer read existing timer task failed: {err}"
                    ))
                })? {
                    let existing_timer = decode_timer_task(
                        existing.value(),
                        "fjall schedule_sleep_timer decode existing timer task",
                    )?;
                    let stale_due_key = encode_timer_due_index_key(
                        existing_timer.visible_at.timestamp_millis(),
                        timer_id,
                    );
                    let _ = due_index.remove(stale_due_key.as_slice()).map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall schedule_sleep_timer remove stale timer_due_index entry failed: {err}"
                        ))
                    })?;
                }
                let timer_value = encode_timer_task(journey_id, TIMER_STATUS_PENDING, wake_at, 0);
                timers
                    .insert(timer_key, timer_value.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall schedule_sleep_timer insert timer task failed: {err}"
                        ))
                    })?;
                let due_key = encode_timer_due_index_key(wake_at.timestamp_millis(), timer_id);
                due_index
                    .insert(due_key.as_slice(), &[] as &[u8])
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall schedule_sleep_timer insert timer_due_index entry failed: {err}"
                        ))
                    })?;
            }

            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall schedule_sleep_timer commit failed: {err}"
                ))
            })?;
        }

        self.append_history(
            RunnerOut::SleepScheduled {
                uuid: journey_id,
                timer_id,
                wake_at_unix_ms,
            },
            Utc::now().timestamp_millis(),
        )
        .await
    }

    async fn next_timer_due_at(&self) -> Result<Option<i64>> {
        let read_tx = self.begin_read().map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall next_timer_due_at begin read failed: {err}"
            ))
        })?;
        let timers = read_tx.open_keyspace(TIMER_TASKS_KEYSPACE).map_err(|err| {
            crate::PersistenceError::Message(format!(
                "fjall next_timer_due_at open timer_tasks keyspace failed: {err}"
            ))
        })?;
        let due_index = read_tx
            .open_keyspace(TIMER_DUE_INDEX_KEYSPACE)
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall next_timer_due_at open timer_due_index keyspace failed: {err}"
                ))
            })?;
        let due_start = encode_timer_due_index_key(i64::MIN, Uuid::nil());
        let due_end = encode_timer_due_index_bound_key(i64::MAX, true);
        let due_iter = due_index
            .range(due_start.as_slice()..=due_end.as_slice())
            .map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall next_timer_due_at range timer_due_index failed: {err}"
                ))
            })?;

        for due_entry in due_iter {
            let (due_key, _) = due_entry.map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall next_timer_due_at read timer_due_index entry failed: {err}"
                ))
            })?;
            let (indexed_visible_at_unix_ms, timer_id) = decode_timer_due_index_key(
                due_key.value(),
                "fjall next_timer_due_at decode timer_due_index key",
            )?;

            let Some(timer_value) = timers.get(&timer_id.as_bytes()[..]).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall next_timer_due_at read timer task by due index failed: {err}"
                ))
            })?
            else {
                continue;
            };

            let timer = decode_timer_task(
                timer_value.value(),
                "fjall next_timer_due_at decode timer task by due index",
            )?;
            let timer_visible_at_unix_ms = timer.visible_at.timestamp_millis();
            if timer.status != TIMER_STATUS_PENDING
                || timer_visible_at_unix_ms != indexed_visible_at_unix_ms
            {
                continue;
            }

            return Ok(Some(timer_visible_at_unix_ms));
        }

        Ok(None)
    }

    async fn poll_timers(&self) -> Result<Option<()>> {
        let now = Utc::now();
        let now_millis = now.timestamp_millis();

        let write_tx = self.begin_write().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall poll_timers begin failed: {err}"))
        })?;

        let mut selected: Option<(Uuid, Uuid, DateTime<Utc>)> = None;
        {
            let mut timers = write_tx
                .open_keyspace(TIMER_TASKS_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers open timer_tasks keyspace failed: {err}"
                    ))
                })?;
            let mut due_index =
                write_tx
                    .open_keyspace(TIMER_DUE_INDEX_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall poll_timers open timer_due_index keyspace failed: {err}"
                        ))
                    })?;
            let due_start = encode_timer_due_index_key(i64::MIN, Uuid::nil());
            let due_end = encode_timer_due_index_bound_key(now_millis, true);
            let due_iter = due_index
                .range(due_start.as_slice()..=due_end.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers range timer_due_index failed: {err}"
                    ))
                })?;
            let mut stale_due_keys: Vec<Vec<u8>> = Vec::new();
            let mut selected_due_key: Option<Vec<u8>> = None;
            for due_entry in due_iter {
                let (due_key, _) = due_entry.map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers read timer_due_index entry failed: {err}"
                    ))
                })?;
                let (indexed_visible_at_unix_ms, timer_id) = decode_timer_due_index_key(
                    due_key.value(),
                    "fjall poll_timers decode timer_due_index key",
                )?;

                let timer_key = &timer_id.as_bytes()[..];
                let Some(timer_value) = timers.get(timer_key).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers read timer task by due index failed: {err}"
                    ))
                })?
                else {
                    stale_due_keys.push(due_key.value().to_vec());
                    continue;
                };

                let timer = decode_timer_task(
                    timer_value.value(),
                    "fjall poll_timers decode timer task by due index",
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
                        "fjall poll_timers remove stale timer_due_index entry failed: {err}"
                    ))
                })?;
            }

            if let Some((timer_id, journey_id, visible_at)) = selected {
                let fired = encode_timer_task(journey_id, TIMER_STATUS_FIRED, now, now_millis);
                timers
                    .insert(&timer_id.as_bytes()[..], fired.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall poll_timers mark timer task fired failed: {err}"
                        ))
                    })?;
                let due_key = selected_due_key.unwrap_or_else(|| {
                    encode_timer_due_index_key(visible_at.timestamp_millis(), timer_id).to_vec()
                });
                let _ = due_index.remove(due_key.as_slice()).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers remove fired timer_due_index entry failed: {err}"
                    ))
                })?;
            }
        }

        let Some((timer_id, journey_id, _)) = selected else {
            write_tx.commit().map_err(|err| {
                crate::PersistenceError::Message(format!("fjall poll_timers commit failed: {err}"))
            })?;
            return Ok(None);
        };

        let mut valid_owner: Option<Uuid> = None;
        {
            let leases = write_tx
                .open_keyspace(JOURNEY_LEASES_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers open journey_leases keyspace failed: {err}"
                    ))
                })?;
            let lease_entry = leases.get(&journey_id.as_bytes()[..]).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall poll_timers read journey lease failed: {err}"
                ))
            })?;
            if let Some(raw) = lease_entry {
                let lease =
                    decode_journey_lease(raw.value(), "fjall poll_timers decode journey lease")?;
                if lease.lease_until_unix_ms > now_millis {
                    valid_owner = Some(lease.owner_id);
                }
            }
        }

        if let Some(owner_id) = valid_owner {
            let mut owner_wakes = write_tx
                .open_keyspace(OWNER_WAKES_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers open owner_wakes keyspace failed: {err}"
                    ))
                })?;
            let wake_id = Uuid::new_v4();
            let key = encode_owner_wake_key(owner_id, now_millis, wake_id);
            let value = encode_owner_wake_value(journey_id, timer_id);
            owner_wakes
                .insert(key.as_slice(), value.as_slice())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers enqueue owner wake failed: {err}"
                    ))
                })?;
        } else {
            let mut work_items = write_tx.open_keyspace(STEPS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall poll_timers open work_items keyspace failed: {err}"
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
                        "fjall poll_timers enqueue resume work item failed: {err}"
                    ))
                })?;
        }

        {
            let mut events = write_tx.open_keyspace(EVENTS_KEYSPACE).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall poll_timers open events keyspace failed: {err}"
                ))
            })?;
            let mut event_timestamps =
                write_tx
                    .open_keyspace(EVENT_TIMESTAMPS_KEYSPACE)
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall poll_timers open event_timestamps keyspace failed: {err}"
                        ))
                    })?;
            let mut sequences = write_tx
                .open_keyspace(JOURNEY_EVENT_SEQUENCE_KEYSPACE)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers open journey_event_sequences keyspace failed: {err}"
                    ))
                })?;
            let key = &journey_id.as_bytes()[..];
            let sequence_id = if let Some(next) = sequences.get(key).map_err(|err| {
                crate::PersistenceError::Message(format!(
                    "fjall poll_timers read journey_event_sequences failed: {err}"
                ))
            })? {
                decode_u64(next.value(), "fjall poll_timers decode next event sequence")?
            } else {
                let start_key = encode_event_key(journey_id, 0);
                let end_key = encode_event_key(journey_id, u64::MAX);
                let iter = events
                    .range(start_key.as_slice()..=end_key.as_slice())
                    .map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall poll_timers range events failed: {err}"
                        ))
                    })?;
                let mut next_sequence = 0_u64;
                for entry in iter {
                    let (event_key, _) = entry.map_err(|err| {
                        crate::PersistenceError::Message(format!(
                            "fjall poll_timers read events entry failed: {err}"
                        ))
                    })?;
                    let (_, existing_sequence_id) =
                        decode_event_key(event_key.value(), "fjall poll_timers decode event key")?;
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
                        "fjall poll_timers insert sleep fired event failed: {err}"
                    ))
                })?;
            event_timestamps
                .insert(event_key.as_slice(), &now_millis.to_be_bytes())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers insert sleep fired event timestamp failed: {err}"
                    ))
                })?;
            sequences
                .insert(key, &sequence_id.saturating_add(1).to_be_bytes())
                .map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "fjall poll_timers write journey_event_sequences failed: {err}"
                    ))
                })?;
        }

        write_tx.commit().map_err(|err| {
            crate::PersistenceError::Message(format!("fjall poll_timers commit failed: {err}"))
        })?;

        Ok(Some(()))
    }
}

fn now_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
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

fn decode_u64(raw: &[u8], context: &str) -> Result<u64> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        crate::PersistenceError::Message(format!(
            "{context}: expected 8-byte u64 value, got {}",
            raw.len()
        ))
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_i64(raw: &[u8], context: &str) -> Result<i64> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        crate::PersistenceError::Message(format!(
            "{context}: expected 8-byte i64 value, got {}",
            raw.len()
        ))
    })?;
    Ok(i64::from_be_bytes(bytes))
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
        EVENT_KIND_NODE_LIFECYCLE => {
            let mut event: NodeLifecycle = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            event.uuid = journey_id;
            Ok(RunnerOut::NodeLifecycle(event))
        }
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported event kind in fjall: {other}"
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
        EVENT_KIND_NODE_LIFECYCLE => {
            let mut event: NodeLifecycle = postcard::from_bytes(&data)
                .map_err(|err| crate::PersistenceError::Message(err.to_string()))?;
            event.uuid = journey_id;
            Ok(RunnerUpdateOut::NodeLifecycle(event))
        }
        other => Err(crate::PersistenceError::Message(format!(
            "unsupported event kind in fjall: {other}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_backed_store_reopens_and_continues_event_sequence() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let path = tempdir.path().join("jungle.fjall");

        let store = FjallStore::builder()
            .path(&path)
            .build()
            .expect("fjall store should open");
        store.migrate().await.expect("migration should succeed");
        let journey_id = store
            .create_journey("reopen".to_string(), 7, 0, vec![1, 2, 3])
            .await
            .expect("journey should be created");
        store
            .append_history(
                RunnerOut::EffectInput {
                    node_id: 10,
                    data: vec![4],
                    uuid: journey_id,
                },
                100,
            )
            .await
            .expect("first event should append");
        store
            .append_history(
                RunnerOut::EffectSuccessOutput {
                    node_id: 10,
                    data: vec![5],
                    uuid: journey_id,
                },
                101,
            )
            .await
            .expect("second event should append");
        store
            .upsert_animal_appearance(journey_id, vec![6, 7])
            .await
            .expect("appearance should persist");
        store
            .enqueue_animal_perturbation(journey_id, vec![8, 9])
            .await
            .expect("perturbation should persist");
        drop(store);

        let reopened = FjallStore::builder()
            .path(&path)
            .build()
            .expect("fjall store should reopen");
        reopened
            .migrate()
            .await
            .expect("migration should be idempotent");
        assert_eq!(
            reopened
                .journey_status(journey_id)
                .await
                .expect("journey status should load"),
            JourneyStatus::Created
        );
        assert_eq!(
            reopened
                .animal_appearance(journey_id)
                .await
                .expect("appearance should load"),
            Some(vec![6, 7])
        );
        let updates = reopened
            .journey_update_events_since(journey_id, None)
            .await
            .expect("updates should load");
        assert_eq!(
            updates
                .iter()
                .map(|event| event.event_unix_ms)
                .collect::<Vec<_>>(),
            vec![100, 101]
        );
        let perturbation = reopened
            .claim_animal_perturbation(journey_id)
            .await
            .expect("perturbation claim should succeed")
            .expect("persisted perturbation should exist");
        assert_eq!(perturbation.id, 0);
        assert_eq!(perturbation.data, vec![8, 9]);
        reopened
            .append_history(
                RunnerOut::EffectFailureOutput {
                    node_id: 10,
                    data: vec![11],
                    uuid: journey_id,
                },
                102,
            )
            .await
            .expect("event should append after reopen");
        let page = reopened
            .journey_replay_page(journey_id, None, None, 10)
            .await
            .expect("replay should load");
        assert_eq!(page.snapshot_end_sequence_id, Some(2));
        assert_eq!(
            page.events
                .iter()
                .map(|event| event.sequence_id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_read_modify_write_operations_are_serialized() {
        const CONCURRENCY: u32 = 16;

        let store = FjallStore::in_memory().expect("temporary fjall store should open");
        store.migrate().await.expect("migration should succeed");
        let journey_id = store
            .create_journey("concurrency".to_string(), 3, 0, vec![1])
            .await
            .expect("journey should be created");

        let mut append_tasks = Vec::new();
        for node_id in 0..CONCURRENCY {
            let store = store.clone();
            append_tasks.push(tokio::spawn(async move {
                store
                    .append_history(
                        RunnerOut::EffectInput {
                            node_id,
                            data: vec![node_id as u8],
                            uuid: journey_id,
                        },
                        i64::from(node_id),
                    )
                    .await
            }));
        }
        for task in append_tasks {
            task.await
                .expect("append task should join")
                .expect("append should succeed");
        }
        let page = store
            .journey_replay_page(journey_id, None, None, CONCURRENCY + 1)
            .await
            .expect("replay should load");
        assert_eq!(
            page.events
                .iter()
                .map(|event| event.sequence_id)
                .collect::<Vec<_>>(),
            (0..u64::from(CONCURRENCY)).collect::<Vec<_>>()
        );

        let mut work_tasks = Vec::new();
        for _ in 0..CONCURRENCY {
            let store = store.clone();
            work_tasks.push(tokio::spawn(async move {
                store
                    .claim_work(
                        "concurrency".to_string(),
                        vec![SupportedAnimal {
                            animal_id: 3,
                            generation: 0,
                        }],
                    )
                    .await
            }));
        }
        let mut work_winners = 0;
        for task in work_tasks {
            if task
                .await
                .expect("work task should join")
                .expect("work claim should succeed")
                .is_some()
            {
                work_winners += 1;
            }
        }
        assert_eq!(work_winners, 1);

        store
            .enqueue_animal_perturbation(journey_id, vec![42])
            .await
            .expect("perturbation should enqueue");
        let mut perturbation_tasks = Vec::new();
        for _ in 0..CONCURRENCY {
            let store = store.clone();
            perturbation_tasks.push(tokio::spawn(async move {
                store.claim_animal_perturbation(journey_id).await
            }));
        }
        let mut perturbation_winners = 0;
        for task in perturbation_tasks {
            if task
                .await
                .expect("perturbation task should join")
                .expect("perturbation claim should succeed")
                .is_some()
            {
                perturbation_winners += 1;
            }
        }
        assert_eq!(perturbation_winners, 1);
    }
}
