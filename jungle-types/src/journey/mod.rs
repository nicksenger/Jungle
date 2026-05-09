use crate::FlowActions;

/// The innate executable workflow of a `Animal`.
pub trait Journey: FlowActions {}

impl<T> Journey for T where T: FlowActions {}
