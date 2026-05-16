use redb::{ReadableTable, TableDefinition};
use tracing::warn;

const SCHEMA_METADATA_TABLE: TableDefinition<u8, u32> =
    TableDefinition::new("jungle_schema_metadata");
const JOURNEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journeys");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const STEPS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");
const TIMER_TASKS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("timer_tasks");
const JOURNEY_LEASES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journey_leases");
const OWNER_WAKES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("owner_wakes");
const APPEARANCES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("animal_appearances");
const PERTURBATIONS_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("animal_perturbations");
const ANIMAL_GENERATIONS_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("animal_generations");
const JOURNEY_EVENT_SEQUENCE_TABLE: TableDefinition<&[u8], u64> =
    TableDefinition::new("journey_event_sequences");

pub fn migrate_redb_v0(db: &redb::Database) -> Result<(), String> {
    let tx = db
        .begin_write()
        .map_err(|err| format!("redb migration begin failed: {err}"))?;

    {
        let mut metadata = tx
            .open_table(SCHEMA_METADATA_TABLE)
            .map_err(|err| format!("redb open metadata table failed: {err}"))?;

        let version = metadata
            .get(1)
            .map_err(|err| format!("redb read schema version failed: {err}"))?
            .map(|value| value.value());

        if let Some(version) = version {
            if version != 0 {
                warn!(
                    expected_schema_version = 0,
                    actual_schema_version = version,
                    "redb schema version mismatch"
                );
            }
        } else {
            metadata
                .insert(1, 0)
                .map_err(|err| format!("redb initialize schema version failed: {err}"))?;
        }
    }

    tx.open_table(JOURNEYS_TABLE)
        .map_err(|err| format!("redb open journeys table failed: {err}"))?;
    tx.open_table(EVENTS_TABLE)
        .map_err(|err| format!("redb open events table failed: {err}"))?;
    tx.open_table(STEPS_TABLE)
        .map_err(|err| format!("redb open work_items table failed: {err}"))?;
    tx.open_table(TIMER_TASKS_TABLE)
        .map_err(|err| format!("redb open timer_tasks table failed: {err}"))?;
    tx.open_table(JOURNEY_LEASES_TABLE)
        .map_err(|err| format!("redb open journey_leases table failed: {err}"))?;
    tx.open_table(OWNER_WAKES_TABLE)
        .map_err(|err| format!("redb open owner_wakes table failed: {err}"))?;
    tx.open_table(APPEARANCES_TABLE)
        .map_err(|err| format!("redb open animal_appearances table failed: {err}"))?;
    tx.open_table(PERTURBATIONS_TABLE)
        .map_err(|err| format!("redb open animal_perturbations table failed: {err}"))?;
    tx.open_table(ANIMAL_GENERATIONS_TABLE)
        .map_err(|err| format!("redb open animal_generations table failed: {err}"))?;
    tx.open_table(JOURNEY_EVENT_SEQUENCE_TABLE)
        .map_err(|err| format!("redb open journey_event_sequences table failed: {err}"))?;

    tx.commit()
        .map_err(|err| format!("redb migration commit failed: {err}"))?;

    Ok(())
}
