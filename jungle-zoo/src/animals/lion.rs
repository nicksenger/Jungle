//! Lion state model.

use crate::state::{Ears, PreyCut, Skeleton, TemporalState, VitalReadings};
use jungle_sdk::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub prey: PreyCut,
    pub skeleton: Skeleton,
    pub ears: Ears,
    pub temporal: TemporalState,
}
