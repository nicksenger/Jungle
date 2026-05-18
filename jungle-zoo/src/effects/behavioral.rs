use super::support::maybe_delay;
use jungle_sdk::prelude::*;

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

pub struct Rest;

#[jungle::effect(id = 2)]
impl<J> Effect<J> for Rest {
    type In = u16;
    type Out = u16;
    type Err = String;

    fn effect(
        _jungle: &J,
        energy: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = BehavioralDependency::default();
        async move {
            maybe_delay().await;
            Ok(energy.saturating_add(dependency.sleep_recovery))
        }
    }
}

pub struct MakeSound;

#[jungle::effect(id = 3)]
impl<J> Effect<J> for MakeSound {
    type In = (String, u8);
    type Out = String;
    type Err = String;

    fn effect(
        _jungle: &J,
        (kind, intensity): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = BehavioralDependency::default();
        async move {
            maybe_delay().await;
            let volume = intensity.saturating_add(dependency.sound_volume_bias);
            Ok(format!("{kind} at volume {volume}"))
        }
    }
}

pub struct ChestBeat;

#[jungle::effect(id = 5)]
impl<J> Effect<J> for ChestBeat {
    type In = (u8, bool);
    type Out = u8;
    type Err = String;

    #[allow(clippy::manual_async_fn)]
    fn effect(
        _jungle: &J,
        (stress, opposable_thumb): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            maybe_delay().await;
            let rhythm = if opposable_thumb { 4 } else { 2 };
            Ok(stress.saturating_add(rhythm))
        }
    }
}
