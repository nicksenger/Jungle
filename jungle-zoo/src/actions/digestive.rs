use super::support::{define_action, maybe_delay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestiveDependency {
    pub chew_efficiency: u8,
    pub hunt_bonus: u16,
    pub peel_bonus: u16,
    pub shell_crack_bonus: u8,
    pub tear_force_bonus: u16,
}

impl Default for DigestiveDependency {
    fn default() -> Self {
        Self {
            chew_efficiency: 3,
            hunt_bonus: 12,
            peel_bonus: 6,
            shell_crack_bonus: 8,
            tear_force_bonus: 15,
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
        async move {
            maybe_delay().await;
            Ok(energy.saturating_add(u16::from(dependency.chew_efficiency)))
        }
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
        async move {
            maybe_delay().await;
            if opposable_thumb {
                Ok("used stick to extract food".to_owned())
            } else {
                Err("no opposable thumb for tool use".to_owned())
            }
        }
    }
);

define_action!(
    StripLeaves,
    id = 15,
    dependency = DigestiveDependency,
    in = (u8, u16),
    out = u16,
    err = String,
    act = |dependency, (toughness, mass_g)| {
        let penalty = u16::from(toughness / 4);
        let edible = mass_g
            .saturating_add(u16::from(dependency.chew_efficiency))
            .saturating_sub(penalty);
        std::future::ready(Ok(edible))
    }
);

define_action!(
    PeelFruit,
    id = 16,
    dependency = DigestiveDependency,
    in = (u8, u16),
    out = u16,
    err = String,
    act = |dependency, (rind_thickness_mm, flesh_mass_g)| {
        async move {
            maybe_delay().await;
            let peel_cost = u16::from(rind_thickness_mm).saturating_mul(2);
            let edible = flesh_mass_g
                .saturating_add(dependency.peel_bonus)
                .saturating_sub(peel_cost);
            Ok(edible)
        }
    }
);

define_action!(
    CrackShell,
    id = 17,
    dependency = DigestiveDependency,
    in = (bool, u8),
    out = bool,
    err = String,
    act = |dependency, (has_shell, bite_strength)| {
        if !has_shell {
            return std::future::ready(Ok(true));
        }
        let success = bite_strength.saturating_add(dependency.shell_crack_bonus) >= 10;
        if success {
            std::future::ready(Ok(true))
        } else {
            std::future::ready(Err("shell too hard to crack".to_owned()))
        }
    }
);

define_action!(
    TearMeat,
    id = 18,
    dependency = DigestiveDependency,
    in = (u16, u8),
    out = u16,
    err = String,
    act = |dependency, (muscle_mass_g, hide_thickness_mm)| {
        let hide_penalty = u16::from(hide_thickness_mm).saturating_mul(2);
        let exposed = muscle_mass_g
            .saturating_add(dependency.tear_force_bonus)
            .saturating_sub(hide_penalty);
        std::future::ready(Ok(exposed))
    }
);
