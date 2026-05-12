use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocomotionDependency {
    pub swim_boost: u16,
}

impl Default for LocomotionDependency {
    fn default() -> Self {
        Self { swim_boost: 4 }
    }
}

impl<T> From<&T> for LocomotionDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Swim,
    id = 20,
    dependency = LocomotionDependency,
    in = u16,
    out = u16,
    err = String,
    act = |dependency, lung_capacity_liters| {
        std::future::ready(Ok(lung_capacity_liters.saturating_add(dependency.swim_boost)))
    }
);

define_action!(
    ClimbTree,
    id = 21,
    dependency = LocomotionDependency,
    in = bool,
    out = String,
    err = String,
    act = |_dependency, opposable_thumb| {
        if opposable_thumb {
            std::future::ready(Ok("climbed canopy route".to_owned()))
        } else {
            std::future::ready(Err("cannot stabilize grip".to_owned()))
        }
    }
);

define_action!(
    DeathRoll,
    id = 22,
    dependency = LocomotionDependency,
    in = (bool, u8),
    out = u8,
    err = String,
    act = |_dependency, (has_tail, stress)| {
        if !has_tail {
            return std::future::ready(Err("tail required for death roll".to_owned()));
        }
        std::future::ready(Ok(stress.saturating_add(10)))
    }
);
