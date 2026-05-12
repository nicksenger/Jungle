//! Elephant state model.

use crate::state::{Ears, LeafCrop, TemporalState, Trunk, Tusks, VitalReadings};
use jungle_sdk::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub forage: LeafCrop,
    pub trunk: Trunk,
    pub tusks: Tusks,
    pub ears: Ears,
    pub temporal: TemporalState,
}
