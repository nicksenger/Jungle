use crate::FlowEffects;

/// The innate executable workflow of an `Animal`, expressed as a type-level Rust DSL.
pub trait Journey: FlowEffects {}

impl<T> Journey for T where T: FlowEffects {}
