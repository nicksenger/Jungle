use super::support::define_action;
use crate::state::Reptile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReptileDependency {
    pub bask_minutes: u16,
    pub nesting_season: bool,
}

impl Default for ReptileDependency {
    fn default() -> Self {
        Self {
            bask_minutes: 45,
            nesting_season: true,
        }
    }
}

impl<T> From<&T> for ReptileDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Bask,
    id = 40,
    dependency = ReptileDependency,
    in = Reptile,
    out = String,
    err = String,
    act = |dependency, reptile| {
        let _ = reptile;
        std::future::ready(Ok(format!("basked for {} minutes", dependency.bask_minutes)))
    }
);

define_action!(
    LayEggs,
    id = 41,
    dependency = ReptileDependency,
    in = Reptile,
    out = u8,
    err = String,
    act = |dependency, reptile| {
        let _ = reptile;
        if dependency.nesting_season {
            std::future::ready(Ok(18))
        } else {
            std::future::ready(Err("not in nesting season".to_owned()))
        }
    }
);

define_action!(
    DeathRoll,
    id = 42,
    dependency = ReptileDependency,
    in = Reptile,
    out = String,
    err = String,
    act = |_dependency, reptile| {
        let _ = reptile;
        std::future::ready(Ok("executed a death roll".to_owned()))
    }
);
