use jungle_sdk::prelude::*;

use crate::effect::{AtomicDualHit, Monad, Rest, Triad};
use crate::instrumentation::{
    Cymbal, CymbalArticulation, HiHat, HiHatArticulation, KickDrum, KickDrumArticulation,
    SnareDrum, SnareDrumArticulation, Toms, TomsArticulation,
};

use super::Drums;

pub type DrummerState = ();
pub type DrummerSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 5_376;
const DRUMS_LANE_ID: u32 = <<Drums as Animal>::Id as AnimalIdValue>::U32;

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

pub struct HatBoot<
    const HAT_NOTE: u8,
    const BOOT_NOTE: u8,
    const HAT_NOTE_TICK: u32,
    const BOOT_NOTE_TICK: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const HAT_NOTE: u8,
        const BOOT_NOTE: u8,
        const HAT_NOTE_TICK: u32,
        const BOOT_NOTE_TICK: u32,
        const REST_TICK: u32,
    > Act for HatBoot<HAT_NOTE, BOOT_NOTE, HAT_NOTE_TICK, BOOT_NOTE_TICK, REST_TICK>
{
    type Effect = AtomicDualHit<
        HiHat,
        KickDrum,
        HiHatArticulation,
        KickDrumArticulation,
        DRUMS_LANE_ID,
        HAT_NOTE,
        BOOT_NOTE,
        HAT_NOTE_TICK,
        BOOT_NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            HiHatArticulation::ClosedTip,
            KickDrumArticulation::StandardHit,
        )
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("hat+kick playback should succeed");
    }
}

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

pub struct HatDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > Act for HatDual<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        HiHat,
        HiHat,
        HiHatArticulation,
        HiHatArticulation,
        DRUMS_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (HiHatArticulation::ClosedTip, HiHatArticulation::ClosedTip)
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("hat dual playback should succeed");
    }
}

pub struct BootDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > Act for BootDual<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        KickDrum,
        KickDrum,
        KickDrumArticulation,
        KickDrumArticulation,
        DRUMS_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            KickDrumArticulation::StandardHit,
            KickDrumArticulation::StandardHit,
        )
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("kick dual playback should succeed");
    }
}

pub struct SnapDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > Act for SnapDual<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        SnareDrum,
        SnareDrum,
        SnareDrumArticulation,
        SnareDrumArticulation,
        DRUMS_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            SnareDrumArticulation::Rimshot,
            SnareDrumArticulation::Rimshot,
        )
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("snare dual playback should succeed");
    }
}

pub struct BlastDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > Act for BlastDual<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        Cymbal,
        Cymbal,
        CymbalArticulation,
        CymbalArticulation,
        DRUMS_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (
            CymbalArticulation::StandardCrash,
            CymbalArticulation::StandardCrash,
        )
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cymbal dual playback should succeed");
    }
}

pub struct TomDual<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > Act for TomDual<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        Toms,
        Toms,
        TomsArticulation,
        TomsArticulation,
        DRUMS_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        (TomsArticulation::StandardHit, TomsArticulation::StandardHit)
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("tom dual playback should succeed");
    }
}

pub struct TomTriad<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > Act for TomTriad<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
{
    type Effect =
        Triad<Toms, TomsArticulation, DRUMS_LANE_ID, NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        TomsArticulation::StandardHit
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("tom triad playback should succeed");
    }
}

#[derive(Flow)]
pub struct DrummerIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, DrumSection01>,
    Transparent<IntroSectionMeta, DrumSection02>,
    Transparent<IntroSectionMeta, DrumSection03>,
    Transparent<IntroSectionMeta, DrumSection04>,
    Transparent<IntroSectionMeta, DrumSection05>,
    Transparent<IntroSectionMeta, DrumSection06>,
    Transparent<IntroSectionMeta, DrumSection07>,
    Transparent<IntroSectionMeta, DrumSection08>,
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
pub struct DrumPart01(
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart02(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart03(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<44, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
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
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart05(
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Join<Step<Boot<36, 96, 0>>, Step<Hat<46, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
    Step<Hat<46, 96, 96>>,
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
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart08(
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart09(
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart10(
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart11(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart12(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart13(
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
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
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
pub struct DrumPart15(
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
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
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart16(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart17(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart18(
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart19(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
);

#[derive(Flow)]
pub struct DrumPart20(
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
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
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart22(
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
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
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart23(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart24(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart25(
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart26(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart27(
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart28(
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 96, 96>>,
    Step<Snap<38, 96, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart29(
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart30(
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<42, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Join<Step<Hat<42, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<42, 192, 192>>,
    Step<Hat<42, 192, 192>>,
    Step<Hat<42, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 336>>,
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
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
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
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
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
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
    Join<Step<Blast<57, 384, 0>>, Step<BootDual<36, 36, 48, 384, 0>>>,
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
    Join<Step<Boot<35, 192, 0>>, Step<SnapDual<38, 38, 48, 192, 0>>>,
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
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart38(
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart39(
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 288>>,
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 96, 192, 0>>>,
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
    Join<Step<BootDual<36, 36, 48, 192, 0>>, Step<Hat<51, 192, 0>>>,
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
    Join<Step<BlastDual<49, 57, 192, 192, 0>>, Step<Boot<36, 192, 0>>>,
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
    Join<Step<BlastDual<49, 57, 96, 96, 0>>, Step<Boot<36, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart50(
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart51(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart52(
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart53(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart54(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart55(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart56(
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart57(
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart58(
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
);

#[derive(Flow)]
pub struct DrumPart59(
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomDual<43, 48, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomDual<62, 62, 96, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Step<Hat<44, 96, 96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Step<Hat<44, 96, 96>>,
);

#[derive(Flow)]
pub struct DrumPart60(
    Step<Hat<44, 96, 96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<HatBoot<44, 36, 96, 96, 0>>, Step<TomHit<43, 96, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<96>>,
    Join<Step<Hat<44, 96, 0>>, Step<TomTriad<48, 62, 62, 96, 0>>>,
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
    Join<Step<Blast<57, 192, 0>>, Step<BootDual<36, 36, 48, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
);

#[derive(Flow)]
pub struct DrumPart62(
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
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
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
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
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
pub struct DrumPart65(
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 192>>,
    Step<Snap<38, 48, 48>>,
    Step<Snap<38, 192, 96>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart66(
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Boot<36, 192, 0>>, Step<HatDual<44, 56, 192, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Step<HatDual<44, 56, 192, 192, 192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<HatDual<44, 56, 192, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Boot<36, 192, 192>>,
    Join<Step<Blast<57, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
);

#[derive(Flow)]
pub struct DrumPart67(
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Join<Step<Blast<49, 192, 0>>, Step<Boot<36, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Hat<46, 192, 0>>, Step<Snap<38, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Hat<46, 192, 192>>,
    Step<Hat<46, 192, 192>>,
    Join<Step<Boot<36, 192, 0>>, Step<Hat<46, 192, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
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
    Join<Step<BlastDual<49, 57, 384, 384, 0>>, Step<Boot<36, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::Drums;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-drums-intro-test")
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
}

