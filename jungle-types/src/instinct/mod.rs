use crate::FlowActions;

/// The innate executable workflow of a `Anima`.
pub trait Instinct: FlowActions {}

impl<T> Instinct for T where T: FlowActions {}
