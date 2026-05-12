use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeciesDependency {
    pub intimidation_scale: u8,
}

impl Default for SpeciesDependency {
    fn default() -> Self {
        Self {
            intimidation_scale: 3,
        }
    }
}

impl<T> From<&T> for SpeciesDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    GorillaChestBeat,
    id = 40,
    dependency = SpeciesDependency,
    in = (u8, bool),
    out = String,
    err = String,
    act = |dependency, (stress, opposable_thumb)| {
        if !opposable_thumb {
            return std::future::ready(Err("gorilla chest beat requires free palms".to_owned()));
        }
        let score = stress.saturating_add(dependency.intimidation_scale);
        std::future::ready(Ok(format!("gorilla chest beat score={score}")))
    }
);

define_action!(
    LionRoar,
    id = 41,
    dependency = SpeciesDependency,
    in = (u16, u8),
    out = String,
    err = String,
    act = |dependency, (lung_capacity_liters, stress)| {
        let score = u16::from(dependency.intimidation_scale) + lung_capacity_liters + u16::from(stress / 10);
        std::future::ready(Ok(format!("lion roar score={score}")))
    }
);

define_action!(
    CrocodileDeathRoll,
    id = 42,
    dependency = SpeciesDependency,
    in = (bool, u8),
    out = String,
    err = String,
    act = |dependency, (has_tail, stress)| {
        if !has_tail {
            return std::future::ready(Err("crocodile needs tail leverage".to_owned()));
        }
        let score = stress.saturating_add(dependency.intimidation_scale.saturating_mul(2));
        std::future::ready(Ok(format!("crocodile death roll score={score}")))
    }
);
