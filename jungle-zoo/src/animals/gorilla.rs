//! Gorilla state model.

use crate::state::{FruitMeal, Hands, NervousSystem, TemporalState, VitalReadings};
use jungle_sdk::Optic;

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub age: u32,
    pub vitals: VitalReadings,
    pub meal: FruitMeal,
    pub hands: Hands,
    pub nervous_system: NervousSystem,
    pub temporal: TemporalState,
}
