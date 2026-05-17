use super::support::maybe_delay;

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

pub struct Eat;

#[jungle_sdk::effect(id = 10)]
impl<J> jungle_sdk::types::Effect<J> for Eat {
    type In = u16;
    type Out = u16;
    type Err = String;

    fn effect(
        _jungle: &J,
        energy: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = DigestiveDependency::default();
        async move {
            maybe_delay().await;
            Ok(energy.saturating_add(u16::from(dependency.chew_efficiency)))
        }
    }
}

pub struct UseTool;

#[jungle_sdk::effect(id = 14)]
impl<J> jungle_sdk::types::Effect<J> for UseTool {
    type In = bool;
    type Out = String;
    type Err = String;

    fn effect(
        _jungle: &J,
        opposable_thumb: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            maybe_delay().await;
            if opposable_thumb {
                Ok("used stick to extract food".to_owned())
            } else {
                Err("no opposable thumb for tool use".to_owned())
            }
        }
    }
}

pub struct PeelFruit;

#[jungle_sdk::effect(id = 16)]
impl<J> jungle_sdk::types::Effect<J> for PeelFruit {
    type In = (u8, u16);
    type Out = u16;
    type Err = String;

    fn effect(
        _jungle: &J,
        (rind_thickness_mm, flesh_mass_g): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = DigestiveDependency::default();
        async move {
            maybe_delay().await;
            let peel_cost = u16::from(rind_thickness_mm).saturating_mul(2);
            let edible = flesh_mass_g
                .saturating_add(dependency.peel_bonus)
                .saturating_sub(peel_cost);
            Ok(edible)
        }
    }
}
