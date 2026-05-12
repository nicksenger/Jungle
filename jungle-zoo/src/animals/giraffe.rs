//! Giraffe state model.

use crate::state::{LeafCrop, TemporalState, Tongue, Torso, VitalReadings};
use jungle_macros::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub forage: LeafCrop,
    pub tongue: Tongue,
    pub torso: Torso,
    pub temporal: TemporalState,
}
