use jungle_sdk::prelude::*;

use crate::effect::{Monad, Rest};
use crate::instrumentation::{
    Cymbal, CymbalArticulation, HiHat, HiHatArticulation, KickDrum, KickDrumArticulation,
    SnareDrum, SnareDrumArticulation, Toms, TomsArticulation,
};

use super::{Double, Drums, Octa, Quad};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct DrummerState {
    groove_variant_is_46: bool,
}

impl Default for DrummerState {
    fn default() -> Self {
        Self {
            groove_variant_is_46: true,
        }
    }
}
pub type DrummerSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 5_376;
const DRUMS_LANE_ID: u32 = <<Drums as Animal>::Id as AnimalIdValue>::U32;
type Hat44Tick = Step<Hat<44, 96, 96>>;
type Hat46Tick = Step<Hat<46, 96, 96>>;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<DRUMS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct Hat<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for Hat<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<HiHat, HiHatArticulation, DRUMS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        HiHatArticulation::ClosedTip
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("hi-hat playback should succeed");
    }
}

pub struct Boot<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for Boot<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<KickDrum, KickDrumArticulation, DRUMS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        KickDrumArticulation::StandardHit
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("kick playback should succeed");
    }
}

pub struct Snap<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for Snap<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect =
        Monad<SnareDrum, SnareDrumArticulation, DRUMS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SnareDrumArticulation::Rimshot
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("snare playback should succeed");
    }
}

pub struct Blast<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for Blast<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Cymbal, CymbalArticulation, DRUMS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        CymbalArticulation::StandardCrash
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cymbal playback should succeed");
    }
}

pub struct TomHit<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for TomHit<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Toms, TomsArticulation, DRUMS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        TomsArticulation::StandardHit
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("tom playback should succeed");
    }
}

#[derive(Flow)]
pub struct HatBoot<
    const HAT_NOTE: u8,
    const BOOT_NOTE: u8,
    const HAT_NOTE_TICK: u32,
    const BOOT_NOTE_TICK: u32,
    const REST_TICK: u32,
>(
    Join<
        Step<Hat<HAT_NOTE, HAT_NOTE_TICK, REST_TICK>>,
        Step<Boot<BOOT_NOTE, BOOT_NOTE_TICK, REST_TICK>>,
    >,
    Step<MergeUnit>,
);

pub struct MergeUnit;
#[jungle::act]
impl Act for MergeUnit {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join merge should complete");
    }
}

pub struct PostMergeRest<const REST_TICK: u32>;
#[jungle::act]
impl<const REST_TICK: u32> Act for PostMergeRest<REST_TICK> {
    type Effect = Rest<DRUMS_LANE_ID, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("post-merge rest should complete");
    }
}

pub struct UseHat46GrooveVariant;
impl Condition<(DrummerState, ())> for UseHat46GrooveVariant {
    fn choose((state, _): &(DrummerState, ())) -> bool {
        state.groove_variant_is_46
    }
}

pub struct MergeGrooveVariantChoice;
#[jungle::act]
impl Act for MergeGrooveVariantChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("drum groove variant merge should complete");
    }
}

pub struct ConditionalJoinTailStub;
#[jungle::act]
impl Act for ConditionalJoinTailStub {
    type Effect = Monad<HiHat, HiHatArticulation, DRUMS_LANE_ID, 46, 1, 0>;
    type Input = Either<(), ()>;
    type Output = Either<(), ()>;

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        HiHatArticulation::ClosedTip
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("conditional join tail stub should succeed");
        Either::Left(())
    }
}

#[derive(Flow)]
pub struct Hat46SnapCadence(
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct BootHat46Cadence(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct Hat42SnapCadence(
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct BootHat42Cadence(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct HatDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<Hat<NOTE_1, NOTE_TICK_1, 0>>, Step<Hat<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct BootDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<Boot<NOTE_1, NOTE_TICK_1, 0>>, Step<Boot<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct SnapDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<Snap<NOTE_1, NOTE_TICK_1, 0>>, Step<Snap<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct BlastDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<Blast<NOTE_1, NOTE_TICK_1, 0>>, Step<Blast<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct TomDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<TomHit<NOTE_1, NOTE_TICK_1, 0>>, Step<TomHit<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct TomTriadPair<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32>(
    Join<Step<TomHit<NOTE_1, NOTE_TICK, 0>>, Step<TomHit<NOTE_2, NOTE_TICK, 0>>>,
    Step<MergeUnit>,
);

#[derive(Flow)]
pub struct TomTriad<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(
    Join<TomTriadPair<NOTE_1, NOTE_2, NOTE_TICK>, Step<TomHit<NOTE_3, NOTE_TICK, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct DrummerIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, DrummerIntroSectionChunk01>,
    Transparent<IntroSectionMeta, DrummerIntroSectionChunk02>,
    Transparent<IntroSectionMeta, DrummerIntroSectionChunk03>,
);

#[derive(Flow)]
pub struct DrummerIntroSectionChunk01(
    Transparent<IntroSectionMeta, DrumSection01>,
    Transparent<IntroSectionMeta, DrumSection02>,
    Transparent<IntroSectionMeta, DrumSection03>,
    Transparent<IntroSectionMeta, DrumSection04>,
);

#[derive(Flow)]
pub struct DrummerIntroSectionChunk02(
    Transparent<IntroSectionMeta, DrumSection05>,
    Transparent<IntroSectionMeta, DrumSection06>,
    Transparent<IntroSectionMeta, DrumSection07>,
    Transparent<IntroSectionMeta, DrumSection08>,
);

#[derive(Flow)]
pub struct DrummerIntroSectionChunk03(
    Transparent<IntroSectionMeta, DrumSection09>,
    Transparent<IntroSectionMeta, DrumSection10>,
    Transparent<IntroSectionMeta, DrumSection11>,
    Transparent<IntroSectionMeta, DrumSection12>,
);

#[derive(Flow)]
pub struct DrumSection01(
    Transparent<IntroSectionMeta, DrumPart01>,
    Transparent<IntroSectionMeta, DrumPart02>,
    Transparent<IntroSectionMeta, DrumPart03>,
    Transparent<IntroSectionMeta, DrumPart04>,
    Transparent<IntroSectionMeta, DrumPart05>,
    Transparent<IntroSectionMeta, DrumPart06>,
);

#[derive(Flow)]
pub struct DrumSection02(
    Transparent<IntroSectionMeta, DrumPart07>,
    Transparent<IntroSectionMeta, DrumPart08>,
    Transparent<IntroSectionMeta, DrumPart09>,
    Transparent<IntroSectionMeta, DrumPart10>,
    Transparent<IntroSectionMeta, DrumPart11>,
    Transparent<IntroSectionMeta, DrumPart12>,
);

#[derive(Flow)]
pub struct DrumSection03(
    Transparent<IntroSectionMeta, DrumPart13>,
    Transparent<IntroSectionMeta, DrumPart14>,
    Transparent<IntroSectionMeta, DrumPart15>,
    Transparent<IntroSectionMeta, DrumPart16>,
    Transparent<IntroSectionMeta, DrumPart17>,
    Transparent<IntroSectionMeta, DrumPart18>,
);

#[derive(Flow)]
pub struct DrumSection04(
    Transparent<IntroSectionMeta, DrumPart19>,
    Transparent<IntroSectionMeta, DrumPart20>,
    Transparent<IntroSectionMeta, DrumPart21>,
    Transparent<IntroSectionMeta, DrumPart22>,
    Transparent<IntroSectionMeta, DrumPart23>,
    Transparent<IntroSectionMeta, DrumPart24>,
);

#[derive(Flow)]
pub struct DrumSection05(
    Transparent<IntroSectionMeta, DrumPart25>,
    Transparent<IntroSectionMeta, DrumPart26>,
    Transparent<IntroSectionMeta, DrumPart27>,
    Transparent<IntroSectionMeta, DrumPart28>,
    Transparent<IntroSectionMeta, DrumPart29>,
    Transparent<IntroSectionMeta, DrumPart30>,
);

#[derive(Flow)]
pub struct DrumSection06(
    Transparent<IntroSectionMeta, DrumPart31>,
    Transparent<IntroSectionMeta, DrumPart32>,
    Transparent<IntroSectionMeta, DrumPart33>,
    Transparent<IntroSectionMeta, DrumPart34>,
    Transparent<IntroSectionMeta, DrumPart35>,
    Transparent<IntroSectionMeta, DrumPart36>,
);

#[derive(Flow)]
pub struct DrumSection07(
    Transparent<IntroSectionMeta, DrumPart37>,
    Transparent<IntroSectionMeta, DrumPart38>,
    Transparent<IntroSectionMeta, DrumPart39>,
    Transparent<IntroSectionMeta, DrumPart40>,
    Transparent<IntroSectionMeta, DrumPart41>,
    Transparent<IntroSectionMeta, DrumPart42>,
);

#[derive(Flow)]
pub struct DrumSection08(
    Transparent<IntroSectionMeta, DrumPart43>,
    Transparent<IntroSectionMeta, DrumPart44>,
    Transparent<IntroSectionMeta, DrumPart45>,
    Transparent<IntroSectionMeta, DrumPart46>,
    Transparent<IntroSectionMeta, DrumPart47>,
    Transparent<IntroSectionMeta, DrumPart48>,
);

#[derive(Flow)]
pub struct DrumSection09(
    Transparent<IntroSectionMeta, DrumPart49>,
    Transparent<IntroSectionMeta, DrumPart50>,
    Transparent<IntroSectionMeta, DrumPart51>,
    Transparent<IntroSectionMeta, DrumPart52>,
    Transparent<IntroSectionMeta, DrumPart53>,
    Transparent<IntroSectionMeta, DrumPart54>,
);

#[derive(Flow)]
pub struct DrumSection10(
    Transparent<IntroSectionMeta, DrumPart55>,
    Transparent<IntroSectionMeta, DrumPart56>,
    Transparent<IntroSectionMeta, DrumPart57>,
    Transparent<IntroSectionMeta, DrumPart58>,
    Transparent<IntroSectionMeta, DrumPart59>,
    Transparent<IntroSectionMeta, DrumPart60>,
);

#[derive(Flow)]
pub struct DrumSection11(
    Transparent<IntroSectionMeta, DrumPart61>,
    Transparent<IntroSectionMeta, DrumPart62>,
    Transparent<IntroSectionMeta, DrumPart63>,
    Transparent<IntroSectionMeta, DrumPart64>,
    Transparent<IntroSectionMeta, DrumPart65>,
    Transparent<IntroSectionMeta, DrumPart66>,
);

#[derive(Flow)]
pub struct DrumSection12(
    Transparent<IntroSectionMeta, DrumPart67>,
    Transparent<IntroSectionMeta, DrumPart68>,
);

#[derive(Flow)]
pub struct DrumHat44Six(Quad<Hat44Tick>, Double<Hat44Tick>);

#[derive(Flow)]
pub struct DrumHat44Nine(Octa<Hat44Tick>, Hat44Tick);

#[derive(Flow)]
pub struct DrumHat44Eleven(Octa<Hat44Tick>, Double<Hat44Tick>, Hat44Tick);

#[derive(Flow)]
pub struct DrumHat44Thirteen(Octa<Hat44Tick>, Quad<Hat44Tick>, Hat44Tick);

#[derive(Flow)]
pub struct DrumHat44Fourteen(Octa<Hat44Tick>, Quad<Hat44Tick>, Double<Hat44Tick>);

#[derive(Flow)]
pub struct DrumHat46Four(Quad<Hat46Tick>);

#[derive(Flow)]
pub struct DrumHat46Ten(Octa<Hat46Tick>, Double<Hat46Tick>);

#[derive(Flow)]
pub struct DrumHat46Eleven(Octa<Hat46Tick>, Double<Hat46Tick>, Hat46Tick);

#[derive(Flow)]
pub struct DrumPart01(
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    DrumHat44Fourteen,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    DrumHat44Six,
);

#[derive(Flow)]
pub struct DrumPart02(
    DrumHat44Nine,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    DrumHat44Thirteen,
);

#[derive(Flow)]
pub struct DrumPart03(
    Double<Hat44Tick>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    DrumHat44Eleven,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Join<Step<Blast<57, 96, 0>>, Step<Boot<36, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart04(
    DrumHat46Eleven,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    DrumHat46Eleven,
);

#[derive(Flow)]
pub struct DrumPart05(
    DrumHat46Four,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    DrumHat46Ten,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Hat<44, 384, 384>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
pub struct DrumPart06(
    Step<Hat<44, 384, 384>>,
    Join<Step<Blast<57, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart07(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 384, 288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
);

#[derive(Flow)]
pub struct DrumPart08(
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart09(
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
);

#[derive(Flow)]
pub struct DrumPart10(
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    BootHat42Cadence,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart11(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart12(
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    BootHat42Cadence,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
);

#[derive(Flow)]
pub struct DrumPart13(
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart14(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
pub struct DrumPart15(
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
);

#[derive(Flow)]
pub struct DrumPart16(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    BootHat42Cadence,
);

#[derive(Flow)]
pub struct DrumPart17(
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Snap<38, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart18(
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
);

#[derive(Flow)]
pub struct DrumPart19(
    BootHat42Cadence,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
);

#[derive(Flow)]
pub struct DrumPart20(
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
);

#[derive(Flow)]
pub struct DrumPart21(
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
);

#[derive(Flow)]
pub struct DrumPart22(
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart23(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    BootHat46Cadence,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart24(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    BootHat46Cadence,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
);

#[derive(Flow)]
pub struct DrumPart25(
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    BootHat46Cadence,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart26(
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    BootHat46Cadence,
    Hat46SnapCadence,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart27(
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart28(
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart29(
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
);

#[derive(Flow)]
pub struct DrumPart30(
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    BootHat42Cadence,
    Step<Hat<42, 192, 192>>,
    Hat42SnapCadence,
    Step<Hat<42, 192, 192>>,
    Step<Hat<42, 192, 192>>,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart31(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
pub struct DrumPart32(
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
);

#[derive(Flow)]
pub struct DrumPart33(
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 384, 0>>, BootDual<36, 36, 48, 384, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
pub struct DrumPart34(
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart35(
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 384, 0>>, Step<Hat<51, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Hat<51, 384, 0>>, Step<Snap<38, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Hat<51, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 384, 336>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 384, 384>>,
    Step<Snap<38, 48, 48>>,
);

#[derive(Flow)]
pub struct DrumPart36(
    Step<Snap<38, 384, 336>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 384, 384>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 384, 336>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Boot<36, 384, 384>>,
    Step<Snap<38, 48, 384>>,
    Join<Step<Boot<35, 192, 0>>, SnapDual<38, 38, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart37(
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
);

#[derive(Flow)]
pub struct DrumPart38(
    BootHat46Cadence,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    BootHat46Cadence,
    BootHat46Cadence,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart39(
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 288>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 96, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart40(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart41(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart42(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Blast<57, 192, 144>>,
    Join<BootDual<36, 36, 48, 192, 0>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart43(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart44(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart45(
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart46(
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<51, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<51, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<51, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart47(
    Step<Snap<38, 384, 384>>,
    Step<Snap<38, 384, 384>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart48(
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<35, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<35, 192, 144>>,
    Join<BlastDual<49, 57, 192, 192, 0>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Boot<36, 96, 288>>,
    Step<TomHit<48, 96, 96>>,
    Step<Boot<36, 96, 96>>,
    Step<TomHit<48, 96, 288>>,
    Step<Boot<36, 96, 288>>,
    Step<TomHit<48, 96, 96>>,
    Step<Boot<36, 96, 384>>,
    Step<Boot<36, 96, 288>>,
    Step<TomHit<48, 96, 96>>,
    Step<Boot<36, 96, 96>>,
    Step<TomHit<48, 96, 288>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<56, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart49(
    Step<Hat<56, 96, 96>>,
    Step<Hat<56, 96, 96>>,
    Step<Hat<56, 96, 96>>,
    Join<BlastDual<49, 57, 96, 96, 0>, Step<Boot<36, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart50(
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
);

#[derive(Flow)]
pub struct DrumPart51(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart52(
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart53(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart54(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart55(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart56(
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart57(
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart58(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart59(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, TomDual<43, 48, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomDual<62, 62, 96, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart60(
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<HatBoot<44, 36, 96, 96, 0>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, TomTriad<48, 62, 62, 96, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<49, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<49, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<49, 384, 0>>, Step<Boot<36, 384, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart61(
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Blast<57, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, BootDual<36, 36, 48, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart62(
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart63(
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart64(
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
);

#[derive(Flow)]
pub struct DrumPart65(
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Hat46SnapCadence,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart66(
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, HatDual<44, 56, 192, 192, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    HatDual<44, 56, 192, 192, 192>,
    Step<Boot<36, 192, 192>>,
    Join<HatDual<44, 56, 192, 192, 0>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart67(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    BootHat46Cadence,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Hat46SnapCadence,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    BootHat46Cadence,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<49, 96, 0>>, Step<Boot<36, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
);

#[derive(Flow)]
pub struct DrumPart68(
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<49, 96, 0>>, Step<Boot<36, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<288>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<576>>,
    Join<BlastDual<49, 57, 384, 384, 0>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[cfg(test)]
#[derive(Flow)]
pub struct ConditionalJoinMonad100LeftArm(
    Join<Step<Hat<46, 100, 0>>, Step<Hat<42, 100, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<0>>,
);

#[cfg(test)]
#[derive(Flow)]
pub struct ConditionalJoinMonad100RightArm(
    Join<Step<Boot<36, 100, 0>>, Step<Hat<44, 100, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<0>>,
);

#[cfg(test)]
#[derive(Flow)]
pub struct ConditionalJoinMonad100Flow(
    Conditional<
        UseHat46GrooveVariant,
        ConditionalJoinMonad100LeftArm,
        ConditionalJoinMonad100RightArm,
    >,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
    Step<ConditionalJoinTailStub>,
);

#[cfg(test)]
pub struct ConditionalJoinMonad100Animal;
#[cfg(test)]
#[jungle::animal(id = 77, generation = 0)]
impl Animal for ConditionalJoinMonad100Animal {
    type State = DrummerState;
    type Seed = DrummerState;
    type Journey = ConditionalJoinMonad100Flow;
}

#[cfg(test)]
impl From<DrummerState> for () {
    fn from(_value: DrummerState) -> Self {}
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::*;
    use jungle_sdk::{JungleClient, LocalClient, RunnerUpdateOut};

    use super::super::{ConditionalJoinMonad100Animal, Drums};
    use crate::ecosystem::TheJungle;

    async fn await_completion(client: &LocalClient, journey_id: uuid::Uuid) {
        let completion = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let status = client
                    .journey_details(journey_id)
                    .await
                    .expect("journey details should be available");
                match status {
                    JourneyStatus::Completed => break,
                    JourneyStatus::Dead | JourneyStatus::Stopped => {
                        panic!("journey reached terminal non-complete status: {status:?}");
                    }
                    JourneyStatus::Created | JourneyStatus::Alive => {}
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(completion.is_ok(), "journey should complete within timeout");
    }

    struct JourneyStreamStats {
        total_events: u32,
        started_count: u32,
        succeeded_count: u32,
        failed_count: u32,
        started_nodes: std::collections::BTreeSet<u32>,
        succeeded_nodes: std::collections::BTreeSet<u32>,
    }

    async fn collect_stream_stats(
        mut stream: jungle_sdk::client::JourneyUpdateSubscription,
        journey_id: uuid::Uuid,
    ) -> JourneyStreamStats {
        let mut total_events = 0_u32;
        let mut started_count = 0_u32;
        let mut succeeded_count = 0_u32;
        let mut failed_count = 0_u32;
        let mut started_nodes = std::collections::BTreeSet::new();
        let mut succeeded_nodes = std::collections::BTreeSet::new();
        let mut last_sequence_id: Option<u64> = None;

        while let Some(next) = stream.next().await {
            let update = next.expect("streamed journey update should succeed");
            if let Some(prev) = last_sequence_id {
                assert!(
                    update.sequence_id > prev,
                    "stream sequence ids should be strictly increasing"
                );
            }
            last_sequence_id = Some(update.sequence_id);

            match update.event {
                RunnerUpdateOut::EffectInput { uuid, node_id } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    started_count += 1;
                    started_nodes.insert(node_id);
                    total_events += 1;
                }
                RunnerUpdateOut::EffectSuccessOutput { uuid, node_id } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    succeeded_count += 1;
                    succeeded_nodes.insert(node_id);
                    total_events += 1;
                }
                RunnerUpdateOut::EffectFailureOutput { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    failed_count += 1;
                    total_events += 1;
                }
                RunnerUpdateOut::SleepScheduled { uuid, .. }
                | RunnerUpdateOut::SleepFired { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    total_events += 1;
                }
            }
        }

        JourneyStreamStats {
            total_events,
            started_count,
            succeeded_count,
            failed_count,
            started_nodes,
            succeeded_nodes,
        }
    }

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-drums-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<Drums>(seed)
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

    #[tokio::test]
    async fn conditional_join_monad_100_ticks_zero_rest_completes_with_local_client() {
        const PARALLEL_JOURNEYS: usize = 5;

        let client = LocalClient::builder()
            .namespace("welcome-conditional-join-monad-test")
            .build()
            .await
            .expect("local client should build");

        let audio_engine = welcome_audio::AudioEngine::start_default()
            .await
            .expect("shared real audio engine should start");
        let shared_audio_handle = audio_engine.handle();
        let shared_metronome = crate::metronome::Metronome::spawn(123.0);
        shared_metronome.arm_start_barrier();

        let mut worker_handles = Vec::with_capacity(PARALLEL_JOURNEYS);
        for _ in 0..PARALLEL_JOURNEYS {
            let ecosystem = TheJungle::new_with_metronome(
                shared_audio_handle.clone(),
                123.0,
                shared_metronome.clone(),
            );
            let worker = JungleWorker::new(ecosystem, client.clone());
            worker_handles.push(tokio::spawn(async move {
                let _ = worker.spawn().await;
            }));
        }

        let seeds = [
            super::DrummerState {
                groove_variant_is_46: true,
            },
            super::DrummerState {
                groove_variant_is_46: false,
            },
            super::DrummerState {
                groove_variant_is_46: true,
            },
            super::DrummerState {
                groove_variant_is_46: false,
            },
            super::DrummerState {
                groove_variant_is_46: true,
            },
        ];

        let mut journey_ids = Vec::with_capacity(PARALLEL_JOURNEYS);
        for (index, seed_state) in seeds.iter().enumerate() {
            let seed = postcard::to_allocvec(seed_state).expect("seed should serialize");
            let journey_id = client
                .start_journey::<ConditionalJoinMonad100Animal>(seed)
                .await
                .unwrap_or_else(|err| panic!("journey {index} should start: {err}"));
            journey_ids.push(journey_id);
        }

        let mut stream_tasks = Vec::with_capacity(PARALLEL_JOURNEYS);
        for journey_id in &journey_ids {
            let stream = client
                .subscribe_step_updates(*journey_id, None)
                .await
                .expect("subscribe_step_updates should succeed");
            let stream_journey_id = *journey_id;
            stream_tasks.push(tokio::spawn(async move {
                collect_stream_stats(stream, stream_journey_id).await
            }));
        }

        let release_task = tokio::spawn(async move {
            shared_metronome.release_start_barrier_on_downbeat().await;
        });

        let completion_futures = journey_ids
            .iter()
            .copied()
            .map(|journey_id| await_completion(&client, journey_id));
        futures::future::join_all(completion_futures).await;

        let mut all_started_nodes = std::collections::BTreeSet::new();
        let mut all_succeeded_nodes = std::collections::BTreeSet::new();
        for (index, stream_task) in stream_tasks.into_iter().enumerate() {
            let stats = stream_task
                .await
                .unwrap_or_else(|err| panic!("stream task {index} should join cleanly: {err}"));
            assert!(
                stats.total_events >= 10,
                "journey {index} stream should emit at least 10 updates, got {}",
                stats.total_events,
            );
            assert_eq!(
                stats.failed_count, 0,
                "journey {index} should not have failed task transitions"
            );
            assert!(
                stats.started_count > 0,
                "journey {index} should receive effect-input transitions"
            );
            assert!(
                stats.succeeded_count > 0,
                "journey {index} should receive effect-success transitions"
            );
            assert_eq!(
                stats.started_count, stats.succeeded_count,
                "journey {index} should have matching input/success transition counts"
            );
            assert_eq!(
                stats.started_nodes, stats.succeeded_nodes,
                "journey {index} should report matching started/succeeded task node transitions"
            );
            all_started_nodes.extend(stats.started_nodes.into_iter());
            all_succeeded_nodes.extend(stats.succeeded_nodes.into_iter());
        }
        assert_eq!(
            all_started_nodes, all_succeeded_nodes,
            "aggregate started/succeeded task transitions should match across all journeys"
        );

        let _ = release_task.await;
        for worker_handle in worker_handles {
            worker_handle.abort();
            let _ = worker_handle.await;
        }
    }
}
