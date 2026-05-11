use super::support::define_action;
use crate::state::Metabolism;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetabolismDependency {
    pub digest_gain: u64,
    pub breath_cost: u64,
    pub sleep_recovery: u64,
}

impl Default for MetabolismDependency {
    fn default() -> Self {
        Self {
            digest_gain: 20,
            breath_cost: 1,
            sleep_recovery: 30,
        }
    }
}

impl<T> From<&T> for MetabolismDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Breathe,
    id = 1,
    dependency = MetabolismDependency,
    in = Metabolism,
    out = Metabolism,
    err = String,
    act = |dependency, metabolism| {
        let mut next = metabolism;
        next.energy = next.energy.saturating_sub(dependency.breath_cost);
        next.is_hungry = next.energy < 30;
        next.is_sleepy = next.energy < 10;
        std::future::ready(Ok(next))
    }
);

define_action!(
    Eat,
    id = 2,
    dependency = MetabolismDependency,
    in = Metabolism,
    out = Metabolism,
    err = String,
    act = |dependency, metabolism| {
        let mut next = metabolism;
        next.energy = next.energy.saturating_add(dependency.digest_gain);
        next.is_hungry = next.energy < 40;
        next.is_sleepy = next.energy < 15;
        std::future::ready(Ok(next))
    }
);

define_action!(
    Sleep,
    id = 3,
    dependency = MetabolismDependency,
    in = Metabolism,
    out = Metabolism,
    err = String,
    act = |dependency, metabolism| {
        let mut next = metabolism;
        next.energy = next.energy.saturating_add(dependency.sleep_recovery);
        next.is_sleepy = next.energy < 20;
        next.is_hungry = next.energy < 25;
        std::future::ready(Ok(next))
    }
);
