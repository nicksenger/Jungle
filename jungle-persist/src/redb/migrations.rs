//! redb persistence migrations.

use super::RedbStore;
use crate::Result;
use redb::{ReadableTable, TableDefinition};
use tracing::warn;

const SCHEMA_METADATA_TABLE: TableDefinition<u8, u32> =
    TableDefinition::new("jungle_schema_metadata");
const FLOWS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("flows");
const EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");

impl RedbStore {
    pub(super) async fn migrate_v0(&self) -> Result<()> {
        let tx = self
            .db
            .begin_write()
            .map_err(|err| {
                crate::PersistenceError::Message(format!("redb migration begin failed: {err}"))
            })?;

        {
            let mut metadata = tx.open_table(SCHEMA_METADATA_TABLE).map_err(|err| {
                crate::PersistenceError::Message(format!("redb open metadata table failed: {err}"))
            })?;

            let version = metadata
                .get(1)
                .map_err(|err| {
                    crate::PersistenceError::Message(format!("redb read schema version failed: {err}"))
                })?
                .map(|version| version.value());

            if let Some(version) = version {
                if version != 0 {
                    warn!(
                        expected_schema_version = 0,
                        actual_schema_version = version,
                        "redb schema version mismatch"
                    );
                }
            } else {
                metadata.insert(1, 0).map_err(|err| {
                    crate::PersistenceError::Message(format!(
                        "redb initialize schema version failed: {err}"
                    ))
                })?;
            }
        }

        tx.open_table(FLOWS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!("redb open flows table failed: {err}"))
        })?;
        tx.open_table(EVENTS_TABLE).map_err(|err| {
            crate::PersistenceError::Message(format!("redb open events table failed: {err}"))
        })?;

        tx.commit()
            .map_err(|err| {
                crate::PersistenceError::Message(format!("redb migration commit failed: {err}"))
            })?;

        Ok(())
    }
}
