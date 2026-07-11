use fjall::{KeyspaceCreateOptions, PersistMode, Readable, SingleWriterTxDatabase};
use tracing::warn;

const SCHEMA_METADATA_KEYSPACE: &str = "jungle_schema_metadata";
const DATA_KEYSPACES: [&str; 12] = [
    "journeys",
    "events",
    "event_timestamps",
    "work_items",
    "timer_tasks",
    "timer_due_index",
    "journey_leases",
    "owner_wakes",
    "animal_appearances",
    "animal_perturbations",
    "animal_generations",
    "journey_event_sequences",
];
const SCHEMA_VERSION_KEY: [u8; 1] = [1];
const SCHEMA_VERSION_V0: u32 = 0;

pub fn migrate_fjall_v0(db: &SingleWriterTxDatabase) -> Result<(), String> {
    for name in DATA_KEYSPACES {
        db.keyspace(name, KeyspaceCreateOptions::default)
            .map_err(|err| format!("fjall open {name} keyspace failed: {err}"))?;
    }

    let metadata = db
        .keyspace(SCHEMA_METADATA_KEYSPACE, KeyspaceCreateOptions::default)
        .map_err(|err| format!("fjall open metadata keyspace failed: {err}"))?;
    let snapshot = db.read_tx();
    let version = snapshot
        .get(&metadata, SCHEMA_VERSION_KEY)
        .map_err(|err| format!("fjall read schema version failed: {err}"))?
        .map(|value| decode_schema_version(&value))
        .transpose()?;

    if let Some(version) = version {
        if version != SCHEMA_VERSION_V0 {
            warn!(
                expected_schema_version = SCHEMA_VERSION_V0,
                actual_schema_version = version,
                "fjall schema version mismatch"
            );
        }
    } else {
        let mut tx = db.write_tx().durability(Some(PersistMode::SyncAll));
        tx.insert(
            &metadata,
            SCHEMA_VERSION_KEY,
            SCHEMA_VERSION_V0.to_be_bytes(),
        );
        tx.commit()
            .map_err(|err| format!("fjall migration commit failed: {err}"))?;
    }

    db.persist(PersistMode::SyncAll)
        .map_err(|err| format!("fjall migration persist failed: {err}"))?;
    Ok(())
}

fn decode_schema_version(raw: &[u8]) -> Result<u32, String> {
    let bytes: [u8; 4] = raw.try_into().map_err(|_| {
        format!(
            "fjall schema version must contain exactly 4 bytes, got {}",
            raw.len()
        )
    })?;
    Ok(u32::from_be_bytes(bytes))
}
