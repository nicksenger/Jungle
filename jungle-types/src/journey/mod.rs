use crate::FlowActions;

/// The innate executable workflow of a `Anima`.
pub trait Journey: FlowActions {}

impl<T> Journey for T where T: FlowActions {}
