use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum BackendError {
    #[error("{0}")]
    Message(String),
}

/// Transport messages sent from runners to external clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RunnerOut {
    ActionInput {
        node_id: u32,
        data: Vec<u8>,
        uuid: Uuid,
    },
    ActionSuccessOutput {
        node_id: u32,
        data: Vec<u8>,
        uuid: Uuid,
    },
    ActionFailureOutput {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimedAnimalPerturbation {
    pub id: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupportedAnimal {
    pub animal_id: u32,
    pub generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OwnerWake {
    pub journey_id: Uuid,
    pub timer_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JourneyStatus {
    Created,
    Alive,
    Stopped,
    Completed,
    Dead,
}

/// Work messages sent from external clients to runners.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireIn {
    CreateJourney {
        namespace: String,
        animal_id: u32,
        seed: Vec<u8>,
    },
    JourneyHistory(Uuid),
    JourneyStatus(Uuid),
    AnimalAppearance(Uuid),
    PerturbAnimal {
        journey_id: Uuid,
        data: Vec<u8>,
    },
    ClaimAnimalPerturbation(Uuid),
    AckAnimalPerturbation {
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
    PollStep {
        namespace: String,
        supported_animals: Vec<SupportedAnimal>,
    },
    PollTimers,
    HistoryEvent(RunnerOut),
}

/// Wire-level messages sent from runners to external clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireOut {
    JourneyCreated(Uuid),
    JourneyHistory(Vec<RunnerOut>),
    JourneyStatus(JourneyStatus),
    AnimalAppearance(Option<Vec<u8>>),
    ClaimedAnimalPerturbation(Option<ClaimedAnimalPerturbation>),
    OwnerWake(Option<OwnerWake>),
    NoAvailableSteps,
    PendingStep(Work),
    Ack,
}
