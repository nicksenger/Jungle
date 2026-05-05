use crate::FlowActions;

/// The innate executable workflow of a `Creature`.
pub trait Instinct: FlowActions {}

impl<T> Instinct for T where T: FlowActions {}
