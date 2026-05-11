use super::support::define_action;
use crate::state::{CrocodileState, GorillaState, LionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeciesDependency {
    pub intimidation_bonus: u8,
}

impl Default for SpeciesDependency {
    fn default() -> Self {
        Self {
            intimidation_bonus: 2,
        }
    }
}

impl<T> From<&T> for SpeciesDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    LionRoar,
    id = 70,
    dependency = SpeciesDependency,
    in = LionState,
    out = String,
    err = String,
    act = |dependency, lion| {
        let species = lion.carnivore.base.species;
        std::future::ready(Ok(format!(
            "{species} roars with intensity {}",
            dependency.intimidation_bonus
        )))
    }
);

define_action!(
    GorillaChestBeat,
    id = 71,
    dependency = SpeciesDependency,
    in = GorillaState,
    out = String,
    err = String,
    act = |dependency, gorilla| {
        let species = gorilla.omnivore.base.species;
        std::future::ready(Ok(format!(
            "{species} chest-beats {} times",
            dependency.intimidation_bonus
        )))
    }
);

define_action!(
    CrocodileDeathRoll,
    id = 72,
    dependency = SpeciesDependency,
    in = CrocodileState,
    out = String,
    err = String,
    act = |dependency, croc| {
        let species = croc.carnivore.base.species;
        std::future::ready(Ok(format!(
            "{species} spins in a death roll (x{})",
            dependency.intimidation_bonus
        )))
    }
);
