use jungle_sdk::prelude::*;

use crate::effect::Rest;
use crate::instrumentation::{
    phonemes_from_text, Generate as LaneGenerate, Lyrics, Sing as LaneSing, VocalsArticulation,
};

use super::{Double, LeadVocalist};

const LEAD_VOCALS_LANE_ID: u32 = <<LeadVocalist as Animal>::Id as AnimalIdValue>::U32;
type Sing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneSing<NOTE, NOTE_TICK, REST_TICK, LEAD_VOCALS_LANE_ID>;
type Generate<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneGenerate<NOTE, NOTE_TICK, REST_TICK, LEAD_VOCALS_LANE_ID>;

type Generate68Tick = Step<Generate<68, 96, 96>>;
type Sing68Tick = Step<Sing<68, 96, 96>>;
type Generate68Hold = Step<Generate<68, 192, 192>>;
type Sing68Hold = Step<Sing<68, 192, 192>>;
type Sing63Hold = Step<Sing<63, 192, 192>>;

#[derive(Optic, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeadVocalistState {
    #[jungle(focus)]
    articulation: VocalsArticulation,
    intro_pickup_remaining: u8,
    pub lyrics: Lyrics,
}

impl Default for LeadVocalistState {
    fn default() -> Self {
        Self {
            articulation: VocalsArticulation::Clean,
            intro_pickup_remaining: 1,
            lyrics: Lyrics {
                phonemes: vec![
                    phonemes_from_text("games"),
                    phonemes_from_text("and"),
                    phonemes_from_text("fun"),
                    phonemes_from_text("got"),
                    phonemes_from_text("weve"),
                    phonemes_from_text("gle"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("well"),
                ],
            },
        }
    }
}

pub type LeadVocalistSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 20_352;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<LEAD_VOCALS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct UseLeadVocalPickup;
impl Condition<(LeadVocalistState, ())> for UseLeadVocalPickup {
    fn choose((state, _): &(LeadVocalistState, ())) -> bool {
        state.intro_pickup_remaining > 0
    }
}

pub struct ConsumeLeadVocalPickup;
#[jungle::act]
impl Act for ConsumeLeadVocalPickup {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead vocal pickup consume should complete");
        state.intro_pickup_remaining = state.intro_pickup_remaining.saturating_sub(1);
    }
}

pub struct MergeLeadVocalPickupChoice;
#[jungle::act]
impl Act for MergeLeadVocalPickupChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        _state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead vocal pickup branch merge should complete");
    }
}

#[derive(Flow)]
pub struct LeadVocalPickupBranch(
    Transparent<IntroSectionMeta, LeadVocalSection01>,
    Step<ConsumeLeadVocalPickup>,
);

#[derive(Flow)]
pub struct LeadVocalMainBranch(Step<ConsumeLeadVocalPickup>);

#[derive(Flow)]
pub struct LeadVocalIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Conditional<UseLeadVocalPickup, LeadVocalPickupBranch, LeadVocalMainBranch>,
    Step<MergeLeadVocalPickupChoice>,
    Transparent<IntroSectionMeta, LeadVocalSection02>,
    Transparent<IntroSectionMeta, LeadVocalSection03>,
);

#[derive(Flow)]
pub struct LeadVocalSection01(
    Transparent<IntroSectionMeta, LeadVocalPart01>,
    Transparent<IntroSectionMeta, LeadVocalPart02>,
    Transparent<IntroSectionMeta, LeadVocalPart03>,
    Transparent<IntroSectionMeta, LeadVocalPart04>,
    Transparent<IntroSectionMeta, LeadVocalPart05>,
    Transparent<IntroSectionMeta, LeadVocalPart06>,
);

#[derive(Flow)]
pub struct LeadVocalSection02(
    Transparent<IntroSectionMeta, LeadVocalPart07>,
    Transparent<IntroSectionMeta, LeadVocalPart08>,
    Transparent<IntroSectionMeta, LeadVocalPart09>,
    Transparent<IntroSectionMeta, LeadVocalPart10>,
    Transparent<IntroSectionMeta, LeadVocalPart11>,
    Transparent<IntroSectionMeta, LeadVocalPart12>,
);

#[derive(Flow)]
pub struct LeadVocalSection03(
    Transparent<IntroSectionMeta, LeadVocalPart13>,
    Transparent<IntroSectionMeta, LeadVocalPart14>,
    Transparent<IntroSectionMeta, LeadVocalPart15>,
    Transparent<IntroSectionMeta, LeadVocalPart16>,
    Transparent<IntroSectionMeta, LeadVocalPart17>,
    Transparent<IntroSectionMeta, LeadVocalPart18>,
);

#[derive(Flow)]
pub struct LeadVocalPart01(
    Step<Generate<58, 192, 6528>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<71, 384, 384>>,
    Step<Generate<68, 192, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<71, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<71, 480, 480>>,
    Step<Generate<66, 192, 192>>,
    Transparent<IntroSectionMeta, LeadVocalPart01Cadence>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart02(
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Transparent<IntroSectionMeta, LeadVocalTriple68Hold>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 480, 768>>,
    Step<Sing<58, 96, 96>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<61, 288, 672>>,
    Step<Sing<61, 192, 192>>,
    Transparent<IntroSectionMeta, LeadVocalTriple63Hold>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<66, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart03(
    Step<Sing<66, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 96, 96>>,
    Step<Sing<63, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 576>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 96, 576>>,
    Step<Sing<68, 96, 96>>,
);

#[derive(Flow)]
pub struct Sing68Triplet(
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
);

#[derive(Flow)]
pub struct Sing73Triplet(
    Step<Sing<73, 96, 96>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<73, 96, 96>>,
);

#[derive(Flow)]
pub struct TripleSing68Triplet(Double<Sing68Triplet>, Sing68Triplet);

#[derive(Flow)]
pub struct LeadVocalPart01Cadence(Double<Generate68Tick>, Generate68Hold);

#[derive(Flow)]
pub struct LeadVocalTriple68Hold(Double<Sing68Hold>, Sing68Hold);

#[derive(Flow)]
pub struct LeadVocalTriple63Hold(Double<Sing63Hold>, Sing63Hold);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart04(
    TripleSing68Triplet,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<70, 384, 672>>,
    Step<Sing<70, 480, 1056>>,
    Step<Sing<70, 384, 768>>,
    Step<Sing<70, 384, 672>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<73, 576, 576>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart05(
    Step<Sing<66, 96, 96>>,
    Step<Sing<71, 384, 384>>,
    Step<Sing<68, 192, 384>>,
    Step<Sing<59, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 192, 576>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<71, 288, 288>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<71, 480, 480>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart06(
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 480, 768>>,
    Step<Sing<58, 96, 96>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 384, 768>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart07(
    Step<Sing<61, 96, 96>>,
    Step<Sing<63, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 576>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 384>>,
    Step<Sing<68, 384, 384>>,
    Step<Sing<66, 192, 864>>,
    Step<Sing<68, 96, 768>>,
    Step<Sing<68, 96, 480>>,
    Step<Sing<70, 384, 672>>,
    Step<Sing<70, 480, 1056>>,
    Step<Sing<70, 384, 768>>,
    Step<Sing<70, 384, 672>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<70, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart08(
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<73, 384, 12864>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<71, 384, 384>>,
    Step<Sing<68, 192, 384>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 192, 384>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<71, 192, 192>>,
    Step<Sing<68, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart09(
    Step<Sing<71, 288, 288>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 128, 128>>,
    Step<Sing<68, 257, 129>>,
    Step<Sing<68, 0, 128>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 480, 768>>,
    Step<Sing<58, 96, 96>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 384, 768>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart10(
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 96, 96>>,
    Step<Sing<63, 288, 288>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<66, 288, 288>>,
    Step<Sing<63, 192, 576>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 384>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart11(
    Step<Sing<68, 384, 384>>,
    Step<Sing<66, 192, 960>>,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Step<Sing<70, 384, 672>>,
    Step<Sing<70, 480, 1056>>,
    Step<Sing<73, 576, 1344>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<70, 384, 7488>>,
    Step<Sing<60, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart12(
    Step<Sing<65, 192, 192>>,
    Step<Sing<68, 384, 384>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<63, 384, 384>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<63, 96, 96>>,
    Step<Sing<61, 288, 288>>,
    Step<Sing<61, 384, 960>>,
    Step<Sing<65, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<65, 192, 192>>,
    Step<Sing<63, 192, 192>>,
    Step<Sing<61, 192, 192>>,
    Step<Sing<61, 960, 1344>>,
    Step<Sing<59, 192, 384>>,
    Step<Sing<59, 192, 192>>,
    Step<Sing<58, 576, 960>>,
    Step<Sing<59, 192, 384>>,
    Step<Sing<59, 192, 192>>,
    Step<Sing<58, 576, 960>>,
    Step<Sing<59, 192, 384>>,
    Step<Sing<63, 768, 768>>,
    Step<Sing<68, 384, 768>>,
    Step<Sing<75, 1536, 1536>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart13(
    Step<Sing<75, 768, 768>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 192, 384>>,
    Step<Sing<70, 384, 40512>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<73, 257, 128>>,
    Step<Sing<73, 0, 129>>,
    Step<Sing<73, 512, 128>>,
    Step<Sing<73, 0, 1152>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart14(
    Step<Sing<71, 128, 128>>,
    Step<Sing<73, 129, 129>>,
    Step<Sing<70, 128, 128>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<73, 192, 1152>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<73, 384, 384>>,
    Step<Sing<66, 3456, 3456>>,
    Step<Sing<73, 384, 384>>,
    Step<Sing<71, 96, 96>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<73, 288, 288>>,
    Step<Sing<71, 192, 576>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<71, 96, 96>>,
    Step<Sing<75, 384, 384>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<73, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart15(
    Step<Sing<72, 192, 192>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<72, 192, 192>>,
    Step<Sing<73, 192, 192>>,
    Step<Sing<72, 96, 576>>,
    Sing73Triplet,
    Sing73Triplet,
    Sing73Triplet,
    Sing73Triplet,
    Step<Sing<75, 384, 672>>,
    Step<Sing<75, 480, 480>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 192, 576>>,
    Step<Sing<70, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart16(
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 384>>,
    Step<Sing<68, 384, 384>>,
    Step<Sing<66, 192, 672>>,
    Step<Sing<70, 288, 288>>,
    Step<Sing<70, 192, 672>>,
    Step<Sing<70, 288, 672>>,
    Step<Sing<69, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<70, 480, 864>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 576>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart17(
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 576>>,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Step<Sing<70, 384, 672>>,
    Step<Sing<70, 288, 288>>,
    Step<Sing<70, 192, 192>>,
    Step<Sing<68, 96, 96>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 288, 288>>,
    Step<Sing<66, 192, 576>>,
    Step<Sing<70, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct LeadVocalPart18(
    Step<Sing<70, 192, 192>>,
    Step<Sing<70, 96, 96>>,
    Step<Sing<73, 96, 96>>,
    Step<Sing<70, 384, 384>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 192>>,
    Step<Sing<68, 192, 192>>,
    Step<Sing<66, 192, 480>>,
    Step<Sing<75, 192, 192>>,
    Step<Sing<75, 96, 96>>,
    Step<Sing<70, 288, 288>>,
    Step<Sing<73, 288, 288>>,
    Step<Sing<70, 384, 480>>,
    Step<Sing<72, 288, 864>>,
    Step<Sing<73, 192, 192>>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::LeadVocalist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-lead-vocal-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<LeadVocalist>(seed)
            .await
            .expect("journey should start");

        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = client
            .journey_details(journey_id)
            .await
            .expect("journey details should be available");
        match status {
            JourneyStatus::Dead | JourneyStatus::Stopped => {
                panic!("journey reached terminal non-complete status: {status:?}");
            }
            JourneyStatus::Created | JourneyStatus::Alive | JourneyStatus::Completed => {}
        }

        worker_handle.abort();
        let _ = worker_handle.await;
    }
}
