use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestiveDependency {
    pub chew_efficiency: u8,
    pub hunt_bonus: u16,
}

impl Default for DigestiveDependency {
    fn default() -> Self {
        Self {
            chew_efficiency: 3,
            hunt_bonus: 12,
        }
    }
}

impl<T> From<&T> for DigestiveDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Eat,
    id = 10,
    dependency = DigestiveDependency,
    in = u16,
    out = u16,
    err = String,
    act = |dependency, energy| {
        std::future::ready(Ok(energy.saturating_add(u16::from(dependency.chew_efficiency))))
    }
);

define_action!(
    Forage,
    id = 11,
    dependency = DigestiveDependency,
    in = bool,
    out = u16,
    err = String,
    act = |dependency, has_fermentation_chamber| {
        let gain = if has_fermentation_chamber {
            u16::from(dependency.chew_efficiency).saturating_mul(2)
        } else {
            u16::from(dependency.chew_efficiency)
        };
        std::future::ready(Ok(gain))
    }
);

define_action!(
    Graze,
    id = 12,
    dependency = DigestiveDependency,
    in = u16,
    out = u16,
    err = String,
    act = |_dependency, stomach_chambers| {
        let gain = stomach_chambers.saturating_mul(2);
        std::future::ready(Ok(gain))
    }
);

define_action!(
    Hunt,
    id = 13,
    dependency = DigestiveDependency,
    in = u8,
    out = u16,
    err = String,
    act = |dependency, stress| {
        if stress > 95 {
            return std::future::ready(Err("stress too high to hunt".to_owned()));
        }
        std::future::ready(Ok(dependency.hunt_bonus.saturating_add(u16::from(stress / 4))))
    }
);

define_action!(
    UseTool,
    id = 14,
    dependency = DigestiveDependency,
    in = bool,
    out = String,
    err = String,
    act = |_dependency, opposable_thumb| {
        if opposable_thumb {
            std::future::ready(Ok("used stick to extract food".to_owned()))
        } else {
            std::future::ready(Err("no opposable thumb for tool use".to_owned()))
        }
    }
);
