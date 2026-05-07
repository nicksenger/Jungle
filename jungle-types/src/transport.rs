use uuid::Uuid;

/// Transport messages sent from runners to external clients.
pub enum ClientIn {
    ActionInput { data: Vec<u8>, uuid: Uuid },
    ActionOutput { data: Vec<u8>, uuid: Uuid },
}

/// Transport messages sent from external clients back to runners.
pub enum ClientOut {}
