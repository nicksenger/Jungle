use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum BackendError {
    #[error("{0}")]
    Message(String),
}

/// Transport messages sent from runners to external clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RunnerOut {
    ActionInput { data: Vec<u8>, uuid: Uuid },
    ActionSuccessOutput { data: Vec<u8>, uuid: Uuid },
    ActionFailureOutput { data: Vec<u8>, uuid: Uuid },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JourneyStatus {
    Created,
    Alive,
    Stopped,
    Completed,
    Dead,
}

/// Step messages sent from external clients to runners.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Step {
    StartJourney {
        journey_id: Uuid,
        ordinal: u32,
        seed: Vec<u8>,
    },
}

/// Wire-level messages sent from external clients to runners.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireIn {
    CreateJourney { ordinal: u32, seed: Vec<u8> },
    JourneyStatus(Uuid),
    JourneyComplete(Uuid),
    PollStep,
    HistoryEvent(RunnerOut),
}

/// Wire-level messages sent from runners to external clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireOut {
    JourneyCreated(Uuid),
    JourneyStatus(JourneyStatus),
    NoAvailableSteps,
    PendingStep(Step),
    Ack,
}
