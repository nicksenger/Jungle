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

/// Work messages sent from external clients to runners.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Work {
    StartFlow {
        flow_id: Uuid,
        ordinal: u32,
        seed: Vec<u8>,
    },
}

/// Wire-level messages sent from external clients to runners.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireIn {
    CreateFlow { ordinal: u32, seed: Vec<u8> },
    PollWork,
    HistoryEvent(RunnerOut),
}

/// Wire-level messages sent from runners to external clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WireOut {
    FlowCreated(Uuid),
    NoWorkAvailable,
    PendingWork(Work),
    Ack,
}
