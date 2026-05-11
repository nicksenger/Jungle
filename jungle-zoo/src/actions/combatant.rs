use super::support::define_action;
use crate::state::{Combatant, Weapon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatDependency {
    pub avoid_conflict: bool,
}

impl Default for CombatDependency {
    fn default() -> Self {
        Self {
            avoid_conflict: false,
        }
    }
}

impl<T> From<&T> for CombatDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Defend,
    id = 60,
    dependency = CombatDependency,
    in = Combatant,
    out = String,
    err = String,
    act = |dependency, combatant| {
        if dependency.avoid_conflict {
            return std::future::ready(Ok("de-escalated and retreated".to_owned()));
        }

        let method = match combatant.weapons {
            Weapon::Teeth => "bite",
            Weapon::Claws => "slash",
            Weapon::Tools => "tool strike",
            Weapon::Mass => "body charge",
        };

        std::future::ready(Ok(format!("defended using {method}")))
    }
);
