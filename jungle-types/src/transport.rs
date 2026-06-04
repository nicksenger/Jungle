use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum BackendError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeLifecyclePhase {
    Entered,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeLifecycle {
    pub node_id: u32,
    pub activation_path: Vec<u64>,
    pub phase: NodeLifecyclePhase,
    pub uuid: Uuid,
}

/// Transport messages sent from runners to external clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerOut {
    NodeLifecycle(NodeLifecycle),
    EffectInput {
        node_id: u32,
        data: Vec<u8>,
        uuid: Uuid,
    },
    EffectSuccessOutput {
        node_id: u32,
        data: Vec<u8>,
        uuid: Uuid,
    },
    EffectFailureOutput {
        node_id: u32,
        data: Vec<u8>,
        uuid: Uuid,
    },
    Appearance {
        data: Vec<u8>,
        uuid: Uuid,
    },
    SleepScheduled {
        uuid: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    },
    SleepFired {
        uuid: Uuid,
        timer_id: Uuid,
        fired_at_unix_ms: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyEvent {
    pub sequence_id: u64,
    pub event: RunnerOut,
}

/// Lightweight event payload for journey update subscriptions.
///
/// Effect variants intentionally omit effect payload bytes to keep subscription
/// streams inexpensive for long-running journeys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerUpdateOut {
    NodeLifecycle(NodeLifecycle),
    EffectInput {
        node_id: u32,
        uuid: Uuid,
    },
    EffectSuccessOutput {
        node_id: u32,
        uuid: Uuid,
    },
    EffectFailureOutput {
        node_id: u32,
        uuid: Uuid,
    },
    SleepScheduled {
        uuid: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    },
    SleepFired {
        uuid: Uuid,
        timer_id: Uuid,
        fired_at_unix_ms: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyUpdateEvent {
    pub sequence_id: u64,
    pub event_unix_ms: i64,
    pub event: RunnerUpdateOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedPerturbable {
    pub id: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportedAnimal {
    pub animal_id: u32,
    pub generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerWake {
    pub journey_id: Uuid,
    pub timer_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JourneyStatus {
    Created,
    Alive,
    Stopped,
    Completed,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyRecord {
    pub journey_id: Uuid,
    pub namespace: String,
    pub animal_id: u32,
    pub generation: u32,
    pub status: JourneyStatus,
    pub seed: Vec<u8>,
}

/// Work messages sent from external clients to runners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Work {
    StartJourney {
        journey_id: Uuid,
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
    },
    ResumeJourney {
        journey_id: Uuid,
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
    },
}

/// Wire-level messages sent from external clients to runners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireIn {
    CreateJourney {
        namespace: String,
        animal_id: u32,
        /// Animal generation provided by the client for `animal_id`.
        ///
        /// Servers reject creation when this exceeds their latest known generation.
        generation: u32,
        seed: Vec<u8>,
    },
    JourneyHistory(Uuid),
    JourneyStatus(Uuid),
    ListJourneys {
        namespace: String,
    },
    SubscribeJourneyUpdates {
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    },
    AnimalAppearance(Uuid),
    PerturbAnimal {
        journey_id: Uuid,
        data: Vec<u8>,
    },
    ClaimPerturbable(Uuid),
    AckPerturbable {
        journey_id: Uuid,
        perturbation_id: u64,
    },
    HeartbeatJourneyLease {
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    },
    PollOwnerWake {
        owner_id: Uuid,
    },
    ScheduleSleep {
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    },
    JourneyComplete(Uuid),
    JourneyDead(Uuid),
    PollStep {
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    },
    WaitForWorkerWake {
        owner_id: Uuid,
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
        timeout_ms: u64,
    },
    PollTimers,
    HistoryEvent {
        event: RunnerOut,
        event_unix_ms: i64,
    },
}

/// Wire-level messages sent from runners to external clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireOut {
    JourneyCreated(Uuid),
    JourneyHistory(Vec<RunnerOut>),
    JourneyStatus(JourneyStatus),
    Journeys(Vec<JourneyRecord>),
    JourneyUpdate(JourneyUpdateEvent),
    AnimalAppearance(Option<Vec<u8>>),
    ClaimedPerturbable(Option<ClaimedPerturbable>),
    OwnerWake(Option<OwnerWake>),
    NoAvailableSteps,
    PendingStep(Work),
    Ack,
}
