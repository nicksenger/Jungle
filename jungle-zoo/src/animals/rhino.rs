//! Rhino state model.

use crate::state::{Dermis, Horns, LeafCrop, TemporalState, Torso, VitalReadings};
use jungle_macros::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub forage: LeafCrop,
    pub horns: Horns,
    pub hide: Dermis,
    pub torso: Torso,
    pub temporal: TemporalState,
}
