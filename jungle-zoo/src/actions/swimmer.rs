use super::support::define_action;
use crate::state::Swimmer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwimDependency {
    pub allow_non_finned_swim: bool,
}

impl Default for SwimDependency {
    fn default() -> Self {
        Self {
            allow_non_finned_swim: true,
        }
    }
}

impl<T> From<&T> for SwimDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Swim,
    id = 10,
    dependency = SwimDependency,
    in = Swimmer,
    out = String,
    err = String,
    act = |dependency, swimmer| {
        if swimmer.has_fins || dependency.allow_non_finned_swim {
            std::future::ready(Ok("swam across the enclosure".to_owned()))
        } else {
            std::future::ready(Err("cannot swim without fins in this mode".to_owned()))
        }
    }
);
