use crate::FlowActions;

/// The innate executable workflow of an `Animal`.
pub trait Instinct: FlowActions {}

impl<T> Instinct for T where T: FlowActions {}
