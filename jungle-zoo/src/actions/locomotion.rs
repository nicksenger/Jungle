use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocomotionDependency {
    pub swim_boost: u16,
    pub walk_stride_bonus_cm: u16,
    pub run_burst_bonus_cm: u16,
    pub charge_force_bonus: u16,
}

impl Default for LocomotionDependency {
    fn default() -> Self {
        Self {
            swim_boost: 4,
            walk_stride_bonus_cm: 5,
            run_burst_bonus_cm: 14,
            charge_force_bonus: 20,
        }
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

define_action!(
    Walk,
    id = 23,
    dependency = LocomotionDependency,
    in = (u16, u16),
    out = u16,
    err = String,
    act = |dependency, (fore_stride_cm, hind_stride_cm)| {
        let base = fore_stride_cm.saturating_add(hind_stride_cm) / 2;
        std::future::ready(Ok(base.saturating_add(dependency.walk_stride_bonus_cm)))
    }
);

define_action!(
    Run,
    id = 24,
    dependency = LocomotionDependency,
    in = (u16, u8),
    out = u16,
    err = String,
    act = |dependency, (lung_capacity_liters, stress)| {
        let stress_penalty = u16::from(stress / 8);
        let run_score = lung_capacity_liters
            .saturating_add(dependency.run_burst_bonus_cm)
            .saturating_sub(stress_penalty);
        std::future::ready(Ok(run_score))
    }
);

define_action!(
    Charge,
    id = 25,
    dependency = LocomotionDependency,
    in = (u16, u8),
    out = u16,
    err = String,
    act = |dependency, (horn_length_cm, stress)| {
        if horn_length_cm == 0 {
            return std::future::ready(Err("insufficient horn length for charge".to_owned()));
        }
        let force = horn_length_cm
            .saturating_add(dependency.charge_force_bonus)
            .saturating_add(u16::from(stress / 4));
        std::future::ready(Ok(force))
    }
);
