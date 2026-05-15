use super::support::{define_effect, maybe_delay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestiveDependency {
    pub chew_efficiency: u8,
    pub peel_bonus: u16,
}

impl Default for DigestiveDependency {
    fn default() -> Self {
        Self {
            chew_efficiency: 3,
            peel_bonus: 6,
        }
    }
}

impl<T> From<&T> for DigestiveDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_effect!(
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

define_effect!(
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

define_effect!(
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
