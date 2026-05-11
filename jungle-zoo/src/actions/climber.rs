use super::support::define_action;
use crate::state::Climber;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClimberDependency {
    pub tree_height_meters: u32,
}

impl Default for ClimberDependency {
    fn default() -> Self {
        Self {
            tree_height_meters: 12,
        }
    }
}

impl<T> From<&T> for ClimberDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    ClimbTree,
    id = 20,
    dependency = ClimberDependency,
    in = Climber,
    out = String,
    err = String,
    act = |dependency, climber| {
        if climber.is_ape {
            std::future::ready(Ok(format!(
                "climbed to {}m",
                dependency.tree_height_meters
            )))
        } else {
            std::future::ready(Err("only ape climbers can use this route".to_owned()))
        }
    }
);
