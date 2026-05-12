use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehavioralDependency {
    pub breath_recovery: u16,
    pub sleep_recovery: u16,
    pub sound_volume_bias: u8,
}

impl Default for BehavioralDependency {
    fn default() -> Self {
        Self {
            breath_recovery: 1,
            sleep_recovery: 16,
            sound_volume_bias: 2,
        }
    }
}

impl<T> From<&T> for BehavioralDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Breathe,
    id = 1,
    dependency = BehavioralDependency,
    in = u16,
    out = u16,
    err = String,
    act = |dependency, energy| {
        std::future::ready(Ok(energy.saturating_add(dependency.breath_recovery)))
    }
);

define_action!(
    Sleep,
    id = 2,
    dependency = BehavioralDependency,
    in = u16,
    out = u16,
    err = String,
    act = |dependency, energy| {
        std::future::ready(Ok(energy.saturating_add(dependency.sleep_recovery)))
    }
);

define_action!(
    MakeSound,
    id = 3,
    dependency = BehavioralDependency,
    in = (String, u8),
    out = String,
    err = String,
    act = |dependency, (kind, intensity)| {
        let volume = intensity.saturating_add(dependency.sound_volume_bias);
        std::future::ready(Ok(format!("{kind} at volume {volume}")))
    }
);

define_action!(
    Roar,
    id = 4,
    dependency = BehavioralDependency,
    in = (u16, u8),
    out = String,
    err = String,
    act = |_dependency, (lung_capacity_liters, stress)| {
        std::future::ready(Ok(format!(
            "territorial roar (capacity={lung_capacity_liters}, stress={stress})"
        )))
    }
);

define_action!(
    ChestBeat,
    id = 5,
    dependency = BehavioralDependency,
    in = (u8, bool),
    out = u8,
    err = String,
    act = |_dependency, (stress, opposable_thumb)| {
        let rhythm = if opposable_thumb { 4 } else { 2 };
        std::future::ready(Ok(stress.saturating_add(rhythm)))
    }
);
