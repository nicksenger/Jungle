use super::support::{define_action, maybe_delay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehavioralDependency {
    pub breath_recovery: u16,
    pub sleep_recovery: u16,
    pub sound_volume_bias: u8,
    pub relax_stress_drop: u8,
    pub socialize_energy_cost: u16,
}

impl Default for BehavioralDependency {
    fn default() -> Self {
        Self {
            breath_recovery: 1,
            sleep_recovery: 16,
            sound_volume_bias: 2,
            relax_stress_drop: 12,
            socialize_energy_cost: 3,
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
    Rest,
    id = 2,
    dependency = BehavioralDependency,
    in = u16,
    out = u16,
    err = String,
    act = |dependency, energy| {
        async move {
            maybe_delay().await;
            Ok(energy.saturating_add(dependency.sleep_recovery))
        }
    }
);

/// Backwards-compatibility alias for prior naming.
pub type Sleep = Rest;

define_action!(
    MakeSound,
    id = 3,
    dependency = BehavioralDependency,
    in = (String, u8),
    out = String,
    err = String,
    act = |dependency, (kind, intensity)| {
        async move {
            maybe_delay().await;
            let volume = intensity.saturating_add(dependency.sound_volume_bias);
            Ok(format!("{kind} at volume {volume}"))
        }
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
        async move {
            maybe_delay().await;
            let rhythm = if opposable_thumb { 4 } else { 2 };
            Ok(stress.saturating_add(rhythm))
        }
    }
);

define_action!(
    Relax,
    id = 6,
    dependency = BehavioralDependency,
    in = (u16, u8),
    out = (u16, u8),
    err = String,
    act = |dependency, (energy, stress)| {
        let calmer_stress = stress.saturating_sub(dependency.relax_stress_drop);
        let restored_energy = energy.saturating_add(2);
        std::future::ready(Ok((restored_energy, calmer_stress)))
    }
);

define_action!(
    Socialize,
    id = 7,
    dependency = BehavioralDependency,
    in = (u8, bool),
    out = String,
    err = String,
    act = |dependency, (stress, can_rotate_ears)| {
        if stress > 95 {
            return std::future::ready(Err("too stressed to socialize".to_owned()));
        }
        let style = if can_rotate_ears { "expressive" } else { "reserved" };
        std::future::ready(Ok(format!(
            "{style} social interaction (energy cost {})",
            dependency.socialize_energy_cost
        )))
    }
);
