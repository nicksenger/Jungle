//! Crocodile state model.

use crate::state::{PreyCut, Scales, TemporalState, VitalReadings};
use jungle_macros::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub prey: PreyCut,
    pub scales: Scales,
    pub temporal: TemporalState,
}
