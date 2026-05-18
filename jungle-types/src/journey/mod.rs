use crate::JourneyEffects;

/// The innate executable workflow of an `Animal`, expressed as a type-level Rust DSL.
pub trait Journey: JourneyEffects {}

impl<T> Journey for T where T: JourneyEffects {}
