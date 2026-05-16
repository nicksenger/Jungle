use super::support::{define_effect, maybe_delay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehavioralDependency {
    pub sleep_recovery: u16,
    pub sound_volume_bias: u8,
}

impl Default for BehavioralDependency {
    fn default() -> Self {
        Self {
            sleep_recovery: 16,
            sound_volume_bias: 2,
        }
    }
}

define_effect!(
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

define_effect!(
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

define_effect!(
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
