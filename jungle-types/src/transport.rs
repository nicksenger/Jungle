use uuid::Uuid;

/// Transport messages sent from runners to external clients.
pub enum RunnerOut {
    ActionInput { data: Vec<u8>, uuid: Uuid },
    ActionSuccessOutput { data: Vec<u8>, uuid: Uuid },
    ActionFailureOutput { data: Vec<u8>, uuid: Uuid },
}

/// Work messages sent from external clients to runners.
pub enum Work {
    StartFlow {
        flow_id: Uuid,
        ordinal: u32,
        seed: Vec<u8>,
    },
}
