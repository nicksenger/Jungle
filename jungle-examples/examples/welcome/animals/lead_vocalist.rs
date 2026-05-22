use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::U1;

use crate::instrumentation::{Sing, VocalsArticulation};

use super::DecrementCounter;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LeadVocalistState {
    #[jungle(focus)]
    articulation: VocalsArticulation,
    intro_pickup_remaining: u8,
}

impl Default for LeadVocalistState {
    fn default() -> Self {
        Self {
            articulation: VocalsArticulation::SirenScream,
            intro_pickup_remaining: 1,
        }
    }
}

pub type LeadVocalistSeed = ();

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroPickupRemaining;
impl LoopCondition<LeadVocalistState> for IntroPickupRemaining {
    type Arg = ();

    fn should_continue(state: &LeadVocalistState) -> bool {
        state.intro_pickup_remaining > 0
    }
}

pub struct IntroNeedsPickup;
impl<In> Condition<(LeadVocalistState, In)> for IntroNeedsPickup {
    fn choose(input: &(LeadVocalistState, In)) -> bool {
        input.0.intro_pickup_remaining == 0
    }
}

type IntroPickupCounter = Lens<LeadVocalistState, U1>;
pub type AdvanceIntroPickup = DecrementCounter<IntroPickupCounter>;

#[derive(Flow)]
pub struct LeadVocalIntro(
    Transparent<IntroSectionMeta, IntroBreath>,
    Transparent<IntroSectionMeta, IntroPickupLoop>,
    Transparent<IntroSectionMeta, Conditional<IntroNeedsPickup, IntroRelease, IntroRest>>,
);

#[derive(Flow)]
pub struct IntroBreath(Transparent<IntroSectionMeta, IntroBreathPhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroBreathPhrase(Step<Sing<58, 1, 192>>);

#[derive(Flow)]
pub struct IntroPickupLoop(While<IntroPickupRemaining, IntroPickupBody>);

#[derive(Flow)]
pub struct IntroPickupBody(
    Transparent<IntroSectionMeta, IntroPickupPhrase>,
    Transparent<IntroSectionMeta, Step<AdvanceIntroPickup>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroPickupPhrase(Step<Sing<58, 192, 192>>);

#[derive(Flow)]
pub struct IntroRest(Transparent<IntroSectionMeta, IntroRestPhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroRestPhrase(Step<Sing<58, 1, 192>>);

#[derive(Flow)]
pub struct IntroRelease(Transparent<IntroSectionMeta, IntroReleasePhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroReleasePhrase(Step<Sing<58, 1, 192>>);
