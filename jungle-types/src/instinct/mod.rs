use crate::WorkflowActions;

/// The innate executable workflow of an `Animal`.
pub trait Instinct: WorkflowActions {}

impl<T> Instinct for T where T: WorkflowActions {}
