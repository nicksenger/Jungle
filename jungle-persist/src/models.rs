//! Persistence-layer data models.

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersion {
    V0,
}

pub const SCHEMA_VERSION: SchemaVersion = SchemaVersion::V0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub id: Uuid,
    pub ordinal: u32,
    pub seed: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub flow_id: Uuid,
    pub sequence_id: u64,
    pub kind: EventKind,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    ActionInput,
    ActionSuccessOutput,
    ActionFailureOutput,
}
