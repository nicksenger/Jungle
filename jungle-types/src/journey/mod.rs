use crate::FlowActions;

/// The innate executable workflow of an `Animal`, expressed as a type-level Rust DSL.
pub trait Journey: FlowActions {}

impl<T> Journey for T where T: FlowActions {}
