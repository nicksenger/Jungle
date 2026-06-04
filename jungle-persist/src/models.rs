//! Persistence-layer data models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersion {
    V0,
}

pub const SCHEMA_VERSION: SchemaVersion = SchemaVersion::V0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journey {
    pub id: Uuid,
    pub namespace: String,
    pub animal_id: u32,
    pub generation: u32,
    pub status: JourneyStatus,
    pub seed: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub journey_id: Uuid,
    pub sequence_id: u64,
    pub kind: EventKind,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub kind: StepKind,
    pub status: StepStatus,
    pub expiry: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    NodeLifecycle,
    EffectInput,
    EffectSuccessOutput,
    EffectFailureOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    StartJourney,
    ResumeJourney,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Available,
    Claimed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JourneyStatus {
    Created,
    Alive,
    Stopped,
    Completed,
    Dead,
}
