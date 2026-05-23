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

//6, 0, Title_t, "Kick Drum"
//6, 0, Program_c, 9, 118
//6, 5376, Note_on_c, 9, 36, 37
//6, 5472, Note_off_c, 9, 36, 0
//6, 6912, Note_on_c, 9, 36, 37
//6, 7008, Note_off_c, 9, 36, 0
//6, 8448, Note_on_c, 9, 36, 37
//6, 8544, Note_off_c, 9, 36, 0
//6, 9984, Note_on_c, 9, 36, 37
//6, 10080, Note_off_c, 9, 36, 0
//6, 11520, Note_on_c, 9, 36, 37
//6, 11616, Note_off_c, 9, 36, 0
//6, 13056, Note_on_c, 9, 36, 37
//6, 13152, Note_off_c, 9, 36, 0
//6, 14592, Note_on_c, 9, 36, 37
//6, 14688, Note_off_c, 9, 36, 0
//6, 16128, Note_on_c, 9, 36, 37
//6, 16512, Note_off_c, 9, 36, 0
//6, 16896, Note_on_c, 9, 36, 37
//6, 17280, Note_off_c, 9, 36, 0
//6, 17856, Note_on_c, 9, 35, 37
//6, 18048, Note_off_c, 9, 35, 0
//6, 18048, Note_on_c, 9, 35, 37
//6, 18240, Note_off_c, 9, 35, 0
//6, 18240, Note_on_c, 9, 35, 37
//6, 18432, Note_off_c, 9, 35, 0
//6, 18432, Note_on_c, 9, 35, 37
//6, 18624, Note_off_c, 9, 35, 0
//6, 18624, Note_on_c, 9, 35, 37
//6, 18816, Note_off_c, 9, 35, 0
//6, 18816, Note_on_c, 9, 35, 37
//6, 19008, Note_off_c, 9, 35, 0
//6, 19008, Note_on_c, 9, 35, 37
//6, 19200, Note_off_c, 9, 35, 0
//6, 19200, Note_on_c, 9, 35, 37
//6, 19392, Note_off_c, 9, 35, 0
//6, 19392, Note_on_c, 9, 35, 37
//6, 19584, Note_off_c, 9, 35, 0
//6, 19584, Note_on_c, 9, 35, 37
//6, 19776, Note_off_c, 9, 35, 0
//6, 19776, Note_on_c, 9, 35, 37
//6, 19968, Note_off_c, 9, 35, 0
//6, 20208, Note_on_c, 9, 36, 37
//6, 20400, Note_off_c, 9, 36, 0
//6, 20736, Note_on_c, 9, 36, 37
//6, 20928, Note_off_c, 9, 36, 0
//6, 21504, Note_on_c, 9, 36, 37
//6, 21696, Note_off_c, 9, 36, 0
//6, 22080, Note_on_c, 9, 36, 37
//6, 22272, Note_off_c, 9, 36, 0
//6, 22272, Note_on_c, 9, 36, 37
//6, 22464, Note_off_c, 9, 36, 0
//6, 23040, Note_on_c, 9, 36, 37
//6, 23232, Note_off_c, 9, 36, 0
//6, 23616, Note_on_c, 9, 36, 37
//6, 23808, Note_off_c, 9, 36, 0
//6, 23808, Note_on_c, 9, 36, 37
//6, 24000, Note_off_c, 9, 36, 0
//6, 24576, Note_on_c, 9, 36, 37
//6, 24768, Note_off_c, 9, 36, 0
//6, 25152, Note_on_c, 9, 36, 37
//6, 25344, Note_off_c, 9, 36, 0
//6, 25344, Note_on_c, 9, 36, 37
//6, 25536, Note_off_c, 9, 36, 0
//6, 26112, Note_on_c, 9, 36, 37
//6, 26304, Note_off_c, 9, 36, 0
//6, 26880, Note_on_c, 9, 36, 37
//6, 27072, Note_off_c, 9, 36, 0
//6, 27648, Note_on_c, 9, 36, 37
//6, 27840, Note_off_c, 9, 36, 0
//6, 28416, Note_on_c, 9, 36, 37
//6, 28608, Note_off_c, 9, 36, 0
//6, 29184, Note_on_c, 9, 36, 37
//6, 29376, Note_off_c, 9, 36, 0
//6, 29760, Note_on_c, 9, 36, 37
//6, 29952, Note_off_c, 9, 36, 0
//6, 29952, Note_on_c, 9, 36, 37
//6, 30144, Note_off_c, 9, 36, 0
//6, 30720, Note_on_c, 9, 36, 37
//6, 30912, Note_off_c, 9, 36, 0
//6, 31488, Note_on_c, 9, 36, 37
//6, 31680, Note_off_c, 9, 36, 0
//6, 32256, Note_on_c, 9, 36, 37
//6, 32448, Note_off_c, 9, 36, 0
//6, 33024, Note_on_c, 9, 36, 37
//6, 33216, Note_off_c, 9, 36, 0
//6, 33792, Note_on_c, 9, 36, 37
//6, 33984, Note_off_c, 9, 36, 0
//6, 34560, Note_on_c, 9, 36, 37
//6, 34752, Note_off_c, 9, 36, 0
//6, 35328, Note_on_c, 9, 36, 37
//6, 35520, Note_off_c, 9, 36, 0
//6, 35904, Note_on_c, 9, 36, 37
//6, 36096, Note_off_c, 9, 36, 0
//6, 36096, Note_on_c, 9, 36, 37
//6, 36288, Note_off_c, 9, 36, 0
//6, 36864, Note_on_c, 9, 36, 37
//6, 37056, Note_off_c, 9, 36, 0
//6, 37632, Note_on_c, 9, 36, 37
//6, 37824, Note_off_c, 9, 36, 0
//6, 38400, Note_on_c, 9, 36, 37
//6, 38592, Note_off_c, 9, 36, 0
//6, 39168, Note_on_c, 9, 36, 37
//6, 39168, Note_on_c, 9, 36, 37
//6, 39216, Note_off_c, 9, 36, 0
//6, 39360, Note_off_c, 9, 36, 0
//6, 40128, Note_on_c, 9, 36, 37
//6, 40320, Note_off_c, 9, 36, 0
//6, 40512, Note_on_c, 9, 36, 37
//6, 40704, Note_off_c, 9, 36, 0
//6, 40704, Note_on_c, 9, 36, 37
//6, 40896, Note_off_c, 9, 36, 0
//6, 41664, Note_on_c, 9, 36, 37
//6, 41856, Note_off_c, 9, 36, 0
//6, 42240, Note_on_c, 9, 36, 37
//6, 42432, Note_off_c, 9, 36, 0
//6, 43200, Note_on_c, 9, 36, 37
//6, 43392, Note_off_c, 9, 36, 0
//6, 43776, Note_on_c, 9, 36, 37
//6, 43968, Note_off_c, 9, 36, 0
//6, 44736, Note_on_c, 9, 36, 37
//6, 44928, Note_off_c, 9, 36, 0
//6, 45312, Note_on_c, 9, 36, 37
//6, 45504, Note_off_c, 9, 36, 0
//6, 46272, Note_on_c, 9, 36, 37
//6, 46464, Note_off_c, 9, 36, 0
//6, 46848, Note_on_c, 9, 36, 37
//6, 47040, Note_off_c, 9, 36, 0
//6, 47136, Note_on_c, 9, 36, 37
//6, 47328, Note_off_c, 9, 36, 0
//6, 47424, Note_on_c, 9, 36, 37
//6, 47616, Note_off_c, 9, 36, 0
//6, 47808, Note_on_c, 9, 36, 37
//6, 48000, Note_off_c, 9, 36, 0
//6, 48384, Note_on_c, 9, 36, 37
//6, 48384, Note_on_c, 9, 36, 37
//6, 48432, Note_off_c, 9, 36, 0
//6, 48576, Note_off_c, 9, 36, 0
//6, 49152, Note_on_c, 9, 36, 37
//6, 49344, Note_off_c, 9, 36, 0
//6, 49920, Note_on_c, 9, 36, 37
//6, 50112, Note_off_c, 9, 36, 0
//6, 50688, Note_on_c, 9, 36, 37
//6, 50880, Note_off_c, 9, 36, 0
//6, 51264, Note_on_c, 9, 36, 37
//6, 51456, Note_off_c, 9, 36, 0
//6, 51456, Note_on_c, 9, 36, 37
//6, 51648, Note_off_c, 9, 36, 0
//6, 52224, Note_on_c, 9, 36, 37
//6, 52416, Note_off_c, 9, 36, 0
//6, 52992, Note_on_c, 9, 36, 37
//6, 53184, Note_off_c, 9, 36, 0
//6, 53760, Note_on_c, 9, 36, 37
//6, 53952, Note_off_c, 9, 36, 0
//6, 54528, Note_on_c, 9, 36, 37
//6, 54720, Note_off_c, 9, 36, 0
//6, 55296, Note_on_c, 9, 36, 37
//6, 55488, Note_off_c, 9, 36, 0
//6, 56064, Note_on_c, 9, 36, 37
//6, 56256, Note_off_c, 9, 36, 0
//6, 56832, Note_on_c, 9, 36, 37
//6, 57024, Note_off_c, 9, 36, 0
//6, 57408, Note_on_c, 9, 36, 37
//6, 57600, Note_off_c, 9, 36, 0
//6, 57600, Note_on_c, 9, 36, 37
//6, 57792, Note_off_c, 9, 36, 0
//6, 58368, Note_on_c, 9, 36, 37
//6, 58560, Note_off_c, 9, 36, 0
//6, 59136, Note_on_c, 9, 36, 37
//6, 59328, Note_off_c, 9, 36, 0
//6, 59904, Note_on_c, 9, 36, 37
//6, 60096, Note_off_c, 9, 36, 0
//6, 60672, Note_on_c, 9, 36, 37
//6, 60672, Note_on_c, 9, 36, 37
//6, 60720, Note_off_c, 9, 36, 0
//6, 60864, Note_off_c, 9, 36, 0
//6, 61632, Note_on_c, 9, 36, 37
//6, 61824, Note_off_c, 9, 36, 0
//6, 62016, Note_on_c, 9, 36, 37
//6, 62208, Note_off_c, 9, 36, 0
//6, 62208, Note_on_c, 9, 36, 37
//6, 62400, Note_off_c, 9, 36, 0
//6, 63168, Note_on_c, 9, 36, 37
//6, 63360, Note_off_c, 9, 36, 0
//6, 63744, Note_on_c, 9, 36, 37
//6, 63936, Note_off_c, 9, 36, 0
//6, 64704, Note_on_c, 9, 36, 37
//6, 64896, Note_off_c, 9, 36, 0
//6, 65280, Note_on_c, 9, 36, 37
//6, 65472, Note_off_c, 9, 36, 0
//6, 66240, Note_on_c, 9, 36, 37
//6, 66432, Note_off_c, 9, 36, 0
//6, 66816, Note_on_c, 9, 36, 37
//6, 67008, Note_off_c, 9, 36, 0
//6, 67776, Note_on_c, 9, 36, 37
//6, 67968, Note_off_c, 9, 36, 0
//6, 68352, Note_on_c, 9, 36, 37
//6, 68544, Note_off_c, 9, 36, 0
//6, 68640, Note_on_c, 9, 36, 37
//6, 68832, Note_off_c, 9, 36, 0
//6, 68928, Note_on_c, 9, 36, 37
//6, 69120, Note_off_c, 9, 36, 0
//6, 69312, Note_on_c, 9, 36, 37
//6, 69504, Note_off_c, 9, 36, 0
//6, 69888, Note_on_c, 9, 36, 37
//6, 69888, Note_on_c, 9, 36, 37
//6, 69936, Note_off_c, 9, 36, 0
//6, 70080, Note_off_c, 9, 36, 0
//6, 70656, Note_on_c, 9, 36, 37
//6, 70848, Note_off_c, 9, 36, 0
//6, 71232, Note_on_c, 9, 36, 37
//6, 71424, Note_off_c, 9, 36, 0
//6, 71424, Note_on_c, 9, 36, 37
//6, 71616, Note_off_c, 9, 36, 0
//6, 72192, Note_on_c, 9, 36, 37
//6, 72384, Note_off_c, 9, 36, 0
//6, 72384, Note_on_c, 9, 36, 37
//6, 72576, Note_off_c, 9, 36, 0
//6, 72960, Note_on_c, 9, 36, 37
//6, 73152, Note_off_c, 9, 36, 0
//6, 73728, Note_on_c, 9, 36, 37
//6, 73920, Note_off_c, 9, 36, 0
//6, 74304, Note_on_c, 9, 36, 37
//6, 74496, Note_off_c, 9, 36, 0
//6, 74496, Note_on_c, 9, 36, 37
//6, 74688, Note_off_c, 9, 36, 0
//6, 75264, Note_on_c, 9, 36, 37
//6, 75456, Note_off_c, 9, 36, 0
//6, 75456, Note_on_c, 9, 36, 37
//6, 75648, Note_off_c, 9, 36, 0
//6, 76032, Note_on_c, 9, 36, 37
//6, 76224, Note_off_c, 9, 36, 0
//6, 76800, Note_on_c, 9, 36, 37
//6, 76992, Note_off_c, 9, 36, 0
//6, 77376, Note_on_c, 9, 36, 37
//6, 77568, Note_off_c, 9, 36, 0
//6, 77568, Note_on_c, 9, 36, 37
//6, 77760, Note_off_c, 9, 36, 0
//6, 78336, Note_on_c, 9, 36, 37
//6, 78528, Note_off_c, 9, 36, 0
//6, 78528, Note_on_c, 9, 36, 37
//6, 78720, Note_off_c, 9, 36, 0
//6, 79104, Note_on_c, 9, 36, 37
//6, 79296, Note_off_c, 9, 36, 0
//6, 79872, Note_on_c, 9, 36, 37
//6, 80064, Note_off_c, 9, 36, 0
//6, 80448, Note_on_c, 9, 36, 37
//6, 80640, Note_off_c, 9, 36, 0
//6, 80640, Note_on_c, 9, 36, 37
//6, 80832, Note_off_c, 9, 36, 0
//6, 81408, Note_on_c, 9, 36, 37
//6, 81600, Note_off_c, 9, 36, 0
//6, 81600, Note_on_c, 9, 36, 37
//6, 81792, Note_off_c, 9, 36, 0
//6, 82176, Note_on_c, 9, 36, 37
//6, 82368, Note_off_c, 9, 36, 0
//6, 82944, Note_on_c, 9, 36, 37
//6, 83136, Note_off_c, 9, 36, 0
//6, 83712, Note_on_c, 9, 36, 37
//6, 83904, Note_off_c, 9, 36, 0
//6, 84480, Note_on_c, 9, 36, 37
//6, 84672, Note_off_c, 9, 36, 0
//6, 85248, Note_on_c, 9, 36, 37
//6, 85440, Note_off_c, 9, 36, 0
//6, 86016, Note_on_c, 9, 36, 37
//6, 86208, Note_off_c, 9, 36, 0
//6, 86784, Note_on_c, 9, 36, 37
//6, 86976, Note_off_c, 9, 36, 0
//6, 87552, Note_on_c, 9, 36, 37
//6, 87744, Note_off_c, 9, 36, 0
//6, 88320, Note_on_c, 9, 36, 37
//6, 88512, Note_off_c, 9, 36, 0
//6, 89088, Note_on_c, 9, 36, 37
//6, 89280, Note_off_c, 9, 36, 0
//6, 89856, Note_on_c, 9, 36, 37
//6, 90048, Note_off_c, 9, 36, 0
//6, 90624, Note_on_c, 9, 36, 37
//6, 90816, Note_off_c, 9, 36, 0
//6, 91392, Note_on_c, 9, 36, 37
//6, 91584, Note_off_c, 9, 36, 0
//6, 92160, Note_on_c, 9, 36, 37
//6, 92352, Note_off_c, 9, 36, 0
//6, 92928, Note_on_c, 9, 36, 37
//6, 93120, Note_off_c, 9, 36, 0
//6, 94464, Note_on_c, 9, 36, 37
//6, 94464, Note_on_c, 9, 36, 37
//6, 94512, Note_off_c, 9, 36, 0
//6, 94656, Note_off_c, 9, 36, 0
//6, 95424, Note_on_c, 9, 36, 37
//6, 95616, Note_off_c, 9, 36, 0
//6, 95808, Note_on_c, 9, 36, 37
//6, 96000, Note_off_c, 9, 36, 0
//6, 96000, Note_on_c, 9, 36, 37
//6, 96192, Note_off_c, 9, 36, 0
//6, 96960, Note_on_c, 9, 36, 37
//6, 97152, Note_off_c, 9, 36, 0
//6, 97536, Note_on_c, 9, 36, 37
//6, 97728, Note_off_c, 9, 36, 0
//6, 98496, Note_on_c, 9, 36, 37
//6, 98688, Note_off_c, 9, 36, 0
//6, 99072, Note_on_c, 9, 36, 37
//6, 99264, Note_off_c, 9, 36, 0
//6, 100032, Note_on_c, 9, 36, 37
//6, 100224, Note_off_c, 9, 36, 0
//6, 100608, Note_on_c, 9, 36, 37
//6, 100800, Note_off_c, 9, 36, 0
//6, 101568, Note_on_c, 9, 36, 37
//6, 101760, Note_off_c, 9, 36, 0
//6, 102144, Note_on_c, 9, 36, 37
//6, 102336, Note_off_c, 9, 36, 0
//6, 102432, Note_on_c, 9, 36, 37
//6, 102624, Note_off_c, 9, 36, 0
//6, 102720, Note_on_c, 9, 36, 37
//6, 102912, Note_off_c, 9, 36, 0
//6, 103104, Note_on_c, 9, 36, 37
//6, 103296, Note_off_c, 9, 36, 0
//6, 103680, Note_on_c, 9, 36, 37
//6, 103680, Note_on_c, 9, 36, 37
//6, 103728, Note_off_c, 9, 36, 0
//6, 104064, Note_off_c, 9, 36, 0
//6, 104640, Note_on_c, 9, 36, 37
//6, 104832, Note_off_c, 9, 36, 0
//6, 105216, Note_on_c, 9, 36, 37
//6, 105600, Note_off_c, 9, 36, 0
//6, 105792, Note_on_c, 9, 36, 37
//6, 105984, Note_off_c, 9, 36, 0
//6, 106176, Note_on_c, 9, 36, 37
//6, 106368, Note_off_c, 9, 36, 0
//6, 106752, Note_on_c, 9, 36, 37
//6, 107136, Note_off_c, 9, 36, 0
//6, 107712, Note_on_c, 9, 36, 37
//6, 107904, Note_off_c, 9, 36, 0
//6, 108288, Note_on_c, 9, 36, 37
//6, 108672, Note_off_c, 9, 36, 0
//6, 108864, Note_on_c, 9, 36, 37
//6, 109056, Note_off_c, 9, 36, 0
//6, 109248, Note_on_c, 9, 36, 37
//6, 109440, Note_off_c, 9, 36, 0
//6, 109824, Note_on_c, 9, 36, 37
//6, 110208, Note_off_c, 9, 36, 0
//6, 110784, Note_on_c, 9, 36, 37
//6, 110976, Note_off_c, 9, 36, 0
//6, 111360, Note_on_c, 9, 36, 37
//6, 111744, Note_off_c, 9, 36, 0
//6, 111936, Note_on_c, 9, 36, 37
//6, 112128, Note_off_c, 9, 36, 0
//6, 112320, Note_on_c, 9, 36, 37
//6, 112512, Note_off_c, 9, 36, 0
//6, 112896, Note_on_c, 9, 36, 37
//6, 113280, Note_off_c, 9, 36, 0
//6, 113856, Note_on_c, 9, 36, 37
//6, 114048, Note_off_c, 9, 36, 0
//6, 114432, Note_on_c, 9, 36, 37
//6, 114624, Note_off_c, 9, 36, 0
//6, 114624, Note_on_c, 9, 36, 37
//6, 114816, Note_off_c, 9, 36, 0
//6, 115008, Note_on_c, 9, 36, 37
//6, 115200, Note_off_c, 9, 36, 0
//6, 115200, Note_on_c, 9, 36, 37
//6, 115584, Note_off_c, 9, 36, 0
//6, 115968, Note_on_c, 9, 36, 37
//6, 116160, Note_off_c, 9, 36, 0
//6, 116160, Note_on_c, 9, 36, 37
//6, 116352, Note_off_c, 9, 36, 0
//6, 116544, Note_on_c, 9, 36, 37
//6, 116736, Note_off_c, 9, 36, 0
//6, 116736, Note_on_c, 9, 36, 37
//6, 117120, Note_off_c, 9, 36, 0
//6, 117504, Note_on_c, 9, 36, 37
//6, 117696, Note_off_c, 9, 36, 0
//6, 117696, Note_on_c, 9, 36, 37
//6, 117888, Note_off_c, 9, 36, 0
//6, 118080, Note_on_c, 9, 36, 37
//6, 118272, Note_off_c, 9, 36, 0
//6, 118272, Note_on_c, 9, 36, 37
//6, 118656, Note_off_c, 9, 36, 0
//6, 119040, Note_on_c, 9, 35, 37
//6, 119232, Note_off_c, 9, 35, 0
//6, 119232, Note_on_c, 9, 35, 37
//6, 119424, Note_off_c, 9, 35, 0
//6, 119424, Note_on_c, 9, 35, 37
//6, 119616, Note_off_c, 9, 35, 0
//6, 119616, Note_on_c, 9, 35, 37
//6, 119808, Note_off_c, 9, 35, 0
//6, 119808, Note_on_c, 9, 35, 37
//6, 120000, Note_off_c, 9, 35, 0
//6, 120000, Note_on_c, 9, 36, 37
//6, 120192, Note_off_c, 9, 36, 0
//6, 120576, Note_on_c, 9, 36, 37
//6, 120576, Note_on_c, 9, 36, 37
//6, 120624, Note_off_c, 9, 36, 0
//6, 120768, Note_off_c, 9, 36, 0
//6, 121344, Note_on_c, 9, 36, 37
//6, 121536, Note_off_c, 9, 36, 0
//6, 121920, Note_on_c, 9, 36, 37
//6, 122112, Note_off_c, 9, 36, 0
//6, 122112, Note_on_c, 9, 36, 37
//6, 122304, Note_off_c, 9, 36, 0
//6, 122880, Note_on_c, 9, 36, 37
//6, 123072, Note_off_c, 9, 36, 0
//6, 123072, Note_on_c, 9, 36, 37
//6, 123264, Note_off_c, 9, 36, 0
//6, 123648, Note_on_c, 9, 36, 37
//6, 123840, Note_off_c, 9, 36, 0
//6, 124416, Note_on_c, 9, 36, 37
//6, 124608, Note_off_c, 9, 36, 0
//6, 124992, Note_on_c, 9, 36, 37
//6, 125184, Note_off_c, 9, 36, 0
//6, 125184, Note_on_c, 9, 36, 37
//6, 125376, Note_off_c, 9, 36, 0
//6, 126192, Note_on_c, 9, 36, 37
//6, 126384, Note_off_c, 9, 36, 0
//6, 126720, Note_on_c, 9, 36, 37
//6, 126720, Note_on_c, 9, 36, 37
//6, 126816, Note_off_c, 9, 36, 0
//6, 126912, Note_off_c, 9, 36, 0
//6, 127488, Note_on_c, 9, 36, 37
//6, 127680, Note_off_c, 9, 36, 0
//6, 127680, Note_on_c, 9, 36, 37
//6, 127872, Note_off_c, 9, 36, 0
//6, 128064, Note_on_c, 9, 36, 37
//6, 128256, Note_off_c, 9, 36, 0
//6, 128448, Note_on_c, 9, 36, 37
//6, 128640, Note_off_c, 9, 36, 0
//6, 128832, Note_on_c, 9, 36, 37
//6, 129024, Note_off_c, 9, 36, 0
//6, 129216, Note_on_c, 9, 36, 37
//6, 129408, Note_off_c, 9, 36, 0
//6, 129792, Note_on_c, 9, 36, 37
//6, 129984, Note_off_c, 9, 36, 0
//6, 130560, Note_on_c, 9, 36, 37
//6, 130752, Note_off_c, 9, 36, 0
//6, 130752, Note_on_c, 9, 36, 37
//6, 130944, Note_off_c, 9, 36, 0
//6, 131136, Note_on_c, 9, 36, 37
//6, 131328, Note_off_c, 9, 36, 0
//6, 131520, Note_on_c, 9, 36, 37
//6, 131712, Note_off_c, 9, 36, 0
//6, 131904, Note_on_c, 9, 36, 37
//6, 132096, Note_off_c, 9, 36, 0
//6, 132288, Note_on_c, 9, 36, 37
//6, 132480, Note_off_c, 9, 36, 0
//6, 132864, Note_on_c, 9, 36, 37
//6, 133056, Note_off_c, 9, 36, 0
//6, 133632, Note_on_c, 9, 36, 37
//6, 133824, Note_off_c, 9, 36, 0
//6, 133824, Note_on_c, 9, 36, 37
//6, 134016, Note_off_c, 9, 36, 0
//6, 134208, Note_on_c, 9, 36, 37
//6, 134400, Note_off_c, 9, 36, 0
//6, 134592, Note_on_c, 9, 36, 37
//6, 134784, Note_off_c, 9, 36, 0
//6, 134976, Note_on_c, 9, 36, 37
//6, 135168, Note_off_c, 9, 36, 0
//6, 135408, Note_on_c, 9, 36, 37
//6, 135600, Note_off_c, 9, 36, 0
//6, 135600, Note_on_c, 9, 36, 37
//6, 135792, Note_off_c, 9, 36, 0
//6, 135936, Note_on_c, 9, 36, 37
//6, 135936, Note_on_c, 9, 36, 37
//6, 135984, Note_off_c, 9, 36, 0
//6, 136128, Note_off_c, 9, 36, 0
//6, 136128, Note_on_c, 9, 36, 37
//6, 136320, Note_off_c, 9, 36, 0
//6, 136704, Note_on_c, 9, 36, 37
//6, 136896, Note_off_c, 9, 36, 0
//6, 136896, Note_on_c, 9, 36, 37
//6, 137088, Note_off_c, 9, 36, 0
//6, 137472, Note_on_c, 9, 36, 37
//6, 137664, Note_off_c, 9, 36, 0
//6, 137664, Note_on_c, 9, 36, 37
//6, 137856, Note_off_c, 9, 36, 0
//6, 138240, Note_on_c, 9, 36, 37
//6, 138432, Note_off_c, 9, 36, 0
//6, 138432, Note_on_c, 9, 36, 37
//6, 138624, Note_off_c, 9, 36, 0
//6, 138624, Note_on_c, 9, 36, 37
//6, 138816, Note_off_c, 9, 36, 0
//6, 139008, Note_on_c, 9, 36, 37
//6, 139200, Note_off_c, 9, 36, 0
//6, 139200, Note_on_c, 9, 36, 37
//6, 139392, Note_off_c, 9, 36, 0
//6, 139776, Note_on_c, 9, 36, 37
//6, 139968, Note_off_c, 9, 36, 0
//6, 139968, Note_on_c, 9, 36, 37
//6, 140160, Note_off_c, 9, 36, 0
//6, 140544, Note_on_c, 9, 36, 37
//6, 140736, Note_off_c, 9, 36, 0
//6, 140736, Note_on_c, 9, 36, 37
//6, 140928, Note_off_c, 9, 36, 0
//6, 141312, Note_on_c, 9, 36, 37
//6, 141504, Note_off_c, 9, 36, 0
//6, 141504, Note_on_c, 9, 36, 37
//6, 141696, Note_off_c, 9, 36, 0
//6, 141888, Note_on_c, 9, 36, 37
//6, 142080, Note_off_c, 9, 36, 0
//6, 142272, Note_on_c, 9, 36, 37
//6, 142464, Note_off_c, 9, 36, 0
//6, 142848, Note_on_c, 9, 36, 37
//6, 143040, Note_off_c, 9, 36, 0
//6, 143040, Note_on_c, 9, 36, 37
//6, 143232, Note_off_c, 9, 36, 0
//6, 143424, Note_on_c, 9, 36, 37
//6, 143616, Note_off_c, 9, 36, 0
//6, 143808, Note_on_c, 9, 36, 37
//6, 144000, Note_off_c, 9, 36, 0
//6, 144384, Note_on_c, 9, 36, 37
//6, 144576, Note_off_c, 9, 36, 0
//6, 144576, Note_on_c, 9, 36, 37
//6, 144768, Note_off_c, 9, 36, 0
//6, 144960, Note_on_c, 9, 36, 37
//6, 145152, Note_off_c, 9, 36, 0
//6, 145344, Note_on_c, 9, 36, 37
//6, 145536, Note_off_c, 9, 36, 0
//6, 145920, Note_on_c, 9, 36, 37
//6, 146112, Note_off_c, 9, 36, 0
//6, 146112, Note_on_c, 9, 36, 37
//6, 146304, Note_off_c, 9, 36, 0
//6, 146688, Note_on_c, 9, 36, 37
//6, 146880, Note_off_c, 9, 36, 0
//6, 146880, Note_on_c, 9, 36, 37
//6, 147072, Note_off_c, 9, 36, 0
//6, 148224, Note_on_c, 9, 35, 37
//6, 148416, Note_off_c, 9, 35, 0
//6, 148416, Note_on_c, 9, 35, 37
//6, 148608, Note_off_c, 9, 35, 0
//6, 148608, Note_on_c, 9, 35, 37
//6, 148800, Note_off_c, 9, 35, 0
//6, 148800, Note_on_c, 9, 35, 37
//6, 148992, Note_off_c, 9, 35, 0
//6, 148992, Note_on_c, 9, 35, 37
//6, 149184, Note_off_c, 9, 35, 0
//6, 149184, Note_on_c, 9, 35, 37
//6, 149376, Note_off_c, 9, 35, 0
//6, 149376, Note_on_c, 9, 35, 37
//6, 149568, Note_off_c, 9, 35, 0
//6, 149568, Note_on_c, 9, 35, 37
//6, 149760, Note_off_c, 9, 35, 0
//6, 149760, Note_on_c, 9, 35, 37
//6, 149952, Note_off_c, 9, 35, 0
//6, 149952, Note_on_c, 9, 35, 37
//6, 150144, Note_off_c, 9, 35, 0
//6, 150144, Note_on_c, 9, 35, 37
//6, 150336, Note_off_c, 9, 35, 0
//6, 150336, Note_on_c, 9, 35, 37
//6, 150528, Note_off_c, 9, 35, 0
//6, 150528, Note_on_c, 9, 35, 37
//6, 150720, Note_off_c, 9, 35, 0
//6, 150720, Note_on_c, 9, 35, 37
//6, 150912, Note_off_c, 9, 35, 0
//6, 151152, Note_on_c, 9, 35, 37
//6, 151296, Note_on_c, 9, 36, 37
//6, 151344, Note_off_c, 9, 35, 0
//6, 151488, Note_off_c, 9, 36, 0
//6, 151680, Note_on_c, 9, 36, 37
//6, 151776, Note_off_c, 9, 36, 0
//6, 152064, Note_on_c, 9, 36, 37
//6, 152160, Note_off_c, 9, 36, 0
//6, 152448, Note_on_c, 9, 36, 37
//6, 152544, Note_off_c, 9, 36, 0
//6, 152832, Note_on_c, 9, 36, 37
//6, 152928, Note_off_c, 9, 36, 0
//6, 153216, Note_on_c, 9, 36, 37
//6, 153312, Note_off_c, 9, 36, 0
//6, 153600, Note_on_c, 9, 36, 37
//6, 153696, Note_off_c, 9, 36, 0
//6, 153984, Note_on_c, 9, 36, 37
//6, 154080, Note_off_c, 9, 36, 0
//6, 154368, Note_on_c, 9, 36, 37
//6, 154464, Note_off_c, 9, 36, 0
//6, 154752, Note_on_c, 9, 36, 37
//6, 154848, Note_off_c, 9, 36, 0
//6, 155136, Note_on_c, 9, 36, 37
//6, 155232, Note_off_c, 9, 36, 0
//6, 155520, Note_on_c, 9, 36, 37
//6, 155616, Note_off_c, 9, 36, 0
//6, 155904, Note_on_c, 9, 36, 37
//6, 156000, Note_off_c, 9, 36, 0
//6, 156288, Note_on_c, 9, 36, 37
//6, 156384, Note_off_c, 9, 36, 0
//6, 156672, Note_on_c, 9, 36, 37
//6, 156768, Note_off_c, 9, 36, 0
//6, 157056, Note_on_c, 9, 36, 37
//6, 157152, Note_off_c, 9, 36, 0
//6, 157440, Note_on_c, 9, 36, 37
//6, 157536, Note_off_c, 9, 36, 0
//6, 157824, Note_on_c, 9, 36, 37
//6, 157920, Note_off_c, 9, 36, 0
//6, 158208, Note_on_c, 9, 36, 37
//6, 158304, Note_off_c, 9, 36, 0
//6, 158592, Note_on_c, 9, 36, 37
//6, 158688, Note_off_c, 9, 36, 0
//6, 158976, Note_on_c, 9, 36, 37
//6, 159072, Note_off_c, 9, 36, 0
//6, 159360, Note_on_c, 9, 36, 37
//6, 159456, Note_off_c, 9, 36, 0
//6, 159744, Note_on_c, 9, 36, 37
//6, 159840, Note_off_c, 9, 36, 0
//6, 160128, Note_on_c, 9, 36, 37
//6, 160224, Note_off_c, 9, 36, 0
//6, 160512, Note_on_c, 9, 36, 37
//6, 160608, Note_off_c, 9, 36, 0
//6, 160896, Note_on_c, 9, 36, 37
//6, 160992, Note_off_c, 9, 36, 0
//6, 161280, Note_on_c, 9, 36, 37
//6, 161376, Note_off_c, 9, 36, 0
//6, 161664, Note_on_c, 9, 36, 37
//6, 161760, Note_off_c, 9, 36, 0
//6, 162048, Note_on_c, 9, 36, 37
//6, 162144, Note_off_c, 9, 36, 0
//6, 162432, Note_on_c, 9, 36, 37
//6, 162528, Note_off_c, 9, 36, 0
//6, 162816, Note_on_c, 9, 36, 37
//6, 162912, Note_off_c, 9, 36, 0
//6, 163200, Note_on_c, 9, 36, 37
//6, 163296, Note_off_c, 9, 36, 0
//6, 163584, Note_on_c, 9, 36, 37
//6, 163680, Note_off_c, 9, 36, 0
//6, 163968, Note_on_c, 9, 36, 37
//6, 164064, Note_off_c, 9, 36, 0
//6, 164352, Note_on_c, 9, 36, 37
//6, 164448, Note_off_c, 9, 36, 0
//6, 164736, Note_on_c, 9, 36, 37
//6, 164832, Note_off_c, 9, 36, 0
//6, 165120, Note_on_c, 9, 36, 37
//6, 165216, Note_off_c, 9, 36, 0
//6, 165504, Note_on_c, 9, 36, 37
//6, 165600, Note_off_c, 9, 36, 0
//6, 165888, Note_on_c, 9, 36, 37
//6, 165984, Note_off_c, 9, 36, 0
//6, 166272, Note_on_c, 9, 36, 37
//6, 166368, Note_off_c, 9, 36, 0
//6, 166656, Note_on_c, 9, 36, 37
//6, 166752, Note_off_c, 9, 36, 0
//6, 167040, Note_on_c, 9, 36, 37
//6, 167136, Note_off_c, 9, 36, 0
//6, 167424, Note_on_c, 9, 36, 37
//6, 167520, Note_off_c, 9, 36, 0
//6, 167808, Note_on_c, 9, 36, 37
//6, 167904, Note_off_c, 9, 36, 0
//6, 168192, Note_on_c, 9, 36, 37
//6, 168288, Note_off_c, 9, 36, 0
//6, 168576, Note_on_c, 9, 36, 37
//6, 168672, Note_off_c, 9, 36, 0
//6, 168960, Note_on_c, 9, 36, 37
//6, 169056, Note_off_c, 9, 36, 0
//6, 169344, Note_on_c, 9, 36, 37
//6, 169440, Note_off_c, 9, 36, 0
//6, 169728, Note_on_c, 9, 36, 37
//6, 169824, Note_off_c, 9, 36, 0
//6, 170112, Note_on_c, 9, 36, 37
//6, 170208, Note_off_c, 9, 36, 0
//6, 170496, Note_on_c, 9, 36, 37
//6, 170592, Note_off_c, 9, 36, 0
//6, 170880, Note_on_c, 9, 36, 37
//6, 170976, Note_off_c, 9, 36, 0
//6, 171264, Note_on_c, 9, 36, 37
//6, 171360, Note_off_c, 9, 36, 0
//6, 171648, Note_on_c, 9, 36, 37
//6, 171744, Note_off_c, 9, 36, 0
//6, 172032, Note_on_c, 9, 36, 37
//6, 172128, Note_off_c, 9, 36, 0
//6, 172800, Note_on_c, 9, 36, 37
//6, 173184, Note_off_c, 9, 36, 0
//6, 173184, Note_on_c, 9, 36, 37
//6, 173568, Note_off_c, 9, 36, 0
//6, 173568, Note_on_c, 9, 36, 37
//6, 173952, Note_off_c, 9, 36, 0
//6, 173952, Note_on_c, 9, 36, 37
//6, 174336, Note_off_c, 9, 36, 0
//6, 174336, Note_on_c, 9, 36, 37
//6, 174720, Note_off_c, 9, 36, 0
//6, 174720, Note_on_c, 9, 36, 37
//6, 175104, Note_off_c, 9, 36, 0
//6, 175104, Note_on_c, 9, 36, 37
//6, 175488, Note_off_c, 9, 36, 0
//6, 175872, Note_on_c, 9, 36, 37
//6, 175872, Note_on_c, 9, 36, 37
//6, 175920, Note_off_c, 9, 36, 0
//6, 176064, Note_off_c, 9, 36, 0
//6, 176832, Note_on_c, 9, 36, 37
//6, 177024, Note_off_c, 9, 36, 0
//6, 177216, Note_on_c, 9, 36, 37
//6, 177408, Note_off_c, 9, 36, 0
//6, 177408, Note_on_c, 9, 36, 37
//6, 177600, Note_off_c, 9, 36, 0
//6, 178368, Note_on_c, 9, 36, 37
//6, 178560, Note_off_c, 9, 36, 0
//6, 178944, Note_on_c, 9, 36, 37
//6, 179136, Note_off_c, 9, 36, 0
//6, 179904, Note_on_c, 9, 36, 37
//6, 180096, Note_off_c, 9, 36, 0
//6, 180480, Note_on_c, 9, 36, 37
//6, 180672, Note_off_c, 9, 36, 0
//6, 181440, Note_on_c, 9, 36, 37
//6, 181632, Note_off_c, 9, 36, 0
//6, 181824, Note_on_c, 9, 36, 37
//6, 182016, Note_off_c, 9, 36, 0
//6, 182016, Note_on_c, 9, 36, 37
//6, 182208, Note_off_c, 9, 36, 0
//6, 182976, Note_on_c, 9, 36, 37
//6, 183168, Note_off_c, 9, 36, 0
//6, 183360, Note_on_c, 9, 36, 37
//6, 183552, Note_off_c, 9, 36, 0
//6, 183552, Note_on_c, 9, 36, 37
//6, 183744, Note_off_c, 9, 36, 0
//6, 184512, Note_on_c, 9, 36, 37
//6, 184704, Note_off_c, 9, 36, 0
//6, 185088, Note_on_c, 9, 36, 37
//6, 185280, Note_off_c, 9, 36, 0
//6, 186048, Note_on_c, 9, 36, 37
//6, 186240, Note_off_c, 9, 36, 0
//6, 186624, Note_on_c, 9, 36, 37
//6, 186816, Note_off_c, 9, 36, 0
//6, 187584, Note_on_c, 9, 36, 37
//6, 187776, Note_off_c, 9, 36, 0
//6, 187968, Note_on_c, 9, 36, 37
//6, 188160, Note_off_c, 9, 36, 0
//6, 188160, Note_on_c, 9, 36, 37
//6, 188352, Note_off_c, 9, 36, 0
//6, 189120, Note_on_c, 9, 36, 37
//6, 189312, Note_off_c, 9, 36, 0
//6, 189504, Note_on_c, 9, 36, 37
//6, 189696, Note_off_c, 9, 36, 0
//6, 189696, Note_on_c, 9, 36, 37
//6, 189888, Note_off_c, 9, 36, 0
//6, 190656, Note_on_c, 9, 36, 37
//6, 190848, Note_off_c, 9, 36, 0
//6, 191232, Note_on_c, 9, 36, 37
//6, 191424, Note_off_c, 9, 36, 0
//6, 192192, Note_on_c, 9, 36, 37
//6, 192384, Note_off_c, 9, 36, 0
//6, 192768, Note_on_c, 9, 36, 37
//6, 192960, Note_off_c, 9, 36, 0
//6, 193728, Note_on_c, 9, 36, 37
//6, 193920, Note_off_c, 9, 36, 0
//6, 194112, Note_on_c, 9, 36, 37
//6, 194304, Note_off_c, 9, 36, 0
//6, 194304, Note_on_c, 9, 36, 37
//6, 194496, Note_off_c, 9, 36, 0
//6, 195264, Note_on_c, 9, 36, 37
//6, 195456, Note_off_c, 9, 36, 0
//6, 195648, Note_on_c, 9, 36, 37
//6, 195840, Note_off_c, 9, 36, 0
//6, 195840, Note_on_c, 9, 36, 37
//6, 196032, Note_off_c, 9, 36, 0
//6, 196800, Note_on_c, 9, 36, 37
//6, 196992, Note_off_c, 9, 36, 0
//6, 197376, Note_on_c, 9, 36, 37
//6, 197568, Note_off_c, 9, 36, 0
//6, 197664, Note_on_c, 9, 36, 37
//6, 197760, Note_off_c, 9, 36, 0
//6, 197952, Note_on_c, 9, 36, 37
//6, 198144, Note_off_c, 9, 36, 0
//6, 198240, Note_on_c, 9, 36, 37
//6, 198432, Note_off_c, 9, 36, 0
//6, 198528, Note_on_c, 9, 36, 37
//6, 198720, Note_off_c, 9, 36, 0
//6, 198720, Note_on_c, 9, 36, 37
//6, 198912, Note_off_c, 9, 36, 0
//6, 198912, Note_on_c, 9, 36, 37
//6, 199104, Note_off_c, 9, 36, 0
//6, 199200, Note_on_c, 9, 36, 37
//6, 199296, Note_off_c, 9, 36, 0
//6, 199488, Note_on_c, 9, 36, 37
//6, 199680, Note_off_c, 9, 36, 0
//6, 200064, Note_on_c, 9, 36, 37
//6, 200448, Note_off_c, 9, 36, 0
//6, 200448, End_track
//
//7, 0, Title_t, "Hi-Hat"
//7, 0, Program_c, 9, 118
//7, 5376, Note_on_c, 9, 46, 31
//7, 5568, Note_off_c, 9, 46, 0
//7, 5568, Note_on_c, 9, 44, 37
//7, 5664, Note_off_c, 9, 44, 0
//7, 5664, Note_on_c, 9, 44, 37
//7, 5760, Note_off_c, 9, 44, 0
//7, 5760, Note_on_c, 9, 44, 37
//7, 5856, Note_off_c, 9, 44, 0
//7, 5856, Note_on_c, 9, 44, 37
//7, 5952, Note_off_c, 9, 44, 0
//7, 5952, Note_on_c, 9, 44, 37
//7, 6048, Note_off_c, 9, 44, 0
//7, 6048, Note_on_c, 9, 44, 31
//7, 6144, Note_off_c, 9, 44, 0
//7, 6144, Note_on_c, 9, 44, 31
//7, 6240, Note_off_c, 9, 44, 0
//7, 6240, Note_on_c, 9, 44, 37
//7, 6336, Note_off_c, 9, 44, 0
//7, 6336, Note_on_c, 9, 44, 31
//7, 6432, Note_off_c, 9, 44, 0
//7, 6432, Note_on_c, 9, 44, 31
//7, 6528, Note_off_c, 9, 44, 0
//7, 6528, Note_on_c, 9, 44, 37
//7, 6624, Note_off_c, 9, 44, 0
//7, 6624, Note_on_c, 9, 44, 31
//7, 6720, Note_off_c, 9, 44, 0
//7, 6720, Note_on_c, 9, 44, 37
//7, 6816, Note_off_c, 9, 44, 0
//7, 6816, Note_on_c, 9, 44, 31
//7, 6912, Note_off_c, 9, 44, 0
//7, 6912, Note_on_c, 9, 44, 37
//7, 7008, Note_off_c, 9, 44, 0
//7, 7008, Note_on_c, 9, 44, 37
//7, 7104, Note_off_c, 9, 44, 0
//7, 7104, Note_on_c, 9, 44, 37
//7, 7200, Note_off_c, 9, 44, 0
//7, 7200, Note_on_c, 9, 44, 37
//7, 7296, Note_off_c, 9, 44, 0
//7, 7296, Note_on_c, 9, 44, 37
//7, 7392, Note_off_c, 9, 44, 0
//7, 7392, Note_on_c, 9, 44, 37
//7, 7488, Note_off_c, 9, 44, 0
//7, 7488, Note_on_c, 9, 44, 37
//7, 7584, Note_off_c, 9, 44, 0
//7, 7584, Note_on_c, 9, 44, 31
//7, 7680, Note_off_c, 9, 44, 0
//7, 7680, Note_on_c, 9, 44, 31
//7, 7776, Note_off_c, 9, 44, 0
//7, 7776, Note_on_c, 9, 44, 37
//7, 7872, Note_off_c, 9, 44, 0
//7, 7872, Note_on_c, 9, 44, 31
//7, 7968, Note_off_c, 9, 44, 0
//7, 7968, Note_on_c, 9, 44, 31
//7, 8064, Note_off_c, 9, 44, 0
//7, 8064, Note_on_c, 9, 44, 37
//7, 8160, Note_off_c, 9, 44, 0
//7, 8160, Note_on_c, 9, 44, 31
//7, 8256, Note_off_c, 9, 44, 0
//7, 8256, Note_on_c, 9, 44, 37
//7, 8352, Note_off_c, 9, 44, 0
//7, 8352, Note_on_c, 9, 44, 31
//7, 8448, Note_off_c, 9, 44, 0
//7, 8448, Note_on_c, 9, 44, 37
//7, 8544, Note_off_c, 9, 44, 0
//7, 8544, Note_on_c, 9, 44, 37
//7, 8640, Note_off_c, 9, 44, 0
//7, 8640, Note_on_c, 9, 44, 37
//7, 8736, Note_off_c, 9, 44, 0
//7, 8736, Note_on_c, 9, 44, 37
//7, 8832, Note_off_c, 9, 44, 0
//7, 8832, Note_on_c, 9, 44, 37
//7, 8928, Note_off_c, 9, 44, 0
//7, 8928, Note_on_c, 9, 44, 37
//7, 9024, Note_off_c, 9, 44, 0
//7, 9024, Note_on_c, 9, 44, 37
//7, 9120, Note_off_c, 9, 44, 0
//7, 9120, Note_on_c, 9, 44, 31
//7, 9216, Note_off_c, 9, 44, 0
//7, 9216, Note_on_c, 9, 44, 31
//7, 9312, Note_off_c, 9, 44, 0
//7, 9312, Note_on_c, 9, 44, 37
//7, 9408, Note_off_c, 9, 44, 0
//7, 9408, Note_on_c, 9, 44, 31
//7, 9504, Note_off_c, 9, 44, 0
//7, 9504, Note_on_c, 9, 44, 31
//7, 9600, Note_off_c, 9, 44, 0
//7, 9600, Note_on_c, 9, 44, 37
//7, 9696, Note_off_c, 9, 44, 0
//7, 9696, Note_on_c, 9, 44, 31
//7, 9792, Note_off_c, 9, 44, 0
//7, 9792, Note_on_c, 9, 44, 37
//7, 9888, Note_off_c, 9, 44, 0
//7, 9888, Note_on_c, 9, 44, 31
//7, 9984, Note_off_c, 9, 44, 0
//7, 9984, Note_on_c, 9, 44, 37
//7, 10080, Note_off_c, 9, 44, 0
//7, 10080, Note_on_c, 9, 44, 37
//7, 10176, Note_off_c, 9, 44, 0
//7, 10176, Note_on_c, 9, 44, 37
//7, 10272, Note_off_c, 9, 44, 0
//7, 10272, Note_on_c, 9, 44, 37
//7, 10368, Note_off_c, 9, 44, 0
//7, 10368, Note_on_c, 9, 44, 37
//7, 10464, Note_off_c, 9, 44, 0
//7, 10464, Note_on_c, 9, 44, 37
//7, 10560, Note_off_c, 9, 44, 0
//7, 10560, Note_on_c, 9, 44, 37
//7, 10656, Note_off_c, 9, 44, 0
//7, 10656, Note_on_c, 9, 44, 31
//7, 10752, Note_off_c, 9, 44, 0
//7, 10752, Note_on_c, 9, 44, 31
//7, 10848, Note_off_c, 9, 44, 0
//7, 10848, Note_on_c, 9, 44, 37
//7, 10944, Note_off_c, 9, 44, 0
//7, 10944, Note_on_c, 9, 44, 31
//7, 11040, Note_off_c, 9, 44, 0
//7, 11040, Note_on_c, 9, 44, 31
//7, 11136, Note_off_c, 9, 44, 0
//7, 11136, Note_on_c, 9, 46, 31
//7, 11328, Note_off_c, 9, 46, 0
//7, 11328, Note_on_c, 9, 46, 31
//7, 11424, Note_off_c, 9, 46, 0
//7, 11424, Note_on_c, 9, 46, 31
//7, 11520, Note_off_c, 9, 46, 0
//7, 11616, Note_on_c, 9, 46, 31
//7, 11712, Note_off_c, 9, 46, 0
//7, 11712, Note_on_c, 9, 46, 31
//7, 11808, Note_off_c, 9, 46, 0
//7, 11808, Note_on_c, 9, 46, 31
//7, 11904, Note_off_c, 9, 46, 0
//7, 11904, Note_on_c, 9, 46, 31
//7, 12000, Note_off_c, 9, 46, 0
//7, 12000, Note_on_c, 9, 46, 31
//7, 12096, Note_off_c, 9, 46, 0
//7, 12096, Note_on_c, 9, 46, 31
//7, 12192, Note_off_c, 9, 46, 0
//7, 12192, Note_on_c, 9, 46, 31
//7, 12288, Note_off_c, 9, 46, 0
//7, 12288, Note_on_c, 9, 46, 31
//7, 12384, Note_off_c, 9, 46, 0
//7, 12384, Note_on_c, 9, 46, 31
//7, 12480, Note_off_c, 9, 46, 0
//7, 12480, Note_on_c, 9, 46, 31
//7, 12576, Note_off_c, 9, 46, 0
//7, 12576, Note_on_c, 9, 46, 31
//7, 12672, Note_off_c, 9, 46, 0
//7, 12672, Note_on_c, 9, 46, 31
//7, 12768, Note_off_c, 9, 46, 0
//7, 12768, Note_on_c, 9, 46, 31
//7, 12864, Note_off_c, 9, 46, 0
//7, 12864, Note_on_c, 9, 46, 31
//7, 12960, Note_off_c, 9, 46, 0
//7, 12960, Note_on_c, 9, 46, 31
//7, 13056, Note_off_c, 9, 46, 0
//7, 13056, Note_on_c, 9, 46, 37
//7, 13152, Note_off_c, 9, 46, 0
//7, 13152, Note_on_c, 9, 46, 31
//7, 13248, Note_off_c, 9, 46, 0
//7, 13248, Note_on_c, 9, 46, 31
//7, 13344, Note_off_c, 9, 46, 0
//7, 13344, Note_on_c, 9, 46, 31
//7, 13440, Note_off_c, 9, 46, 0
//7, 13440, Note_on_c, 9, 46, 31
//7, 13536, Note_off_c, 9, 46, 0
//7, 13536, Note_on_c, 9, 46, 31
//7, 13632, Note_off_c, 9, 46, 0
//7, 13632, Note_on_c, 9, 46, 31
//7, 13728, Note_off_c, 9, 46, 0
//7, 13728, Note_on_c, 9, 46, 31
//7, 13824, Note_off_c, 9, 46, 0
//7, 13824, Note_on_c, 9, 46, 31
//7, 13920, Note_off_c, 9, 46, 0
//7, 13920, Note_on_c, 9, 46, 31
//7, 14016, Note_off_c, 9, 46, 0
//7, 14016, Note_on_c, 9, 46, 31
//7, 14112, Note_off_c, 9, 46, 0
//7, 14112, Note_on_c, 9, 46, 31
//7, 14208, Note_off_c, 9, 46, 0
//7, 14208, Note_on_c, 9, 46, 31
//7, 14304, Note_off_c, 9, 46, 0
//7, 14304, Note_on_c, 9, 46, 31
//7, 14400, Note_off_c, 9, 46, 0
//7, 14400, Note_on_c, 9, 46, 31
//7, 14496, Note_off_c, 9, 46, 0
//7, 14496, Note_on_c, 9, 46, 31
//7, 14592, Note_off_c, 9, 46, 0
//7, 14592, Note_on_c, 9, 46, 37
//7, 14688, Note_off_c, 9, 46, 0
//7, 14688, Note_on_c, 9, 46, 31
//7, 14784, Note_off_c, 9, 46, 0
//7, 14784, Note_on_c, 9, 46, 31
//7, 14880, Note_off_c, 9, 46, 0
//7, 14880, Note_on_c, 9, 46, 31
//7, 14976, Note_off_c, 9, 46, 0
//7, 14976, Note_on_c, 9, 46, 31
//7, 15072, Note_off_c, 9, 46, 0
//7, 15072, Note_on_c, 9, 46, 31
//7, 15168, Note_off_c, 9, 46, 0
//7, 15168, Note_on_c, 9, 46, 31
//7, 15264, Note_off_c, 9, 46, 0
//7, 15264, Note_on_c, 9, 46, 31
//7, 15360, Note_off_c, 9, 46, 0
//7, 15360, Note_on_c, 9, 46, 31
//7, 15456, Note_off_c, 9, 46, 0
//7, 15456, Note_on_c, 9, 46, 31
//7, 15552, Note_off_c, 9, 46, 0
//7, 15552, Note_on_c, 9, 46, 31
//7, 15648, Note_off_c, 9, 46, 0
//7, 15648, Note_on_c, 9, 46, 31
//7, 15744, Note_off_c, 9, 46, 0
//7, 15744, Note_on_c, 9, 46, 31
//7, 15936, Note_off_c, 9, 46, 0
//7, 15936, Note_on_c, 9, 46, 31
//7, 16128, Note_off_c, 9, 46, 0
//7, 16512, Note_on_c, 9, 44, 37
//7, 16896, Note_off_c, 9, 44, 0
//7, 17280, Note_on_c, 9, 44, 37
//7, 17664, Note_off_c, 9, 44, 0
//7, 20928, Note_on_c, 9, 46, 37
//7, 21120, Note_off_c, 9, 46, 0
//7, 21120, Note_on_c, 9, 46, 31
//7, 21312, Note_off_c, 9, 46, 0
//7, 21312, Note_on_c, 9, 46, 31
//7, 21504, Note_off_c, 9, 46, 0
//7, 21504, Note_on_c, 9, 46, 31
//7, 21696, Note_off_c, 9, 46, 0
//7, 21696, Note_on_c, 9, 46, 31
//7, 21888, Note_off_c, 9, 46, 0
//7, 21888, Note_on_c, 9, 46, 31
//7, 22080, Note_off_c, 9, 46, 0
//7, 22080, Note_on_c, 9, 46, 31
//7, 22272, Note_off_c, 9, 46, 0
//7, 22272, Note_on_c, 9, 46, 31
//7, 22464, Note_off_c, 9, 46, 0
//7, 22464, Note_on_c, 9, 46, 31
//7, 22656, Note_off_c, 9, 46, 0
//7, 22656, Note_on_c, 9, 46, 31
//7, 22848, Note_off_c, 9, 46, 0
//7, 22848, Note_on_c, 9, 46, 31
//7, 23040, Note_off_c, 9, 46, 0
//7, 23040, Note_on_c, 9, 46, 31
//7, 23232, Note_off_c, 9, 46, 0
//7, 23232, Note_on_c, 9, 46, 31
//7, 23424, Note_off_c, 9, 46, 0
//7, 23424, Note_on_c, 9, 46, 31
//7, 23616, Note_off_c, 9, 46, 0
//7, 23616, Note_on_c, 9, 46, 31
//7, 23808, Note_off_c, 9, 46, 0
//7, 23808, Note_on_c, 9, 46, 31
//7, 24000, Note_off_c, 9, 46, 0
//7, 24000, Note_on_c, 9, 46, 31
//7, 24192, Note_off_c, 9, 46, 0
//7, 24192, Note_on_c, 9, 46, 31
//7, 24384, Note_off_c, 9, 46, 0
//7, 24384, Note_on_c, 9, 46, 31
//7, 24576, Note_off_c, 9, 46, 0
//7, 24576, Note_on_c, 9, 46, 31
//7, 24768, Note_off_c, 9, 46, 0
//7, 24768, Note_on_c, 9, 46, 31
//7, 24960, Note_off_c, 9, 46, 0
//7, 24960, Note_on_c, 9, 46, 31
//7, 25152, Note_off_c, 9, 46, 0
//7, 25152, Note_on_c, 9, 46, 31
//7, 25344, Note_off_c, 9, 46, 0
//7, 25344, Note_on_c, 9, 46, 31
//7, 25536, Note_off_c, 9, 46, 0
//7, 25536, Note_on_c, 9, 46, 31
//7, 25728, Note_off_c, 9, 46, 0
//7, 25728, Note_on_c, 9, 46, 31
//7, 25920, Note_off_c, 9, 46, 0
//7, 25920, Note_on_c, 9, 46, 31
//7, 26112, Note_off_c, 9, 46, 0
//7, 26112, Note_on_c, 9, 46, 31
//7, 26304, Note_off_c, 9, 46, 0
//7, 26304, Note_on_c, 9, 46, 31
//7, 26496, Note_off_c, 9, 46, 0
//7, 26496, Note_on_c, 9, 46, 31
//7, 26688, Note_off_c, 9, 46, 0
//7, 27072, Note_on_c, 9, 42, 37
//7, 27264, Note_off_c, 9, 42, 0
//7, 27264, Note_on_c, 9, 42, 37
//7, 27456, Note_off_c, 9, 42, 0
//7, 27456, Note_on_c, 9, 42, 37
//7, 27648, Note_off_c, 9, 42, 0
//7, 27648, Note_on_c, 9, 42, 37
//7, 27840, Note_off_c, 9, 42, 0
//7, 27840, Note_on_c, 9, 42, 37
//7, 28032, Note_off_c, 9, 42, 0
//7, 28032, Note_on_c, 9, 42, 37
//7, 28224, Note_off_c, 9, 42, 0
//7, 28224, Note_on_c, 9, 42, 37
//7, 28416, Note_off_c, 9, 42, 0
//7, 28416, Note_on_c, 9, 42, 37
//7, 28608, Note_off_c, 9, 42, 0
//7, 28608, Note_on_c, 9, 42, 37
//7, 28800, Note_off_c, 9, 42, 0
//7, 28800, Note_on_c, 9, 42, 37
//7, 28992, Note_off_c, 9, 42, 0
//7, 28992, Note_on_c, 9, 42, 37
//7, 29184, Note_off_c, 9, 42, 0
//7, 29184, Note_on_c, 9, 42, 37
//7, 29376, Note_off_c, 9, 42, 0
//7, 29376, Note_on_c, 9, 42, 37
//7, 29568, Note_off_c, 9, 42, 0
//7, 29568, Note_on_c, 9, 42, 37
//7, 29760, Note_off_c, 9, 42, 0
//7, 29760, Note_on_c, 9, 42, 37
//7, 29952, Note_off_c, 9, 42, 0
//7, 29952, Note_on_c, 9, 42, 37
//7, 30144, Note_off_c, 9, 42, 0
//7, 30144, Note_on_c, 9, 42, 37
//7, 30336, Note_off_c, 9, 42, 0
//7, 30336, Note_on_c, 9, 42, 37
//7, 30528, Note_off_c, 9, 42, 0
//7, 30528, Note_on_c, 9, 42, 37
//7, 30720, Note_off_c, 9, 42, 0
//7, 30720, Note_on_c, 9, 42, 37
//7, 30912, Note_off_c, 9, 42, 0
//7, 30912, Note_on_c, 9, 42, 37
//7, 31104, Note_off_c, 9, 42, 0
//7, 31104, Note_on_c, 9, 42, 37
//7, 31296, Note_off_c, 9, 42, 0
//7, 31296, Note_on_c, 9, 42, 37
//7, 31488, Note_off_c, 9, 42, 0
//7, 31488, Note_on_c, 9, 42, 37
//7, 31680, Note_off_c, 9, 42, 0
//7, 31680, Note_on_c, 9, 42, 37
//7, 31872, Note_off_c, 9, 42, 0
//7, 31872, Note_on_c, 9, 42, 37
//7, 32064, Note_off_c, 9, 42, 0
//7, 32064, Note_on_c, 9, 42, 37
//7, 32256, Note_off_c, 9, 42, 0
//7, 32256, Note_on_c, 9, 42, 37
//7, 32448, Note_off_c, 9, 42, 0
//7, 32448, Note_on_c, 9, 42, 37
//7, 32640, Note_off_c, 9, 42, 0
//7, 32640, Note_on_c, 9, 42, 37
//7, 32832, Note_off_c, 9, 42, 0
//7, 33216, Note_on_c, 9, 42, 37
//7, 33408, Note_off_c, 9, 42, 0
//7, 33408, Note_on_c, 9, 42, 37
//7, 33600, Note_off_c, 9, 42, 0
//7, 33600, Note_on_c, 9, 42, 37
//7, 33792, Note_off_c, 9, 42, 0
//7, 33792, Note_on_c, 9, 42, 37
//7, 33984, Note_off_c, 9, 42, 0
//7, 33984, Note_on_c, 9, 42, 37
//7, 34176, Note_off_c, 9, 42, 0
//7, 34176, Note_on_c, 9, 42, 37
//7, 34368, Note_off_c, 9, 42, 0
//7, 34368, Note_on_c, 9, 42, 37
//7, 34560, Note_off_c, 9, 42, 0
//7, 34560, Note_on_c, 9, 42, 37
//7, 34752, Note_off_c, 9, 42, 0
//7, 34752, Note_on_c, 9, 42, 37
//7, 34944, Note_off_c, 9, 42, 0
//7, 34944, Note_on_c, 9, 42, 37
//7, 35136, Note_off_c, 9, 42, 0
//7, 35136, Note_on_c, 9, 42, 37
//7, 35328, Note_off_c, 9, 42, 0
//7, 35328, Note_on_c, 9, 42, 37
//7, 35520, Note_off_c, 9, 42, 0
//7, 35520, Note_on_c, 9, 42, 37
//7, 35712, Note_off_c, 9, 42, 0
//7, 35712, Note_on_c, 9, 42, 37
//7, 35904, Note_off_c, 9, 42, 0
//7, 35904, Note_on_c, 9, 42, 37
//7, 36096, Note_off_c, 9, 42, 0
//7, 36096, Note_on_c, 9, 42, 37
//7, 36288, Note_off_c, 9, 42, 0
//7, 36288, Note_on_c, 9, 42, 37
//7, 36480, Note_off_c, 9, 42, 0
//7, 36480, Note_on_c, 9, 42, 37
//7, 36672, Note_off_c, 9, 42, 0
//7, 36672, Note_on_c, 9, 42, 37
//7, 36864, Note_off_c, 9, 42, 0
//7, 36864, Note_on_c, 9, 42, 37
//7, 37056, Note_off_c, 9, 42, 0
//7, 37056, Note_on_c, 9, 42, 37
//7, 37248, Note_off_c, 9, 42, 0
//7, 37248, Note_on_c, 9, 42, 37
//7, 37440, Note_off_c, 9, 42, 0
//7, 37440, Note_on_c, 9, 42, 37
//7, 37632, Note_off_c, 9, 42, 0
//7, 37632, Note_on_c, 9, 42, 37
//7, 37824, Note_off_c, 9, 42, 0
//7, 37824, Note_on_c, 9, 42, 37
//7, 38016, Note_off_c, 9, 42, 0
//7, 38016, Note_on_c, 9, 42, 37
//7, 38208, Note_off_c, 9, 42, 0
//7, 38208, Note_on_c, 9, 42, 37
//7, 38400, Note_off_c, 9, 42, 0
//7, 38400, Note_on_c, 9, 42, 37
//7, 38592, Note_off_c, 9, 42, 0
//7, 38592, Note_on_c, 9, 42, 37
//7, 38784, Note_off_c, 9, 42, 0
//7, 39360, Note_on_c, 9, 46, 31
//7, 39552, Note_off_c, 9, 46, 0
//7, 39552, Note_on_c, 9, 46, 31
//7, 39744, Note_off_c, 9, 46, 0
//7, 39744, Note_on_c, 9, 46, 31
//7, 39936, Note_off_c, 9, 46, 0
//7, 39936, Note_on_c, 9, 46, 31
//7, 40128, Note_off_c, 9, 46, 0
//7, 40128, Note_on_c, 9, 46, 31
//7, 40320, Note_off_c, 9, 46, 0
//7, 40320, Note_on_c, 9, 46, 31
//7, 40512, Note_off_c, 9, 46, 0
//7, 40512, Note_on_c, 9, 46, 31
//7, 40704, Note_off_c, 9, 46, 0
//7, 40896, Note_on_c, 9, 46, 31
//7, 41088, Note_off_c, 9, 46, 0
//7, 41088, Note_on_c, 9, 46, 31
//7, 41280, Note_off_c, 9, 46, 0
//7, 41280, Note_on_c, 9, 46, 31
//7, 41472, Note_off_c, 9, 46, 0
//7, 41472, Note_on_c, 9, 46, 31
//7, 41664, Note_off_c, 9, 46, 0
//7, 41664, Note_on_c, 9, 46, 31
//7, 41856, Note_off_c, 9, 46, 0
//7, 42624, Note_on_c, 9, 56, 37
//7, 42624, Note_on_c, 9, 44, 37
//7, 42816, Note_off_c, 9, 44, 0
//7, 42816, Note_off_c, 9, 56, 0
//7, 43008, Note_on_c, 9, 56, 37
//7, 43008, Note_on_c, 9, 44, 37
//7, 43200, Note_off_c, 9, 44, 0
//7, 43200, Note_off_c, 9, 56, 0
//7, 43392, Note_on_c, 9, 56, 37
//7, 43392, Note_on_c, 9, 44, 37
//7, 43584, Note_off_c, 9, 44, 0
//7, 43584, Note_off_c, 9, 56, 0
//7, 43776, Note_on_c, 9, 56, 37
//7, 43776, Note_on_c, 9, 44, 37
//7, 43968, Note_off_c, 9, 44, 0
//7, 43968, Note_off_c, 9, 56, 0
//7, 44160, Note_on_c, 9, 56, 37
//7, 44160, Note_on_c, 9, 44, 37
//7, 44352, Note_off_c, 9, 44, 0
//7, 44352, Note_off_c, 9, 56, 0
//7, 44544, Note_on_c, 9, 56, 37
//7, 44544, Note_on_c, 9, 44, 37
//7, 44736, Note_off_c, 9, 44, 0
//7, 44736, Note_off_c, 9, 56, 0
//7, 44928, Note_on_c, 9, 56, 37
//7, 44928, Note_on_c, 9, 44, 37
//7, 45120, Note_off_c, 9, 44, 0
//7, 45120, Note_off_c, 9, 56, 0
//7, 45312, Note_on_c, 9, 56, 37
//7, 45312, Note_on_c, 9, 44, 37
//7, 45504, Note_off_c, 9, 44, 0
//7, 45504, Note_off_c, 9, 56, 0
//7, 45696, Note_on_c, 9, 56, 37
//7, 45696, Note_on_c, 9, 44, 37
//7, 45888, Note_off_c, 9, 44, 0
//7, 45888, Note_off_c, 9, 56, 0
//7, 46080, Note_on_c, 9, 56, 37
//7, 46080, Note_on_c, 9, 44, 37
//7, 46272, Note_off_c, 9, 44, 0
//7, 46272, Note_off_c, 9, 56, 0
//7, 46464, Note_on_c, 9, 56, 37
//7, 46464, Note_on_c, 9, 44, 37
//7, 46656, Note_off_c, 9, 44, 0
//7, 46656, Note_off_c, 9, 56, 0
//7, 48576, Note_on_c, 9, 42, 37
//7, 48768, Note_off_c, 9, 42, 0
//7, 48768, Note_on_c, 9, 42, 37
//7, 48960, Note_off_c, 9, 42, 0
//7, 48960, Note_on_c, 9, 42, 37
//7, 49152, Note_off_c, 9, 42, 0
//7, 49152, Note_on_c, 9, 42, 37
//7, 49344, Note_off_c, 9, 42, 0
//7, 49344, Note_on_c, 9, 42, 37
//7, 49536, Note_off_c, 9, 42, 0
//7, 49536, Note_on_c, 9, 42, 37
//7, 49728, Note_off_c, 9, 42, 0
//7, 49728, Note_on_c, 9, 42, 37
//7, 49920, Note_off_c, 9, 42, 0
//7, 49920, Note_on_c, 9, 42, 37
//7, 50112, Note_off_c, 9, 42, 0
//7, 50112, Note_on_c, 9, 42, 37
//7, 50304, Note_off_c, 9, 42, 0
//7, 50304, Note_on_c, 9, 42, 37
//7, 50496, Note_off_c, 9, 42, 0
//7, 50496, Note_on_c, 9, 42, 37
//7, 50688, Note_off_c, 9, 42, 0
//7, 50688, Note_on_c, 9, 42, 37
//7, 50880, Note_off_c, 9, 42, 0
//7, 50880, Note_on_c, 9, 42, 37
//7, 51072, Note_off_c, 9, 42, 0
//7, 51072, Note_on_c, 9, 42, 37
//7, 51264, Note_off_c, 9, 42, 0
//7, 51264, Note_on_c, 9, 42, 37
//7, 51456, Note_off_c, 9, 42, 0
//7, 51456, Note_on_c, 9, 42, 37
//7, 51648, Note_off_c, 9, 42, 0
//7, 51648, Note_on_c, 9, 42, 37
//7, 51840, Note_off_c, 9, 42, 0
//7, 51840, Note_on_c, 9, 42, 37
//7, 52032, Note_off_c, 9, 42, 0
//7, 52032, Note_on_c, 9, 42, 37
//7, 52224, Note_off_c, 9, 42, 0
//7, 52224, Note_on_c, 9, 42, 37
//7, 52416, Note_off_c, 9, 42, 0
//7, 52416, Note_on_c, 9, 42, 37
//7, 52608, Note_off_c, 9, 42, 0
//7, 52608, Note_on_c, 9, 42, 37
//7, 52800, Note_off_c, 9, 42, 0
//7, 52800, Note_on_c, 9, 42, 37
//7, 52992, Note_off_c, 9, 42, 0
//7, 52992, Note_on_c, 9, 42, 37
//7, 53184, Note_off_c, 9, 42, 0
//7, 53184, Note_on_c, 9, 42, 37
//7, 53376, Note_off_c, 9, 42, 0
//7, 53376, Note_on_c, 9, 42, 37
//7, 53568, Note_off_c, 9, 42, 0
//7, 53568, Note_on_c, 9, 42, 37
//7, 53760, Note_off_c, 9, 42, 0
//7, 53760, Note_on_c, 9, 42, 37
//7, 53952, Note_off_c, 9, 42, 0
//7, 53952, Note_on_c, 9, 42, 37
//7, 54144, Note_off_c, 9, 42, 0
//7, 54144, Note_on_c, 9, 42, 37
//7, 54336, Note_off_c, 9, 42, 0
//7, 54720, Note_on_c, 9, 42, 37
//7, 54912, Note_off_c, 9, 42, 0
//7, 54912, Note_on_c, 9, 42, 37
//7, 55104, Note_off_c, 9, 42, 0
//7, 55104, Note_on_c, 9, 42, 37
//7, 55296, Note_off_c, 9, 42, 0
//7, 55296, Note_on_c, 9, 42, 37
//7, 55488, Note_off_c, 9, 42, 0
//7, 55488, Note_on_c, 9, 42, 37
//7, 55680, Note_off_c, 9, 42, 0
//7, 55680, Note_on_c, 9, 42, 37
//7, 55872, Note_off_c, 9, 42, 0
//7, 55872, Note_on_c, 9, 42, 37
//7, 56064, Note_off_c, 9, 42, 0
//7, 56064, Note_on_c, 9, 42, 37
//7, 56256, Note_off_c, 9, 42, 0
//7, 56256, Note_on_c, 9, 42, 37
//7, 56448, Note_off_c, 9, 42, 0
//7, 56448, Note_on_c, 9, 42, 37
//7, 56640, Note_off_c, 9, 42, 0
//7, 56640, Note_on_c, 9, 42, 37
//7, 56832, Note_off_c, 9, 42, 0
//7, 56832, Note_on_c, 9, 42, 37
//7, 57024, Note_off_c, 9, 42, 0
//7, 57024, Note_on_c, 9, 42, 37
//7, 57216, Note_off_c, 9, 42, 0
//7, 57216, Note_on_c, 9, 42, 37
//7, 57408, Note_off_c, 9, 42, 0
//7, 57408, Note_on_c, 9, 42, 37
//7, 57600, Note_off_c, 9, 42, 0
//7, 57600, Note_on_c, 9, 42, 37
//7, 57792, Note_off_c, 9, 42, 0
//7, 57792, Note_on_c, 9, 42, 37
//7, 57984, Note_off_c, 9, 42, 0
//7, 57984, Note_on_c, 9, 42, 37
//7, 58176, Note_off_c, 9, 42, 0
//7, 58176, Note_on_c, 9, 42, 37
//7, 58368, Note_off_c, 9, 42, 0
//7, 58368, Note_on_c, 9, 42, 37
//7, 58560, Note_off_c, 9, 42, 0
//7, 58560, Note_on_c, 9, 42, 37
//7, 58752, Note_off_c, 9, 42, 0
//7, 58752, Note_on_c, 9, 42, 37
//7, 58944, Note_off_c, 9, 42, 0
//7, 58944, Note_on_c, 9, 42, 37
//7, 59136, Note_off_c, 9, 42, 0
//7, 59136, Note_on_c, 9, 42, 37
//7, 59328, Note_off_c, 9, 42, 0
//7, 59328, Note_on_c, 9, 42, 37
//7, 59520, Note_off_c, 9, 42, 0
//7, 59520, Note_on_c, 9, 42, 37
//7, 59712, Note_off_c, 9, 42, 0
//7, 59712, Note_on_c, 9, 42, 37
//7, 59904, Note_off_c, 9, 42, 0
//7, 59904, Note_on_c, 9, 42, 37
//7, 60096, Note_off_c, 9, 42, 0
//7, 60096, Note_on_c, 9, 42, 37
//7, 60288, Note_off_c, 9, 42, 0
//7, 60864, Note_on_c, 9, 46, 31
//7, 61056, Note_off_c, 9, 46, 0
//7, 61056, Note_on_c, 9, 46, 31
//7, 61248, Note_off_c, 9, 46, 0
//7, 61248, Note_on_c, 9, 46, 31
//7, 61440, Note_off_c, 9, 46, 0
//7, 61440, Note_on_c, 9, 46, 31
//7, 61632, Note_off_c, 9, 46, 0
//7, 61632, Note_on_c, 9, 46, 31
//7, 61824, Note_off_c, 9, 46, 0
//7, 61824, Note_on_c, 9, 46, 31
//7, 62016, Note_off_c, 9, 46, 0
//7, 62016, Note_on_c, 9, 46, 31
//7, 62208, Note_off_c, 9, 46, 0
//7, 62400, Note_on_c, 9, 46, 31
//7, 62592, Note_off_c, 9, 46, 0
//7, 62592, Note_on_c, 9, 46, 31
//7, 62784, Note_off_c, 9, 46, 0
//7, 62784, Note_on_c, 9, 46, 31
//7, 62976, Note_off_c, 9, 46, 0
//7, 62976, Note_on_c, 9, 46, 31
//7, 63168, Note_off_c, 9, 46, 0
//7, 63168, Note_on_c, 9, 46, 31
//7, 63360, Note_off_c, 9, 46, 0
//7, 64128, Note_on_c, 9, 56, 37
//7, 64128, Note_on_c, 9, 44, 37
//7, 64320, Note_off_c, 9, 44, 0
//7, 64320, Note_off_c, 9, 56, 0
//7, 64512, Note_on_c, 9, 56, 37
//7, 64512, Note_on_c, 9, 44, 37
//7, 64704, Note_off_c, 9, 44, 0
//7, 64704, Note_off_c, 9, 56, 0
//7, 64896, Note_on_c, 9, 56, 37
//7, 64896, Note_on_c, 9, 44, 37
//7, 65088, Note_off_c, 9, 44, 0
//7, 65088, Note_off_c, 9, 56, 0
//7, 65280, Note_on_c, 9, 56, 37
//7, 65280, Note_on_c, 9, 44, 37
//7, 65472, Note_off_c, 9, 44, 0
//7, 65472, Note_off_c, 9, 56, 0
//7, 65664, Note_on_c, 9, 56, 37
//7, 65664, Note_on_c, 9, 44, 37
//7, 65856, Note_off_c, 9, 44, 0
//7, 65856, Note_off_c, 9, 56, 0
//7, 66048, Note_on_c, 9, 56, 37
//7, 66048, Note_on_c, 9, 44, 37
//7, 66240, Note_off_c, 9, 44, 0
//7, 66240, Note_off_c, 9, 56, 0
//7, 66432, Note_on_c, 9, 56, 37
//7, 66432, Note_on_c, 9, 44, 37
//7, 66624, Note_off_c, 9, 44, 0
//7, 66624, Note_off_c, 9, 56, 0
//7, 66816, Note_on_c, 9, 56, 37
//7, 66816, Note_on_c, 9, 44, 37
//7, 67008, Note_off_c, 9, 44, 0
//7, 67008, Note_off_c, 9, 56, 0
//7, 67200, Note_on_c, 9, 56, 37
//7, 67200, Note_on_c, 9, 44, 37
//7, 67392, Note_off_c, 9, 44, 0
//7, 67392, Note_off_c, 9, 56, 0
//7, 67584, Note_on_c, 9, 56, 37
//7, 67584, Note_on_c, 9, 44, 37
//7, 67776, Note_off_c, 9, 44, 0
//7, 67776, Note_off_c, 9, 56, 0
//7, 67968, Note_on_c, 9, 56, 37
//7, 67968, Note_on_c, 9, 44, 37
//7, 68160, Note_off_c, 9, 44, 0
//7, 68160, Note_off_c, 9, 56, 0
//7, 70080, Note_on_c, 9, 46, 37
//7, 70272, Note_off_c, 9, 46, 0
//7, 70272, Note_on_c, 9, 46, 31
//7, 70464, Note_off_c, 9, 46, 0
//7, 70464, Note_on_c, 9, 46, 31
//7, 70656, Note_off_c, 9, 46, 0
//7, 70656, Note_on_c, 9, 46, 31
//7, 70848, Note_off_c, 9, 46, 0
//7, 70848, Note_on_c, 9, 46, 31
//7, 71040, Note_off_c, 9, 46, 0
//7, 71040, Note_on_c, 9, 46, 31
//7, 71232, Note_off_c, 9, 46, 0
//7, 71232, Note_on_c, 9, 46, 31
//7, 71424, Note_off_c, 9, 46, 0
//7, 71424, Note_on_c, 9, 46, 31
//7, 71616, Note_off_c, 9, 46, 0
//7, 71616, Note_on_c, 9, 46, 31
//7, 71808, Note_off_c, 9, 46, 0
//7, 71808, Note_on_c, 9, 46, 31
//7, 72000, Note_off_c, 9, 46, 0
//7, 72000, Note_on_c, 9, 46, 31
//7, 72192, Note_off_c, 9, 46, 0
//7, 72192, Note_on_c, 9, 46, 31
//7, 72384, Note_off_c, 9, 46, 0
//7, 72384, Note_on_c, 9, 46, 31
//7, 72576, Note_off_c, 9, 46, 0
//7, 72576, Note_on_c, 9, 46, 31
//7, 72768, Note_off_c, 9, 46, 0
//7, 72768, Note_on_c, 9, 46, 31
//7, 72960, Note_off_c, 9, 46, 0
//7, 72960, Note_on_c, 9, 46, 31
//7, 73152, Note_off_c, 9, 46, 0
//7, 73152, Note_on_c, 9, 46, 31
//7, 73344, Note_off_c, 9, 46, 0
//7, 73344, Note_on_c, 9, 46, 31
//7, 73536, Note_off_c, 9, 46, 0
//7, 73536, Note_on_c, 9, 46, 31
//7, 73728, Note_off_c, 9, 46, 0
//7, 73728, Note_on_c, 9, 46, 31
//7, 73920, Note_off_c, 9, 46, 0
//7, 73920, Note_on_c, 9, 46, 31
//7, 74112, Note_off_c, 9, 46, 0
//7, 74112, Note_on_c, 9, 46, 31
//7, 74304, Note_off_c, 9, 46, 0
//7, 74304, Note_on_c, 9, 46, 31
//7, 74496, Note_off_c, 9, 46, 0
//7, 74496, Note_on_c, 9, 46, 31
//7, 74688, Note_off_c, 9, 46, 0
//7, 74688, Note_on_c, 9, 46, 31
//7, 74880, Note_off_c, 9, 46, 0
//7, 74880, Note_on_c, 9, 46, 31
//7, 75072, Note_off_c, 9, 46, 0
//7, 75072, Note_on_c, 9, 46, 31
//7, 75264, Note_off_c, 9, 46, 0
//7, 75264, Note_on_c, 9, 46, 31
//7, 75456, Note_off_c, 9, 46, 0
//7, 75456, Note_on_c, 9, 46, 31
//7, 75648, Note_off_c, 9, 46, 0
//7, 75648, Note_on_c, 9, 46, 31
//7, 75840, Note_off_c, 9, 46, 0
//7, 75840, Note_on_c, 9, 46, 31
//7, 76032, Note_off_c, 9, 46, 0
//7, 76032, Note_on_c, 9, 46, 31
//7, 76224, Note_off_c, 9, 46, 0
//7, 76224, Note_on_c, 9, 46, 31
//7, 76416, Note_off_c, 9, 46, 0
//7, 76416, Note_on_c, 9, 46, 31
//7, 76608, Note_off_c, 9, 46, 0
//7, 76608, Note_on_c, 9, 46, 31
//7, 76800, Note_off_c, 9, 46, 0
//7, 76800, Note_on_c, 9, 46, 31
//7, 76992, Note_off_c, 9, 46, 0
//7, 76992, Note_on_c, 9, 46, 31
//7, 77184, Note_off_c, 9, 46, 0
//7, 77184, Note_on_c, 9, 46, 31
//7, 77376, Note_off_c, 9, 46, 0
//7, 77376, Note_on_c, 9, 46, 31
//7, 77568, Note_off_c, 9, 46, 0
//7, 77568, Note_on_c, 9, 46, 31
//7, 77760, Note_off_c, 9, 46, 0
//7, 77760, Note_on_c, 9, 46, 31
//7, 77952, Note_off_c, 9, 46, 0
//7, 77952, Note_on_c, 9, 46, 31
//7, 78144, Note_off_c, 9, 46, 0
//7, 78144, Note_on_c, 9, 46, 31
//7, 78336, Note_off_c, 9, 46, 0
//7, 78336, Note_on_c, 9, 46, 31
//7, 78528, Note_off_c, 9, 46, 0
//7, 78528, Note_on_c, 9, 46, 31
//7, 78720, Note_off_c, 9, 46, 0
//7, 78720, Note_on_c, 9, 46, 31
//7, 78912, Note_off_c, 9, 46, 0
//7, 78912, Note_on_c, 9, 46, 31
//7, 79104, Note_off_c, 9, 46, 0
//7, 79104, Note_on_c, 9, 46, 31
//7, 79296, Note_off_c, 9, 46, 0
//7, 79296, Note_on_c, 9, 46, 31
//7, 79488, Note_off_c, 9, 46, 0
//7, 79488, Note_on_c, 9, 46, 31
//7, 79680, Note_off_c, 9, 46, 0
//7, 79680, Note_on_c, 9, 46, 31
//7, 79872, Note_off_c, 9, 46, 0
//7, 79872, Note_on_c, 9, 46, 31
//7, 80064, Note_off_c, 9, 46, 0
//7, 80064, Note_on_c, 9, 46, 31
//7, 80256, Note_off_c, 9, 46, 0
//7, 80256, Note_on_c, 9, 46, 31
//7, 80448, Note_off_c, 9, 46, 0
//7, 80448, Note_on_c, 9, 46, 31
//7, 80640, Note_off_c, 9, 46, 0
//7, 80640, Note_on_c, 9, 46, 31
//7, 80832, Note_off_c, 9, 46, 0
//7, 80832, Note_on_c, 9, 46, 31
//7, 81024, Note_off_c, 9, 46, 0
//7, 81024, Note_on_c, 9, 46, 31
//7, 81216, Note_off_c, 9, 46, 0
//7, 81216, Note_on_c, 9, 46, 31
//7, 81408, Note_off_c, 9, 46, 0
//7, 81408, Note_on_c, 9, 46, 31
//7, 81600, Note_off_c, 9, 46, 0
//7, 81600, Note_on_c, 9, 46, 31
//7, 81792, Note_off_c, 9, 46, 0
//7, 81792, Note_on_c, 9, 46, 31
//7, 81984, Note_off_c, 9, 46, 0
//7, 82368, Note_on_c, 9, 42, 37
//7, 82560, Note_off_c, 9, 42, 0
//7, 82560, Note_on_c, 9, 42, 37
//7, 82752, Note_off_c, 9, 42, 0
//7, 82752, Note_on_c, 9, 42, 37
//7, 82944, Note_off_c, 9, 42, 0
//7, 82944, Note_on_c, 9, 42, 37
//7, 83136, Note_off_c, 9, 42, 0
//7, 83136, Note_on_c, 9, 42, 37
//7, 83328, Note_off_c, 9, 42, 0
//7, 83328, Note_on_c, 9, 42, 37
//7, 83520, Note_off_c, 9, 42, 0
//7, 83520, Note_on_c, 9, 42, 37
//7, 83712, Note_off_c, 9, 42, 0
//7, 83712, Note_on_c, 9, 42, 37
//7, 83904, Note_off_c, 9, 42, 0
//7, 83904, Note_on_c, 9, 42, 37
//7, 84096, Note_off_c, 9, 42, 0
//7, 84096, Note_on_c, 9, 42, 37
//7, 84288, Note_off_c, 9, 42, 0
//7, 84288, Note_on_c, 9, 42, 37
//7, 84480, Note_off_c, 9, 42, 0
//7, 84480, Note_on_c, 9, 42, 37
//7, 84672, Note_off_c, 9, 42, 0
//7, 84672, Note_on_c, 9, 42, 37
//7, 84864, Note_off_c, 9, 42, 0
//7, 84864, Note_on_c, 9, 42, 37
//7, 85056, Note_off_c, 9, 42, 0
//7, 85056, Note_on_c, 9, 42, 37
//7, 85248, Note_off_c, 9, 42, 0
//7, 85248, Note_on_c, 9, 42, 37
//7, 85440, Note_off_c, 9, 42, 0
//7, 85440, Note_on_c, 9, 42, 37
//7, 85632, Note_off_c, 9, 42, 0
//7, 85632, Note_on_c, 9, 42, 37
//7, 85824, Note_off_c, 9, 42, 0
//7, 85824, Note_on_c, 9, 42, 37
//7, 86016, Note_off_c, 9, 42, 0
//7, 86016, Note_on_c, 9, 42, 37
//7, 86208, Note_off_c, 9, 42, 0
//7, 86208, Note_on_c, 9, 42, 37
//7, 86400, Note_off_c, 9, 42, 0
//7, 86400, Note_on_c, 9, 42, 37
//7, 86592, Note_off_c, 9, 42, 0
//7, 86592, Note_on_c, 9, 42, 37
//7, 86784, Note_off_c, 9, 42, 0
//7, 86784, Note_on_c, 9, 42, 37
//7, 86976, Note_off_c, 9, 42, 0
//7, 86976, Note_on_c, 9, 42, 37
//7, 87168, Note_off_c, 9, 42, 0
//7, 87168, Note_on_c, 9, 42, 37
//7, 87360, Note_off_c, 9, 42, 0
//7, 87360, Note_on_c, 9, 42, 37
//7, 87552, Note_off_c, 9, 42, 0
//7, 87552, Note_on_c, 9, 42, 37
//7, 87744, Note_off_c, 9, 42, 0
//7, 87744, Note_on_c, 9, 42, 37
//7, 87936, Note_off_c, 9, 42, 0
//7, 87936, Note_on_c, 9, 42, 37
//7, 88128, Note_off_c, 9, 42, 0
//7, 88512, Note_on_c, 9, 42, 37
//7, 88704, Note_off_c, 9, 42, 0
//7, 88704, Note_on_c, 9, 42, 37
//7, 88896, Note_off_c, 9, 42, 0
//7, 88896, Note_on_c, 9, 42, 37
//7, 89088, Note_off_c, 9, 42, 0
//7, 89088, Note_on_c, 9, 42, 37
//7, 89280, Note_off_c, 9, 42, 0
//7, 89280, Note_on_c, 9, 42, 37
//7, 89472, Note_off_c, 9, 42, 0
//7, 89472, Note_on_c, 9, 42, 37
//7, 89664, Note_off_c, 9, 42, 0
//7, 89664, Note_on_c, 9, 42, 37
//7, 89856, Note_off_c, 9, 42, 0
//7, 89856, Note_on_c, 9, 42, 37
//7, 90048, Note_off_c, 9, 42, 0
//7, 90048, Note_on_c, 9, 42, 37
//7, 90240, Note_off_c, 9, 42, 0
//7, 90240, Note_on_c, 9, 42, 37
//7, 90432, Note_off_c, 9, 42, 0
//7, 90432, Note_on_c, 9, 42, 37
//7, 90624, Note_off_c, 9, 42, 0
//7, 90624, Note_on_c, 9, 42, 37
//7, 90816, Note_off_c, 9, 42, 0
//7, 90816, Note_on_c, 9, 42, 37
//7, 91008, Note_off_c, 9, 42, 0
//7, 91008, Note_on_c, 9, 42, 37
//7, 91200, Note_off_c, 9, 42, 0
//7, 91200, Note_on_c, 9, 42, 37
//7, 91392, Note_off_c, 9, 42, 0
//7, 91392, Note_on_c, 9, 42, 37
//7, 91584, Note_off_c, 9, 42, 0
//7, 91584, Note_on_c, 9, 42, 37
//7, 91776, Note_off_c, 9, 42, 0
//7, 91776, Note_on_c, 9, 42, 37
//7, 91968, Note_off_c, 9, 42, 0
//7, 91968, Note_on_c, 9, 42, 37
//7, 92160, Note_off_c, 9, 42, 0
//7, 92160, Note_on_c, 9, 42, 37
//7, 92352, Note_off_c, 9, 42, 0
//7, 92352, Note_on_c, 9, 42, 37
//7, 92544, Note_off_c, 9, 42, 0
//7, 92544, Note_on_c, 9, 42, 37
//7, 92736, Note_off_c, 9, 42, 0
//7, 92736, Note_on_c, 9, 42, 37
//7, 92928, Note_off_c, 9, 42, 0
//7, 92928, Note_on_c, 9, 42, 37
//7, 93120, Note_off_c, 9, 42, 0
//7, 93120, Note_on_c, 9, 42, 37
//7, 93312, Note_off_c, 9, 42, 0
//7, 93312, Note_on_c, 9, 42, 37
//7, 93504, Note_off_c, 9, 42, 0
//7, 93504, Note_on_c, 9, 42, 37
//7, 93696, Note_off_c, 9, 42, 0
//7, 93696, Note_on_c, 9, 42, 37
//7, 93888, Note_off_c, 9, 42, 0
//7, 93888, Note_on_c, 9, 42, 37
//7, 94080, Note_off_c, 9, 42, 0
//7, 94656, Note_on_c, 9, 46, 31
//7, 94848, Note_off_c, 9, 46, 0
//7, 94848, Note_on_c, 9, 46, 31
//7, 95040, Note_off_c, 9, 46, 0
//7, 95040, Note_on_c, 9, 46, 31
//7, 95232, Note_off_c, 9, 46, 0
//7, 95232, Note_on_c, 9, 46, 31
//7, 95424, Note_off_c, 9, 46, 0
//7, 95424, Note_on_c, 9, 46, 31
//7, 95616, Note_off_c, 9, 46, 0
//7, 95616, Note_on_c, 9, 46, 31
//7, 95808, Note_off_c, 9, 46, 0
//7, 95808, Note_on_c, 9, 46, 31
//7, 96000, Note_off_c, 9, 46, 0
//7, 96192, Note_on_c, 9, 46, 31
//7, 96384, Note_off_c, 9, 46, 0
//7, 96384, Note_on_c, 9, 46, 31
//7, 96576, Note_off_c, 9, 46, 0
//7, 96576, Note_on_c, 9, 46, 31
//7, 96768, Note_off_c, 9, 46, 0
//7, 96768, Note_on_c, 9, 46, 31
//7, 96960, Note_off_c, 9, 46, 0
//7, 96960, Note_on_c, 9, 46, 31
//7, 97152, Note_off_c, 9, 46, 0
//7, 97920, Note_on_c, 9, 56, 37
//7, 97920, Note_on_c, 9, 44, 37
//7, 98112, Note_off_c, 9, 44, 0
//7, 98112, Note_off_c, 9, 56, 0
//7, 98304, Note_on_c, 9, 56, 37
//7, 98304, Note_on_c, 9, 44, 37
//7, 98496, Note_off_c, 9, 44, 0
//7, 98496, Note_off_c, 9, 56, 0
//7, 98688, Note_on_c, 9, 56, 37
//7, 98688, Note_on_c, 9, 44, 37
//7, 98880, Note_off_c, 9, 44, 0
//7, 98880, Note_off_c, 9, 56, 0
//7, 99072, Note_on_c, 9, 56, 37
//7, 99072, Note_on_c, 9, 44, 37
//7, 99264, Note_off_c, 9, 44, 0
//7, 99264, Note_off_c, 9, 56, 0
//7, 99456, Note_on_c, 9, 56, 37
//7, 99456, Note_on_c, 9, 44, 37
//7, 99648, Note_off_c, 9, 44, 0
//7, 99648, Note_off_c, 9, 56, 0
//7, 99840, Note_on_c, 9, 56, 37
//7, 99840, Note_on_c, 9, 44, 37
//7, 100032, Note_off_c, 9, 44, 0
//7, 100032, Note_off_c, 9, 56, 0
//7, 100224, Note_on_c, 9, 56, 37
//7, 100224, Note_on_c, 9, 44, 37
//7, 100416, Note_off_c, 9, 44, 0
//7, 100416, Note_off_c, 9, 56, 0
//7, 100608, Note_on_c, 9, 56, 37
//7, 100608, Note_on_c, 9, 44, 37
//7, 100800, Note_off_c, 9, 44, 0
//7, 100800, Note_off_c, 9, 56, 0
//7, 100992, Note_on_c, 9, 56, 37
//7, 100992, Note_on_c, 9, 44, 37
//7, 101184, Note_off_c, 9, 44, 0
//7, 101184, Note_off_c, 9, 56, 0
//7, 101376, Note_on_c, 9, 56, 37
//7, 101376, Note_on_c, 9, 44, 37
//7, 101568, Note_off_c, 9, 44, 0
//7, 101568, Note_off_c, 9, 56, 0
//7, 101760, Note_on_c, 9, 56, 37
//7, 101760, Note_on_c, 9, 44, 37
//7, 101952, Note_off_c, 9, 44, 0
//7, 101952, Note_off_c, 9, 56, 0
//7, 104064, Note_on_c, 9, 51, 37
//7, 104448, Note_off_c, 9, 51, 0
//7, 104448, Note_on_c, 9, 51, 37
//7, 104640, Note_off_c, 9, 51, 0
//7, 104832, Note_on_c, 9, 51, 37
//7, 105216, Note_off_c, 9, 51, 0
//7, 105216, Note_on_c, 9, 51, 37
//7, 105600, Note_off_c, 9, 51, 0
//7, 105600, Note_on_c, 9, 51, 37
//7, 105792, Note_off_c, 9, 51, 0
//7, 105984, Note_on_c, 9, 51, 37
//7, 106176, Note_off_c, 9, 51, 0
//7, 106368, Note_on_c, 9, 51, 37
//7, 106752, Note_off_c, 9, 51, 0
//7, 106752, Note_on_c, 9, 51, 37
//7, 107136, Note_off_c, 9, 51, 0
//7, 107136, Note_on_c, 9, 51, 37
//7, 107520, Note_off_c, 9, 51, 0
//7, 107520, Note_on_c, 9, 51, 37
//7, 107712, Note_off_c, 9, 51, 0
//7, 107904, Note_on_c, 9, 51, 37
//7, 108288, Note_off_c, 9, 51, 0
//7, 108288, Note_on_c, 9, 51, 37
//7, 108672, Note_off_c, 9, 51, 0
//7, 108672, Note_on_c, 9, 51, 37
//7, 108864, Note_off_c, 9, 51, 0
//7, 109056, Note_on_c, 9, 51, 37
//7, 109248, Note_off_c, 9, 51, 0
//7, 109440, Note_on_c, 9, 51, 37
//7, 109824, Note_off_c, 9, 51, 0
//7, 109824, Note_on_c, 9, 51, 37
//7, 110208, Note_off_c, 9, 51, 0
//7, 110208, Note_on_c, 9, 51, 37
//7, 110592, Note_off_c, 9, 51, 0
//7, 110592, Note_on_c, 9, 51, 37
//7, 110784, Note_off_c, 9, 51, 0
//7, 110976, Note_on_c, 9, 51, 37
//7, 111360, Note_off_c, 9, 51, 0
//7, 111360, Note_on_c, 9, 51, 37
//7, 111744, Note_off_c, 9, 51, 0
//7, 111744, Note_on_c, 9, 51, 37
//7, 111936, Note_off_c, 9, 51, 0
//7, 112128, Note_on_c, 9, 51, 37
//7, 112320, Note_off_c, 9, 51, 0
//7, 112512, Note_on_c, 9, 51, 37
//7, 112896, Note_off_c, 9, 51, 0
//7, 112896, Note_on_c, 9, 51, 37
//7, 113280, Note_off_c, 9, 51, 0
//7, 113280, Note_on_c, 9, 51, 37
//7, 113664, Note_off_c, 9, 51, 0
//7, 113664, Note_on_c, 9, 51, 37
//7, 113856, Note_off_c, 9, 51, 0
//7, 120768, Note_on_c, 9, 46, 37
//7, 120960, Note_off_c, 9, 46, 0
//7, 120960, Note_on_c, 9, 46, 31
//7, 121152, Note_off_c, 9, 46, 0
//7, 121152, Note_on_c, 9, 46, 31
//7, 121344, Note_off_c, 9, 46, 0
//7, 121344, Note_on_c, 9, 46, 31
//7, 121536, Note_off_c, 9, 46, 0
//7, 121536, Note_on_c, 9, 46, 31
//7, 121728, Note_off_c, 9, 46, 0
//7, 121728, Note_on_c, 9, 46, 31
//7, 121920, Note_off_c, 9, 46, 0
//7, 121920, Note_on_c, 9, 46, 31
//7, 122112, Note_off_c, 9, 46, 0
//7, 122112, Note_on_c, 9, 46, 31
//7, 122304, Note_off_c, 9, 46, 0
//7, 122304, Note_on_c, 9, 46, 31
//7, 122496, Note_off_c, 9, 46, 0
//7, 122496, Note_on_c, 9, 46, 31
//7, 122688, Note_off_c, 9, 46, 0
//7, 122688, Note_on_c, 9, 46, 31
//7, 122880, Note_off_c, 9, 46, 0
//7, 122880, Note_on_c, 9, 46, 31
//7, 123072, Note_off_c, 9, 46, 0
//7, 123072, Note_on_c, 9, 46, 31
//7, 123264, Note_off_c, 9, 46, 0
//7, 123264, Note_on_c, 9, 46, 31
//7, 123456, Note_off_c, 9, 46, 0
//7, 123456, Note_on_c, 9, 46, 31
//7, 123648, Note_off_c, 9, 46, 0
//7, 123648, Note_on_c, 9, 46, 31
//7, 123840, Note_off_c, 9, 46, 0
//7, 123840, Note_on_c, 9, 46, 31
//7, 124032, Note_off_c, 9, 46, 0
//7, 124032, Note_on_c, 9, 46, 31
//7, 124224, Note_off_c, 9, 46, 0
//7, 124224, Note_on_c, 9, 46, 31
//7, 124416, Note_off_c, 9, 46, 0
//7, 124416, Note_on_c, 9, 46, 31
//7, 124608, Note_off_c, 9, 46, 0
//7, 124608, Note_on_c, 9, 46, 31
//7, 124800, Note_off_c, 9, 46, 0
//7, 124800, Note_on_c, 9, 46, 31
//7, 124992, Note_off_c, 9, 46, 0
//7, 124992, Note_on_c, 9, 46, 31
//7, 125184, Note_off_c, 9, 46, 0
//7, 125184, Note_on_c, 9, 46, 31
//7, 125376, Note_off_c, 9, 46, 0
//7, 125376, Note_on_c, 9, 46, 31
//7, 125568, Note_off_c, 9, 46, 0
//7, 125568, Note_on_c, 9, 46, 31
//7, 125760, Note_off_c, 9, 46, 0
//7, 125760, Note_on_c, 9, 46, 31
//7, 125952, Note_off_c, 9, 46, 0
//7, 126912, Note_on_c, 9, 51, 37
//7, 127104, Note_off_c, 9, 51, 0
//7, 127104, Note_on_c, 9, 51, 37
//7, 127296, Note_off_c, 9, 51, 0
//7, 127296, Note_on_c, 9, 51, 37
//7, 127488, Note_off_c, 9, 51, 0
//7, 127488, Note_on_c, 9, 51, 37
//7, 127680, Note_off_c, 9, 51, 0
//7, 127680, Note_on_c, 9, 51, 37
//7, 127872, Note_off_c, 9, 51, 0
//7, 127872, Note_on_c, 9, 51, 37
//7, 128064, Note_off_c, 9, 51, 0
//7, 128256, Note_on_c, 9, 51, 37
//7, 128448, Note_off_c, 9, 51, 0
//7, 128448, Note_on_c, 9, 51, 37
//7, 128640, Note_off_c, 9, 51, 0
//7, 128640, Note_on_c, 9, 51, 37
//7, 128832, Note_off_c, 9, 51, 0
//7, 128832, Note_on_c, 9, 51, 37
//7, 129024, Note_off_c, 9, 51, 0
//7, 129024, Note_on_c, 9, 51, 37
//7, 129216, Note_off_c, 9, 51, 0
//7, 129216, Note_on_c, 9, 51, 37
//7, 129408, Note_off_c, 9, 51, 0
//7, 129408, Note_on_c, 9, 51, 37
//7, 129600, Note_off_c, 9, 51, 0
//7, 129600, Note_on_c, 9, 51, 37
//7, 129792, Note_off_c, 9, 51, 0
//7, 129984, Note_on_c, 9, 51, 37
//7, 130176, Note_off_c, 9, 51, 0
//7, 130176, Note_on_c, 9, 51, 37
//7, 130368, Note_off_c, 9, 51, 0
//7, 130368, Note_on_c, 9, 51, 37
//7, 130560, Note_off_c, 9, 51, 0
//7, 130560, Note_on_c, 9, 51, 37
//7, 130752, Note_off_c, 9, 51, 0
//7, 130752, Note_on_c, 9, 51, 37
//7, 130944, Note_off_c, 9, 51, 0
//7, 130944, Note_on_c, 9, 51, 37
//7, 131136, Note_off_c, 9, 51, 0
//7, 131328, Note_on_c, 9, 51, 37
//7, 131520, Note_off_c, 9, 51, 0
//7, 131520, Note_on_c, 9, 51, 37
//7, 131712, Note_off_c, 9, 51, 0
//7, 131712, Note_on_c, 9, 51, 37
//7, 131904, Note_off_c, 9, 51, 0
//7, 131904, Note_on_c, 9, 51, 37
//7, 132096, Note_off_c, 9, 51, 0
//7, 132096, Note_on_c, 9, 51, 37
//7, 132288, Note_off_c, 9, 51, 0
//7, 132288, Note_on_c, 9, 51, 37
//7, 132480, Note_off_c, 9, 51, 0
//7, 132480, Note_on_c, 9, 51, 37
//7, 132672, Note_off_c, 9, 51, 0
//7, 132672, Note_on_c, 9, 51, 37
//7, 132864, Note_off_c, 9, 51, 0
//7, 132864, Note_on_c, 9, 51, 37
//7, 133056, Note_off_c, 9, 51, 0
//7, 133056, Note_on_c, 9, 51, 37
//7, 133248, Note_off_c, 9, 51, 0
//7, 133248, Note_on_c, 9, 51, 37
//7, 133440, Note_off_c, 9, 51, 0
//7, 133440, Note_on_c, 9, 51, 37
//7, 133632, Note_off_c, 9, 51, 0
//7, 133632, Note_on_c, 9, 51, 37
//7, 133824, Note_off_c, 9, 51, 0
//7, 133824, Note_on_c, 9, 51, 37
//7, 134016, Note_off_c, 9, 51, 0
//7, 134016, Note_on_c, 9, 51, 37
//7, 134208, Note_off_c, 9, 51, 0
//7, 134400, Note_on_c, 9, 51, 37
//7, 134592, Note_off_c, 9, 51, 0
//7, 134592, Note_on_c, 9, 51, 37
//7, 134784, Note_off_c, 9, 51, 0
//7, 134784, Note_on_c, 9, 51, 37
//7, 134976, Note_off_c, 9, 51, 0
//7, 134976, Note_on_c, 9, 51, 37
//7, 135168, Note_off_c, 9, 51, 0
//7, 135936, Note_on_c, 9, 51, 37
//7, 136128, Note_off_c, 9, 51, 0
//7, 136128, Note_on_c, 9, 51, 37
//7, 136320, Note_off_c, 9, 51, 0
//7, 136512, Note_on_c, 9, 51, 37
//7, 136704, Note_off_c, 9, 51, 0
//7, 136704, Note_on_c, 9, 51, 37
//7, 136896, Note_off_c, 9, 51, 0
//7, 136896, Note_on_c, 9, 51, 37
//7, 137088, Note_off_c, 9, 51, 0
//7, 137280, Note_on_c, 9, 51, 37
//7, 137472, Note_off_c, 9, 51, 0
//7, 137472, Note_on_c, 9, 51, 37
//7, 137664, Note_off_c, 9, 51, 0
//7, 137664, Note_on_c, 9, 51, 37
//7, 137856, Note_off_c, 9, 51, 0
//7, 138048, Note_on_c, 9, 51, 37
//7, 138240, Note_off_c, 9, 51, 0
//7, 138240, Note_on_c, 9, 51, 37
//7, 138432, Note_off_c, 9, 51, 0
//7, 138624, Note_on_c, 9, 51, 37
//7, 138816, Note_off_c, 9, 51, 0
//7, 138816, Note_on_c, 9, 51, 37
//7, 139008, Note_off_c, 9, 51, 0
//7, 139200, Note_on_c, 9, 51, 37
//7, 139392, Note_off_c, 9, 51, 0
//7, 139392, Note_on_c, 9, 51, 37
//7, 139584, Note_off_c, 9, 51, 0
//7, 139584, Note_on_c, 9, 51, 37
//7, 139776, Note_off_c, 9, 51, 0
//7, 139776, Note_on_c, 9, 51, 37
//7, 139968, Note_off_c, 9, 51, 0
//7, 139968, Note_on_c, 9, 51, 37
//7, 140160, Note_off_c, 9, 51, 0
//7, 140160, Note_on_c, 9, 51, 37
//7, 140352, Note_off_c, 9, 51, 0
//7, 140352, Note_on_c, 9, 51, 37
//7, 140544, Note_off_c, 9, 51, 0
//7, 140544, Note_on_c, 9, 51, 37
//7, 140736, Note_off_c, 9, 51, 0
//7, 140736, Note_on_c, 9, 51, 37
//7, 140928, Note_off_c, 9, 51, 0
//7, 140928, Note_on_c, 9, 51, 37
//7, 141120, Note_off_c, 9, 51, 0
//7, 141120, Note_on_c, 9, 51, 37
//7, 141312, Note_off_c, 9, 51, 0
//7, 141312, Note_on_c, 9, 51, 37
//7, 141504, Note_off_c, 9, 51, 0
//7, 141504, Note_on_c, 9, 51, 37
//7, 141696, Note_off_c, 9, 51, 0
//7, 141696, Note_on_c, 9, 51, 37
//7, 141888, Note_off_c, 9, 51, 0
//7, 142080, Note_on_c, 9, 51, 37
//7, 142272, Note_off_c, 9, 51, 0
//7, 142272, Note_on_c, 9, 51, 37
//7, 142464, Note_off_c, 9, 51, 0
//7, 142464, Note_on_c, 9, 51, 37
//7, 142656, Note_off_c, 9, 51, 0
//7, 142656, Note_on_c, 9, 51, 37
//7, 142848, Note_off_c, 9, 51, 0
//7, 142848, Note_on_c, 9, 51, 37
//7, 143040, Note_off_c, 9, 51, 0
//7, 143040, Note_on_c, 9, 51, 37
//7, 143232, Note_off_c, 9, 51, 0
//7, 143232, Note_on_c, 9, 51, 37
//7, 143424, Note_off_c, 9, 51, 0
//7, 143424, Note_on_c, 9, 51, 37
//7, 143616, Note_off_c, 9, 51, 0
//7, 143616, Note_on_c, 9, 51, 37
//7, 143808, Note_off_c, 9, 51, 0
//7, 143808, Note_on_c, 9, 51, 37
//7, 144000, Note_off_c, 9, 51, 0
//7, 144000, Note_on_c, 9, 51, 37
//7, 144192, Note_off_c, 9, 51, 0
//7, 144192, Note_on_c, 9, 51, 37
//7, 144384, Note_off_c, 9, 51, 0
//7, 144384, Note_on_c, 9, 51, 37
//7, 144576, Note_off_c, 9, 51, 0
//7, 144576, Note_on_c, 9, 51, 37
//7, 144768, Note_off_c, 9, 51, 0
//7, 144768, Note_on_c, 9, 51, 37
//7, 144960, Note_off_c, 9, 51, 0
//7, 145152, Note_on_c, 9, 51, 37
//7, 145344, Note_off_c, 9, 51, 0
//7, 145344, Note_on_c, 9, 51, 37
//7, 145536, Note_off_c, 9, 51, 0
//7, 145536, Note_on_c, 9, 51, 37
//7, 145728, Note_off_c, 9, 51, 0
//7, 145728, Note_on_c, 9, 51, 37
//7, 145920, Note_off_c, 9, 51, 0
//7, 145920, Note_on_c, 9, 51, 37
//7, 146112, Note_off_c, 9, 51, 0
//7, 146112, Note_on_c, 9, 51, 37
//7, 146304, Note_off_c, 9, 51, 0
//7, 146304, Note_on_c, 9, 51, 37
//7, 146496, Note_off_c, 9, 51, 0
//7, 146496, Note_on_c, 9, 51, 37
//7, 146688, Note_off_c, 9, 51, 0
//7, 146688, Note_on_c, 9, 51, 37
//7, 146880, Note_off_c, 9, 51, 0
//7, 146880, Note_on_c, 9, 51, 37
//7, 147072, Note_off_c, 9, 51, 0
//7, 147072, Note_on_c, 9, 51, 37
//7, 147264, Note_off_c, 9, 51, 0
//7, 147264, Note_on_c, 9, 51, 37
//7, 147456, Note_off_c, 9, 51, 0
//7, 153984, Note_on_c, 9, 56, 37
//7, 154080, Note_off_c, 9, 56, 0
//7, 154080, Note_on_c, 9, 56, 37
//7, 154176, Note_off_c, 9, 56, 0
//7, 154176, Note_on_c, 9, 56, 37
//7, 154272, Note_off_c, 9, 56, 0
//7, 154272, Note_on_c, 9, 56, 37
//7, 154368, Note_off_c, 9, 56, 0
//7, 154464, Note_on_c, 9, 44, 37
//7, 154560, Note_off_c, 9, 44, 0
//7, 154560, Note_on_c, 9, 44, 37
//7, 154656, Note_off_c, 9, 44, 0
//7, 154656, Note_on_c, 9, 44, 43
//7, 154752, Note_off_c, 9, 44, 0
//7, 154752, Note_on_c, 9, 44, 43
//7, 154848, Note_off_c, 9, 44, 0
//7, 154848, Note_on_c, 9, 44, 37
//7, 154944, Note_off_c, 9, 44, 0
//7, 154944, Note_on_c, 9, 44, 37
//7, 155040, Note_off_c, 9, 44, 0
//7, 155040, Note_on_c, 9, 44, 43
//7, 155136, Note_off_c, 9, 44, 0
//7, 155136, Note_on_c, 9, 44, 37
//7, 155232, Note_off_c, 9, 44, 0
//7, 155232, Note_on_c, 9, 44, 37
//7, 155328, Note_off_c, 9, 44, 0
//7, 155328, Note_on_c, 9, 44, 37
//7, 155424, Note_off_c, 9, 44, 0
//7, 155424, Note_on_c, 9, 44, 43
//7, 155520, Note_off_c, 9, 44, 0
//7, 155520, Note_on_c, 9, 44, 43
//7, 155616, Note_off_c, 9, 44, 0
//7, 155616, Note_on_c, 9, 44, 37
//7, 155712, Note_off_c, 9, 44, 0
//7, 155712, Note_on_c, 9, 44, 37
//7, 155808, Note_off_c, 9, 44, 0
//7, 155808, Note_on_c, 9, 44, 43
//7, 155904, Note_off_c, 9, 44, 0
//7, 155904, Note_on_c, 9, 44, 37
//7, 156000, Note_off_c, 9, 44, 0
//7, 156000, Note_on_c, 9, 44, 37
//7, 156096, Note_off_c, 9, 44, 0
//7, 156096, Note_on_c, 9, 44, 37
//7, 156192, Note_off_c, 9, 44, 0
//7, 156192, Note_on_c, 9, 44, 43
//7, 156288, Note_off_c, 9, 44, 0
//7, 156288, Note_on_c, 9, 44, 43
//7, 156384, Note_off_c, 9, 44, 0
//7, 156384, Note_on_c, 9, 44, 37
//7, 156480, Note_off_c, 9, 44, 0
//7, 156480, Note_on_c, 9, 44, 37
//7, 156576, Note_off_c, 9, 44, 0
//7, 156576, Note_on_c, 9, 44, 43
//7, 156672, Note_off_c, 9, 44, 0
//7, 156672, Note_on_c, 9, 44, 37
//7, 156768, Note_off_c, 9, 44, 0
//7, 156768, Note_on_c, 9, 44, 37
//7, 156864, Note_off_c, 9, 44, 0
//7, 156864, Note_on_c, 9, 44, 37
//7, 156960, Note_off_c, 9, 44, 0
//7, 156960, Note_on_c, 9, 44, 43
//7, 157056, Note_off_c, 9, 44, 0
//7, 157056, Note_on_c, 9, 44, 43
//7, 157152, Note_off_c, 9, 44, 0
//7, 157152, Note_on_c, 9, 44, 37
//7, 157248, Note_off_c, 9, 44, 0
//7, 157248, Note_on_c, 9, 44, 37
//7, 157344, Note_off_c, 9, 44, 0
//7, 157344, Note_on_c, 9, 44, 43
//7, 157440, Note_off_c, 9, 44, 0
//7, 157440, Note_on_c, 9, 44, 37
//7, 157536, Note_off_c, 9, 44, 0
//7, 157536, Note_on_c, 9, 44, 37
//7, 157632, Note_off_c, 9, 44, 0
//7, 157632, Note_on_c, 9, 44, 37
//7, 157728, Note_off_c, 9, 44, 0
//7, 157728, Note_on_c, 9, 44, 43
//7, 157824, Note_off_c, 9, 44, 0
//7, 157824, Note_on_c, 9, 44, 43
//7, 157920, Note_off_c, 9, 44, 0
//7, 157920, Note_on_c, 9, 44, 37
//7, 158016, Note_off_c, 9, 44, 0
//7, 158016, Note_on_c, 9, 44, 37
//7, 158112, Note_off_c, 9, 44, 0
//7, 158112, Note_on_c, 9, 44, 43
//7, 158208, Note_off_c, 9, 44, 0
//7, 158208, Note_on_c, 9, 44, 37
//7, 158304, Note_off_c, 9, 44, 0
//7, 158304, Note_on_c, 9, 44, 37
//7, 158400, Note_off_c, 9, 44, 0
//7, 158400, Note_on_c, 9, 44, 37
//7, 158496, Note_off_c, 9, 44, 0
//7, 158496, Note_on_c, 9, 44, 43
//7, 158592, Note_off_c, 9, 44, 0
//7, 158592, Note_on_c, 9, 44, 43
//7, 158688, Note_off_c, 9, 44, 0
//7, 158688, Note_on_c, 9, 44, 37
//7, 158784, Note_off_c, 9, 44, 0
//7, 158784, Note_on_c, 9, 44, 37
//7, 158880, Note_off_c, 9, 44, 0
//7, 158880, Note_on_c, 9, 44, 43
//7, 158976, Note_off_c, 9, 44, 0
//7, 158976, Note_on_c, 9, 44, 37
//7, 159072, Note_off_c, 9, 44, 0
//7, 159072, Note_on_c, 9, 44, 37
//7, 159168, Note_off_c, 9, 44, 0
//7, 159168, Note_on_c, 9, 44, 37
//7, 159264, Note_off_c, 9, 44, 0
//7, 159264, Note_on_c, 9, 44, 43
//7, 159360, Note_off_c, 9, 44, 0
//7, 159360, Note_on_c, 9, 44, 43
//7, 159456, Note_off_c, 9, 44, 0
//7, 159456, Note_on_c, 9, 44, 37
//7, 159552, Note_off_c, 9, 44, 0
//7, 159552, Note_on_c, 9, 44, 37
//7, 159648, Note_off_c, 9, 44, 0
//7, 159648, Note_on_c, 9, 44, 43
//7, 159744, Note_off_c, 9, 44, 0
//7, 159744, Note_on_c, 9, 44, 37
//7, 159840, Note_off_c, 9, 44, 0
//7, 159840, Note_on_c, 9, 44, 37
//7, 159936, Note_off_c, 9, 44, 0
//7, 159936, Note_on_c, 9, 44, 37
//7, 160032, Note_off_c, 9, 44, 0
//7, 160032, Note_on_c, 9, 44, 43
//7, 160128, Note_off_c, 9, 44, 0
//7, 160128, Note_on_c, 9, 44, 43
//7, 160224, Note_off_c, 9, 44, 0
//7, 160224, Note_on_c, 9, 44, 37
//7, 160320, Note_off_c, 9, 44, 0
//7, 160320, Note_on_c, 9, 44, 37
//7, 160416, Note_off_c, 9, 44, 0
//7, 160416, Note_on_c, 9, 44, 43
//7, 160512, Note_off_c, 9, 44, 0
//7, 160512, Note_on_c, 9, 44, 37
//7, 160608, Note_off_c, 9, 44, 0
//7, 160608, Note_on_c, 9, 44, 37
//7, 160704, Note_off_c, 9, 44, 0
//7, 160704, Note_on_c, 9, 44, 37
//7, 160800, Note_off_c, 9, 44, 0
//7, 160800, Note_on_c, 9, 44, 43
//7, 160896, Note_off_c, 9, 44, 0
//7, 160896, Note_on_c, 9, 44, 43
//7, 160992, Note_off_c, 9, 44, 0
//7, 160992, Note_on_c, 9, 44, 37
//7, 161088, Note_off_c, 9, 44, 0
//7, 161088, Note_on_c, 9, 44, 37
//7, 161184, Note_off_c, 9, 44, 0
//7, 161184, Note_on_c, 9, 44, 43
//7, 161280, Note_off_c, 9, 44, 0
//7, 161280, Note_on_c, 9, 44, 37
//7, 161376, Note_off_c, 9, 44, 0
//7, 161376, Note_on_c, 9, 44, 37
//7, 161472, Note_off_c, 9, 44, 0
//7, 161472, Note_on_c, 9, 44, 37
//7, 161568, Note_off_c, 9, 44, 0
//7, 161568, Note_on_c, 9, 44, 43
//7, 161664, Note_off_c, 9, 44, 0
//7, 161664, Note_on_c, 9, 44, 43
//7, 161760, Note_off_c, 9, 44, 0
//7, 161760, Note_on_c, 9, 44, 37
//7, 161856, Note_off_c, 9, 44, 0
//7, 161856, Note_on_c, 9, 44, 37
//7, 161952, Note_off_c, 9, 44, 0
//7, 161952, Note_on_c, 9, 44, 43
//7, 162048, Note_off_c, 9, 44, 0
//7, 162048, Note_on_c, 9, 44, 37
//7, 162144, Note_off_c, 9, 44, 0
//7, 162144, Note_on_c, 9, 44, 37
//7, 162240, Note_off_c, 9, 44, 0
//7, 162240, Note_on_c, 9, 44, 37
//7, 162336, Note_off_c, 9, 44, 0
//7, 162336, Note_on_c, 9, 44, 43
//7, 162432, Note_off_c, 9, 44, 0
//7, 162432, Note_on_c, 9, 44, 43
//7, 162528, Note_off_c, 9, 44, 0
//7, 162528, Note_on_c, 9, 44, 37
//7, 162624, Note_off_c, 9, 44, 0
//7, 162624, Note_on_c, 9, 44, 37
//7, 162720, Note_off_c, 9, 44, 0
//7, 162720, Note_on_c, 9, 44, 43
//7, 162816, Note_off_c, 9, 44, 0
//7, 162816, Note_on_c, 9, 44, 37
//7, 162912, Note_off_c, 9, 44, 0
//7, 162912, Note_on_c, 9, 44, 37
//7, 163008, Note_off_c, 9, 44, 0
//7, 163008, Note_on_c, 9, 44, 37
//7, 163104, Note_off_c, 9, 44, 0
//7, 163104, Note_on_c, 9, 44, 43
//7, 163200, Note_off_c, 9, 44, 0
//7, 163200, Note_on_c, 9, 44, 43
//7, 163296, Note_off_c, 9, 44, 0
//7, 163296, Note_on_c, 9, 44, 37
//7, 163392, Note_off_c, 9, 44, 0
//7, 163392, Note_on_c, 9, 44, 37
//7, 163488, Note_off_c, 9, 44, 0
//7, 163488, Note_on_c, 9, 44, 43
//7, 163584, Note_off_c, 9, 44, 0
//7, 163584, Note_on_c, 9, 44, 37
//7, 163680, Note_off_c, 9, 44, 0
//7, 163680, Note_on_c, 9, 44, 37
//7, 163776, Note_off_c, 9, 44, 0
//7, 163776, Note_on_c, 9, 44, 37
//7, 163872, Note_off_c, 9, 44, 0
//7, 163872, Note_on_c, 9, 44, 43
//7, 163968, Note_off_c, 9, 44, 0
//7, 163968, Note_on_c, 9, 44, 43
//7, 164064, Note_off_c, 9, 44, 0
//7, 164064, Note_on_c, 9, 44, 37
//7, 164160, Note_off_c, 9, 44, 0
//7, 164160, Note_on_c, 9, 44, 37
//7, 164256, Note_off_c, 9, 44, 0
//7, 164256, Note_on_c, 9, 44, 43
//7, 164352, Note_off_c, 9, 44, 0
//7, 164352, Note_on_c, 9, 44, 37
//7, 164448, Note_off_c, 9, 44, 0
//7, 164448, Note_on_c, 9, 44, 37
//7, 164544, Note_off_c, 9, 44, 0
//7, 164544, Note_on_c, 9, 44, 37
//7, 164640, Note_off_c, 9, 44, 0
//7, 164640, Note_on_c, 9, 44, 43
//7, 164736, Note_off_c, 9, 44, 0
//7, 164736, Note_on_c, 9, 44, 43
//7, 164832, Note_off_c, 9, 44, 0
//7, 164832, Note_on_c, 9, 44, 37
//7, 164928, Note_off_c, 9, 44, 0
//7, 164928, Note_on_c, 9, 44, 37
//7, 165024, Note_off_c, 9, 44, 0
//7, 165024, Note_on_c, 9, 44, 43
//7, 165120, Note_off_c, 9, 44, 0
//7, 165120, Note_on_c, 9, 44, 37
//7, 165216, Note_off_c, 9, 44, 0
//7, 165216, Note_on_c, 9, 44, 37
//7, 165312, Note_off_c, 9, 44, 0
//7, 165312, Note_on_c, 9, 44, 37
//7, 165408, Note_off_c, 9, 44, 0
//7, 165408, Note_on_c, 9, 44, 43
//7, 165504, Note_off_c, 9, 44, 0
//7, 165504, Note_on_c, 9, 44, 43
//7, 165600, Note_off_c, 9, 44, 0
//7, 165600, Note_on_c, 9, 44, 37
//7, 165696, Note_off_c, 9, 44, 0
//7, 165696, Note_on_c, 9, 44, 37
//7, 165792, Note_off_c, 9, 44, 0
//7, 165792, Note_on_c, 9, 44, 43
//7, 165888, Note_off_c, 9, 44, 0
//7, 165888, Note_on_c, 9, 44, 37
//7, 165984, Note_off_c, 9, 44, 0
//7, 165984, Note_on_c, 9, 44, 37
//7, 166080, Note_off_c, 9, 44, 0
//7, 166080, Note_on_c, 9, 44, 37
//7, 166176, Note_off_c, 9, 44, 0
//7, 166176, Note_on_c, 9, 44, 43
//7, 166272, Note_off_c, 9, 44, 0
//7, 166272, Note_on_c, 9, 44, 43
//7, 166368, Note_off_c, 9, 44, 0
//7, 166368, Note_on_c, 9, 44, 37
//7, 166464, Note_off_c, 9, 44, 0
//7, 166464, Note_on_c, 9, 44, 37
//7, 166560, Note_off_c, 9, 44, 0
//7, 166560, Note_on_c, 9, 44, 43
//7, 166656, Note_off_c, 9, 44, 0
//7, 166656, Note_on_c, 9, 44, 37
//7, 166752, Note_off_c, 9, 44, 0
//7, 166752, Note_on_c, 9, 44, 37
//7, 166848, Note_off_c, 9, 44, 0
//7, 166848, Note_on_c, 9, 44, 37
//7, 166944, Note_off_c, 9, 44, 0
//7, 166944, Note_on_c, 9, 44, 43
//7, 167040, Note_off_c, 9, 44, 0
//7, 167040, Note_on_c, 9, 44, 43
//7, 167136, Note_off_c, 9, 44, 0
//7, 167136, Note_on_c, 9, 44, 37
//7, 167232, Note_off_c, 9, 44, 0
//7, 167232, Note_on_c, 9, 44, 37
//7, 167328, Note_off_c, 9, 44, 0
//7, 167328, Note_on_c, 9, 44, 43
//7, 167424, Note_off_c, 9, 44, 0
//7, 167424, Note_on_c, 9, 44, 37
//7, 167520, Note_off_c, 9, 44, 0
//7, 167520, Note_on_c, 9, 44, 37
//7, 167616, Note_off_c, 9, 44, 0
//7, 167616, Note_on_c, 9, 44, 37
//7, 167712, Note_off_c, 9, 44, 0
//7, 167712, Note_on_c, 9, 44, 43
//7, 167808, Note_off_c, 9, 44, 0
//7, 167808, Note_on_c, 9, 44, 43
//7, 167904, Note_off_c, 9, 44, 0
//7, 167904, Note_on_c, 9, 44, 37
//7, 168000, Note_off_c, 9, 44, 0
//7, 168000, Note_on_c, 9, 44, 37
//7, 168096, Note_off_c, 9, 44, 0
//7, 168096, Note_on_c, 9, 44, 43
//7, 168192, Note_off_c, 9, 44, 0
//7, 168192, Note_on_c, 9, 44, 37
//7, 168288, Note_off_c, 9, 44, 0
//7, 168288, Note_on_c, 9, 44, 37
//7, 168384, Note_off_c, 9, 44, 0
//7, 168384, Note_on_c, 9, 44, 37
//7, 168480, Note_off_c, 9, 44, 0
//7, 168480, Note_on_c, 9, 44, 43
//7, 168576, Note_off_c, 9, 44, 0
//7, 168576, Note_on_c, 9, 44, 43
//7, 168672, Note_off_c, 9, 44, 0
//7, 168672, Note_on_c, 9, 44, 37
//7, 168768, Note_off_c, 9, 44, 0
//7, 168768, Note_on_c, 9, 44, 37
//7, 168864, Note_off_c, 9, 44, 0
//7, 168864, Note_on_c, 9, 44, 43
//7, 168960, Note_off_c, 9, 44, 0
//7, 168960, Note_on_c, 9, 44, 37
//7, 169056, Note_off_c, 9, 44, 0
//7, 169056, Note_on_c, 9, 44, 37
//7, 169152, Note_off_c, 9, 44, 0
//7, 169152, Note_on_c, 9, 44, 37
//7, 169248, Note_off_c, 9, 44, 0
//7, 169248, Note_on_c, 9, 44, 43
//7, 169344, Note_off_c, 9, 44, 0
//7, 169344, Note_on_c, 9, 44, 43
//7, 169440, Note_off_c, 9, 44, 0
//7, 169440, Note_on_c, 9, 44, 37
//7, 169536, Note_off_c, 9, 44, 0
//7, 169536, Note_on_c, 9, 44, 37
//7, 169632, Note_off_c, 9, 44, 0
//7, 169632, Note_on_c, 9, 44, 43
//7, 169728, Note_off_c, 9, 44, 0
//7, 169728, Note_on_c, 9, 44, 37
//7, 169824, Note_off_c, 9, 44, 0
//7, 169824, Note_on_c, 9, 44, 37
//7, 169920, Note_off_c, 9, 44, 0
//7, 169920, Note_on_c, 9, 44, 37
//7, 170016, Note_off_c, 9, 44, 0
//7, 170016, Note_on_c, 9, 44, 43
//7, 170112, Note_off_c, 9, 44, 0
//7, 170112, Note_on_c, 9, 44, 43
//7, 170208, Note_off_c, 9, 44, 0
//7, 170208, Note_on_c, 9, 44, 37
//7, 170304, Note_off_c, 9, 44, 0
//7, 170304, Note_on_c, 9, 44, 37
//7, 170400, Note_off_c, 9, 44, 0
//7, 170400, Note_on_c, 9, 44, 43
//7, 170496, Note_off_c, 9, 44, 0
//7, 170496, Note_on_c, 9, 44, 37
//7, 170592, Note_off_c, 9, 44, 0
//7, 170592, Note_on_c, 9, 44, 37
//7, 170688, Note_off_c, 9, 44, 0
//7, 170688, Note_on_c, 9, 44, 37
//7, 170784, Note_off_c, 9, 44, 0
//7, 170784, Note_on_c, 9, 44, 43
//7, 170880, Note_off_c, 9, 44, 0
//7, 170880, Note_on_c, 9, 44, 43
//7, 170976, Note_off_c, 9, 44, 0
//7, 170976, Note_on_c, 9, 44, 37
//7, 171072, Note_off_c, 9, 44, 0
//7, 171072, Note_on_c, 9, 44, 37
//7, 171168, Note_off_c, 9, 44, 0
//7, 171168, Note_on_c, 9, 44, 43
//7, 171264, Note_off_c, 9, 44, 0
//7, 171264, Note_on_c, 9, 44, 37
//7, 171360, Note_off_c, 9, 44, 0
//7, 171360, Note_on_c, 9, 44, 37
//7, 171456, Note_off_c, 9, 44, 0
//7, 171456, Note_on_c, 9, 44, 37
//7, 171552, Note_off_c, 9, 44, 0
//7, 171552, Note_on_c, 9, 44, 43
//7, 171648, Note_off_c, 9, 44, 0
//7, 171648, Note_on_c, 9, 44, 43
//7, 171744, Note_off_c, 9, 44, 0
//7, 171744, Note_on_c, 9, 44, 37
//7, 171840, Note_off_c, 9, 44, 0
//7, 171840, Note_on_c, 9, 44, 37
//7, 171936, Note_off_c, 9, 44, 0
//7, 171936, Note_on_c, 9, 44, 43
//7, 172032, Note_off_c, 9, 44, 0
//7, 172032, Note_on_c, 9, 44, 37
//7, 172128, Note_off_c, 9, 44, 0
//7, 172128, Note_on_c, 9, 44, 37
//7, 172224, Note_off_c, 9, 44, 0
//7, 172224, Note_on_c, 9, 44, 37
//7, 172320, Note_off_c, 9, 44, 0
//7, 172320, Note_on_c, 9, 44, 43
//7, 172416, Note_off_c, 9, 44, 0
//7, 176064, Note_on_c, 9, 46, 31
//7, 176256, Note_off_c, 9, 46, 0
//7, 176256, Note_on_c, 9, 46, 31
//7, 176448, Note_off_c, 9, 46, 0
//7, 176448, Note_on_c, 9, 46, 31
//7, 176640, Note_off_c, 9, 46, 0
//7, 176640, Note_on_c, 9, 46, 31
//7, 176832, Note_off_c, 9, 46, 0
//7, 176832, Note_on_c, 9, 46, 31
//7, 177024, Note_off_c, 9, 46, 0
//7, 177024, Note_on_c, 9, 46, 31
//7, 177216, Note_off_c, 9, 46, 0
//7, 177216, Note_on_c, 9, 46, 31
//7, 177408, Note_off_c, 9, 46, 0
//7, 177600, Note_on_c, 9, 46, 31
//7, 177792, Note_off_c, 9, 46, 0
//7, 177792, Note_on_c, 9, 46, 31
//7, 177984, Note_off_c, 9, 46, 0
//7, 177984, Note_on_c, 9, 46, 31
//7, 178176, Note_off_c, 9, 46, 0
//7, 178176, Note_on_c, 9, 46, 31
//7, 178368, Note_off_c, 9, 46, 0
//7, 178368, Note_on_c, 9, 46, 31
//7, 178560, Note_off_c, 9, 46, 0
//7, 179328, Note_on_c, 9, 56, 37
//7, 179328, Note_on_c, 9, 44, 37
//7, 179520, Note_off_c, 9, 44, 0
//7, 179520, Note_off_c, 9, 56, 0
//7, 179712, Note_on_c, 9, 56, 37
//7, 179712, Note_on_c, 9, 44, 37
//7, 179904, Note_off_c, 9, 44, 0
//7, 179904, Note_off_c, 9, 56, 0
//7, 180096, Note_on_c, 9, 56, 37
//7, 180096, Note_on_c, 9, 44, 37
//7, 180288, Note_off_c, 9, 44, 0
//7, 180288, Note_off_c, 9, 56, 0
//7, 180480, Note_on_c, 9, 56, 37
//7, 180480, Note_on_c, 9, 44, 37
//7, 180672, Note_off_c, 9, 44, 0
//7, 180672, Note_off_c, 9, 56, 0
//7, 180864, Note_on_c, 9, 56, 37
//7, 180864, Note_on_c, 9, 44, 37
//7, 181056, Note_off_c, 9, 44, 0
//7, 181056, Note_off_c, 9, 56, 0
//7, 181248, Note_on_c, 9, 56, 37
//7, 181248, Note_on_c, 9, 44, 37
//7, 181440, Note_off_c, 9, 44, 0
//7, 181440, Note_off_c, 9, 56, 0
//7, 181632, Note_on_c, 9, 56, 37
//7, 181632, Note_on_c, 9, 44, 37
//7, 181824, Note_off_c, 9, 44, 0
//7, 181824, Note_off_c, 9, 56, 0
//7, 182208, Note_on_c, 9, 46, 31
//7, 182400, Note_off_c, 9, 46, 0
//7, 182400, Note_on_c, 9, 46, 31
//7, 182592, Note_off_c, 9, 46, 0
//7, 182592, Note_on_c, 9, 46, 31
//7, 182784, Note_off_c, 9, 46, 0
//7, 182784, Note_on_c, 9, 46, 31
//7, 182976, Note_off_c, 9, 46, 0
//7, 182976, Note_on_c, 9, 46, 31
//7, 183168, Note_off_c, 9, 46, 0
//7, 183168, Note_on_c, 9, 46, 31
//7, 183360, Note_off_c, 9, 46, 0
//7, 183360, Note_on_c, 9, 46, 31
//7, 183552, Note_off_c, 9, 46, 0
//7, 183744, Note_on_c, 9, 46, 31
//7, 183936, Note_off_c, 9, 46, 0
//7, 183936, Note_on_c, 9, 46, 31
//7, 184128, Note_off_c, 9, 46, 0
//7, 184128, Note_on_c, 9, 46, 31
//7, 184320, Note_off_c, 9, 46, 0
//7, 184320, Note_on_c, 9, 46, 31
//7, 184512, Note_off_c, 9, 46, 0
//7, 184512, Note_on_c, 9, 46, 31
//7, 184704, Note_off_c, 9, 46, 0
//7, 185472, Note_on_c, 9, 56, 37
//7, 185472, Note_on_c, 9, 44, 37
//7, 185664, Note_off_c, 9, 44, 0
//7, 185664, Note_off_c, 9, 56, 0
//7, 185856, Note_on_c, 9, 56, 37
//7, 185856, Note_on_c, 9, 44, 37
//7, 186048, Note_off_c, 9, 44, 0
//7, 186048, Note_off_c, 9, 56, 0
//7, 186240, Note_on_c, 9, 56, 37
//7, 186240, Note_on_c, 9, 44, 37
//7, 186432, Note_off_c, 9, 44, 0
//7, 186432, Note_off_c, 9, 56, 0
//7, 186624, Note_on_c, 9, 56, 37
//7, 186624, Note_on_c, 9, 44, 37
//7, 186816, Note_off_c, 9, 44, 0
//7, 186816, Note_off_c, 9, 56, 0
//7, 187008, Note_on_c, 9, 56, 37
//7, 187008, Note_on_c, 9, 44, 37
//7, 187200, Note_off_c, 9, 44, 0
//7, 187200, Note_off_c, 9, 56, 0
//7, 187392, Note_on_c, 9, 56, 37
//7, 187392, Note_on_c, 9, 44, 37
//7, 187584, Note_off_c, 9, 44, 0
//7, 187584, Note_off_c, 9, 56, 0
//7, 187776, Note_on_c, 9, 56, 37
//7, 187776, Note_on_c, 9, 44, 37
//7, 187968, Note_off_c, 9, 44, 0
//7, 187968, Note_off_c, 9, 56, 0
//7, 188352, Note_on_c, 9, 46, 31
//7, 188544, Note_off_c, 9, 46, 0
//7, 188544, Note_on_c, 9, 46, 31
//7, 188736, Note_off_c, 9, 46, 0
//7, 188736, Note_on_c, 9, 46, 31
//7, 188928, Note_off_c, 9, 46, 0
//7, 188928, Note_on_c, 9, 46, 31
//7, 189120, Note_off_c, 9, 46, 0
//7, 189120, Note_on_c, 9, 46, 31
//7, 189312, Note_off_c, 9, 46, 0
//7, 189312, Note_on_c, 9, 46, 31
//7, 189504, Note_off_c, 9, 46, 0
//7, 189504, Note_on_c, 9, 46, 31
//7, 189696, Note_off_c, 9, 46, 0
//7, 189888, Note_on_c, 9, 46, 31
//7, 190080, Note_off_c, 9, 46, 0
//7, 190080, Note_on_c, 9, 46, 31
//7, 190272, Note_off_c, 9, 46, 0
//7, 190272, Note_on_c, 9, 46, 31
//7, 190464, Note_off_c, 9, 46, 0
//7, 190464, Note_on_c, 9, 46, 31
//7, 190656, Note_off_c, 9, 46, 0
//7, 190656, Note_on_c, 9, 46, 31
//7, 190848, Note_off_c, 9, 46, 0
//7, 191616, Note_on_c, 9, 56, 37
//7, 191616, Note_on_c, 9, 44, 37
//7, 191808, Note_off_c, 9, 44, 0
//7, 191808, Note_off_c, 9, 56, 0
//7, 192000, Note_on_c, 9, 56, 37
//7, 192000, Note_on_c, 9, 44, 37
//7, 192192, Note_off_c, 9, 44, 0
//7, 192192, Note_off_c, 9, 56, 0
//7, 192384, Note_on_c, 9, 56, 37
//7, 192384, Note_on_c, 9, 44, 37
//7, 192576, Note_off_c, 9, 44, 0
//7, 192576, Note_off_c, 9, 56, 0
//7, 192768, Note_on_c, 9, 56, 37
//7, 192768, Note_on_c, 9, 44, 37
//7, 192960, Note_off_c, 9, 44, 0
//7, 192960, Note_off_c, 9, 56, 0
//7, 193152, Note_on_c, 9, 56, 37
//7, 193152, Note_on_c, 9, 44, 37
//7, 193344, Note_off_c, 9, 44, 0
//7, 193344, Note_off_c, 9, 56, 0
//7, 193536, Note_on_c, 9, 56, 37
//7, 193536, Note_on_c, 9, 44, 37
//7, 193728, Note_off_c, 9, 44, 0
//7, 193728, Note_off_c, 9, 56, 0
//7, 193920, Note_on_c, 9, 56, 37
//7, 193920, Note_on_c, 9, 44, 37
//7, 194112, Note_off_c, 9, 44, 0
//7, 194112, Note_off_c, 9, 56, 0
//7, 194496, Note_on_c, 9, 46, 31
//7, 194688, Note_off_c, 9, 46, 0
//7, 194688, Note_on_c, 9, 46, 31
//7, 194880, Note_off_c, 9, 46, 0
//7, 194880, Note_on_c, 9, 46, 31
//7, 195072, Note_off_c, 9, 46, 0
//7, 195072, Note_on_c, 9, 46, 31
//7, 195264, Note_off_c, 9, 46, 0
//7, 195264, Note_on_c, 9, 46, 31
//7, 195456, Note_off_c, 9, 46, 0
//7, 195456, Note_on_c, 9, 46, 31
//7, 195648, Note_off_c, 9, 46, 0
//7, 195648, Note_on_c, 9, 46, 31
//7, 195840, Note_off_c, 9, 46, 0
//7, 196032, Note_on_c, 9, 46, 31
//7, 196224, Note_off_c, 9, 46, 0
//7, 196224, Note_on_c, 9, 46, 31
//7, 196416, Note_off_c, 9, 46, 0
//7, 196416, Note_on_c, 9, 46, 31
//7, 196608, Note_off_c, 9, 46, 0
//7, 196608, Note_on_c, 9, 46, 31
//7, 196800, Note_off_c, 9, 46, 0
//7, 196800, Note_on_c, 9, 46, 31
//7, 196992, Note_off_c, 9, 46, 0
//7, 196992, End_track

//8, 0, Title_t, "Cymbal"
//8, 0, Program_c, 9, 118
//8, 11520, Note_on_c, 9, 57, 37
//8, 11616, Note_off_c, 9, 57, 0
//8, 16128, Note_on_c, 9, 57, 37
//8, 16512, Note_off_c, 9, 57, 0
//8, 16896, Note_on_c, 9, 57, 37
//8, 17280, Note_off_c, 9, 57, 0
//8, 17664, Note_on_c, 9, 57, 37
//8, 17856, Note_off_c, 9, 57, 0
//8, 20736, Note_on_c, 9, 57, 37
//8, 20928, Note_off_c, 9, 57, 0
//8, 26880, Note_on_c, 9, 57, 31
//8, 27072, Note_off_c, 9, 57, 0
//8, 33024, Note_on_c, 9, 57, 31
//8, 33216, Note_off_c, 9, 57, 0
//8, 39168, Note_on_c, 9, 57, 37
//8, 39360, Note_off_c, 9, 57, 0
//8, 40704, Note_on_c, 9, 49, 37
//8, 40896, Note_off_c, 9, 49, 0
//8, 42240, Note_on_c, 9, 57, 37
//8, 42432, Note_off_c, 9, 57, 0
//8, 46848, Note_on_c, 9, 57, 37
//8, 47040, Note_off_c, 9, 57, 0
//8, 47136, Note_on_c, 9, 49, 37
//8, 47328, Note_off_c, 9, 49, 0
//8, 47424, Note_on_c, 9, 57, 37
//8, 47616, Note_off_c, 9, 57, 0
//8, 48384, Note_on_c, 9, 57, 31
//8, 48576, Note_off_c, 9, 57, 0
//8, 54528, Note_on_c, 9, 57, 31
//8, 54720, Note_off_c, 9, 57, 0
//8, 60672, Note_on_c, 9, 57, 37
//8, 60864, Note_off_c, 9, 57, 0
//8, 62208, Note_on_c, 9, 49, 37
//8, 62400, Note_off_c, 9, 49, 0
//8, 63744, Note_on_c, 9, 57, 37
//8, 63936, Note_off_c, 9, 57, 0
//8, 68352, Note_on_c, 9, 57, 37
//8, 68544, Note_off_c, 9, 57, 0
//8, 68640, Note_on_c, 9, 49, 37
//8, 68832, Note_off_c, 9, 49, 0
//8, 68928, Note_on_c, 9, 57, 37
//8, 69120, Note_off_c, 9, 57, 0
//8, 69888, Note_on_c, 9, 57, 37
//8, 70080, Note_off_c, 9, 57, 0
//8, 82176, Note_on_c, 9, 57, 31
//8, 82368, Note_off_c, 9, 57, 0
//8, 88320, Note_on_c, 9, 57, 31
//8, 88512, Note_off_c, 9, 57, 0
//8, 94464, Note_on_c, 9, 57, 37
//8, 94656, Note_off_c, 9, 57, 0
//8, 96000, Note_on_c, 9, 49, 37
//8, 96192, Note_off_c, 9, 49, 0
//8, 97536, Note_on_c, 9, 57, 37
//8, 97728, Note_off_c, 9, 57, 0
//8, 102144, Note_on_c, 9, 57, 37
//8, 102336, Note_off_c, 9, 57, 0
//8, 102432, Note_on_c, 9, 49, 37
//8, 102624, Note_off_c, 9, 49, 0
//8, 102720, Note_on_c, 9, 57, 37
//8, 102912, Note_off_c, 9, 57, 0
//8, 103680, Note_on_c, 9, 57, 37
//8, 104064, Note_off_c, 9, 57, 0
//8, 120576, Note_on_c, 9, 57, 37
//8, 120768, Note_off_c, 9, 57, 0
//8, 126720, Note_on_c, 9, 57, 37
//8, 126912, Note_off_c, 9, 57, 0
//8, 128064, Note_on_c, 9, 57, 31
//8, 128256, Note_off_c, 9, 57, 0
//8, 129792, Note_on_c, 9, 57, 37
//8, 129984, Note_off_c, 9, 57, 0
//8, 131136, Note_on_c, 9, 57, 31
//8, 131328, Note_off_c, 9, 57, 0
//8, 134208, Note_on_c, 9, 57, 31
//8, 134400, Note_off_c, 9, 57, 0
//8, 135408, Note_on_c, 9, 57, 31
//8, 135600, Note_off_c, 9, 57, 0
//8, 135600, Note_on_c, 9, 49, 31
//8, 135792, Note_off_c, 9, 49, 0
//8, 135792, Note_on_c, 9, 57, 37
//8, 135984, Note_off_c, 9, 57, 0
//8, 136320, Note_on_c, 9, 57, 37
//8, 136512, Note_off_c, 9, 57, 0
//8, 137088, Note_on_c, 9, 57, 37
//8, 137280, Note_off_c, 9, 57, 0
//8, 137856, Note_on_c, 9, 57, 37
//8, 138048, Note_off_c, 9, 57, 0
//8, 138432, Note_on_c, 9, 57, 37
//8, 138624, Note_off_c, 9, 57, 0
//8, 139008, Note_on_c, 9, 57, 37
//8, 139200, Note_off_c, 9, 57, 0
//8, 141888, Note_on_c, 9, 57, 37
//8, 142080, Note_off_c, 9, 57, 0
//8, 144960, Note_on_c, 9, 57, 37
//8, 145152, Note_off_c, 9, 57, 0
//8, 151296, Note_on_c, 9, 49, 37
//8, 151296, Note_on_c, 9, 57, 37
//8, 151488, Note_off_c, 9, 57, 0
//8, 151488, Note_off_c, 9, 49, 0
//8, 154368, Note_on_c, 9, 49, 37
//8, 154368, Note_on_c, 9, 57, 37
//8, 154464, Note_off_c, 9, 57, 0
//8, 154464, Note_off_c, 9, 49, 0
//8, 172800, Note_on_c, 9, 57, 37
//8, 173184, Note_off_c, 9, 57, 0
//8, 173184, Note_on_c, 9, 49, 37
//8, 173568, Note_off_c, 9, 49, 0
//8, 173568, Note_on_c, 9, 57, 37
//8, 173952, Note_off_c, 9, 57, 0
//8, 173952, Note_on_c, 9, 49, 37
//8, 174336, Note_off_c, 9, 49, 0
//8, 174336, Note_on_c, 9, 57, 37
//8, 174720, Note_off_c, 9, 57, 0
//8, 174720, Note_on_c, 9, 49, 37
//8, 175104, Note_off_c, 9, 49, 0
//8, 175104, Note_on_c, 9, 57, 37
//8, 175488, Note_off_c, 9, 57, 0
//8, 175872, Note_on_c, 9, 57, 37
//8, 176064, Note_off_c, 9, 57, 0
//8, 177408, Note_on_c, 9, 49, 37
//8, 177600, Note_off_c, 9, 49, 0
//8, 178944, Note_on_c, 9, 57, 37
//8, 179136, Note_off_c, 9, 57, 0
//8, 182016, Note_on_c, 9, 57, 37
//8, 182208, Note_off_c, 9, 57, 0
//8, 183552, Note_on_c, 9, 49, 37
//8, 183744, Note_off_c, 9, 49, 0
//8, 185088, Note_on_c, 9, 57, 37
//8, 185280, Note_off_c, 9, 57, 0
//8, 188160, Note_on_c, 9, 57, 37
//8, 188352, Note_off_c, 9, 57, 0
//8, 189696, Note_on_c, 9, 49, 37
//8, 189888, Note_off_c, 9, 49, 0
//8, 191232, Note_on_c, 9, 57, 37
//8, 191424, Note_off_c, 9, 57, 0
//8, 194304, Note_on_c, 9, 57, 37
//8, 194496, Note_off_c, 9, 57, 0
//8, 195840, Note_on_c, 9, 49, 37
//8, 196032, Note_off_c, 9, 49, 0
//8, 197376, Note_on_c, 9, 57, 31
//8, 197568, Note_off_c, 9, 57, 0
//8, 197664, Note_on_c, 9, 49, 31
//8, 197760, Note_off_c, 9, 49, 0
//8, 197952, Note_on_c, 9, 57, 31
//8, 198144, Note_off_c, 9, 57, 0
//8, 198240, Note_on_c, 9, 49, 31
//8, 198432, Note_off_c, 9, 49, 0
//8, 198528, Note_on_c, 9, 57, 31
//8, 198720, Note_off_c, 9, 57, 0
//8, 198720, Note_on_c, 9, 49, 31
//8, 198912, Note_off_c, 9, 49, 0
//8, 198912, Note_on_c, 9, 57, 31
//8, 199104, Note_off_c, 9, 57, 0
//8, 199200, Note_on_c, 9, 49, 31
//8, 199296, Note_off_c, 9, 49, 0
//8, 199488, Note_on_c, 9, 57, 31
//8, 199680, Note_off_c, 9, 57, 0
//8, 200064, Note_on_c, 9, 49, 31
//8, 200064, Note_on_c, 9, 57, 31
//8, 200448, Note_off_c, 9, 57, 0
//8, 200448, Note_off_c, 9, 49, 0
//8, 200448, End_track
//9, 0, Start_track
//9, 0, Title_t, "Snare Drum"
//9, 0, Program_c, 9, 118
//9, 17664, Note_on_c, 9, 38, 37
//9, 17856, Note_off_c, 9, 38, 0
//9, 17856, Note_on_c, 9, 38, 31
//9, 18048, Note_off_c, 9, 38, 0
//9, 18048, Note_on_c, 9, 38, 31
//9, 18240, Note_off_c, 9, 38, 0
//9, 18240, Note_on_c, 9, 38, 31
//9, 18432, Note_off_c, 9, 38, 0
//9, 18432, Note_on_c, 9, 38, 31
//9, 18624, Note_off_c, 9, 38, 0
//9, 18624, Note_on_c, 9, 38, 31
//9, 18816, Note_off_c, 9, 38, 0
//9, 18816, Note_on_c, 9, 38, 31
//9, 19008, Note_off_c, 9, 38, 0
//9, 19008, Note_on_c, 9, 38, 31
//9, 19200, Note_off_c, 9, 38, 0
//9, 19200, Note_on_c, 9, 38, 31
//9, 19392, Note_off_c, 9, 38, 0
//9, 19392, Note_on_c, 9, 38, 31
//9, 19584, Note_off_c, 9, 38, 0
//9, 19584, Note_on_c, 9, 38, 31
//9, 19776, Note_off_c, 9, 38, 0
//9, 19776, Note_on_c, 9, 38, 31
//9, 19968, Note_off_c, 9, 38, 0
//9, 19968, Note_on_c, 9, 38, 31
//9, 20016, Note_off_c, 9, 38, 0
//9, 20016, Note_on_c, 9, 38, 37
//9, 20208, Note_off_c, 9, 38, 0
//9, 20400, Note_on_c, 9, 38, 31
//9, 20448, Note_off_c, 9, 38, 0
//9, 20448, Note_on_c, 9, 38, 37
//9, 20832, Note_off_c, 9, 38, 0
//9, 21120, Note_on_c, 9, 38, 37
//9, 21312, Note_off_c, 9, 38, 0
//9, 21888, Note_on_c, 9, 38, 37
//9, 22080, Note_off_c, 9, 38, 0
//9, 22656, Note_on_c, 9, 38, 37
//9, 22848, Note_off_c, 9, 38, 0
//9, 23424, Note_on_c, 9, 38, 37
//9, 23616, Note_off_c, 9, 38, 0
//9, 24192, Note_on_c, 9, 38, 37
//9, 24384, Note_off_c, 9, 38, 0
//9, 24960, Note_on_c, 9, 38, 37
//9, 25152, Note_off_c, 9, 38, 0
//9, 25728, Note_on_c, 9, 38, 37
//9, 25920, Note_off_c, 9, 38, 0
//9, 26496, Note_on_c, 9, 38, 37
//9, 26688, Note_off_c, 9, 38, 0
//9, 26688, Note_on_c, 9, 38, 37
//9, 26784, Note_off_c, 9, 38, 0
//9, 26784, Note_on_c, 9, 38, 37
//9, 26880, Note_off_c, 9, 38, 0
//9, 27264, Note_on_c, 9, 38, 37
//9, 27456, Note_off_c, 9, 38, 0
//9, 28032, Note_on_c, 9, 38, 37
//9, 28224, Note_off_c, 9, 38, 0
//9, 28800, Note_on_c, 9, 38, 37
//9, 28992, Note_off_c, 9, 38, 0
//9, 29568, Note_on_c, 9, 38, 37
//9, 29760, Note_off_c, 9, 38, 0
//9, 30336, Note_on_c, 9, 38, 37
//9, 30528, Note_off_c, 9, 38, 0
//9, 31104, Note_on_c, 9, 38, 37
//9, 31296, Note_off_c, 9, 38, 0
//9, 31872, Note_on_c, 9, 38, 37
//9, 32064, Note_off_c, 9, 38, 0
//9, 32640, Note_on_c, 9, 38, 37
//9, 32832, Note_off_c, 9, 38, 0
//9, 32832, Note_on_c, 9, 38, 37
//9, 32928, Note_off_c, 9, 38, 0
//9, 32928, Note_on_c, 9, 38, 37
//9, 33024, Note_off_c, 9, 38, 0
//9, 33408, Note_on_c, 9, 38, 37
//9, 33600, Note_off_c, 9, 38, 0
//9, 34176, Note_on_c, 9, 38, 37
//9, 34368, Note_off_c, 9, 38, 0
//9, 34944, Note_on_c, 9, 38, 37
//9, 35136, Note_off_c, 9, 38, 0
//9, 35712, Note_on_c, 9, 38, 37
//9, 35904, Note_off_c, 9, 38, 0
//9, 36480, Note_on_c, 9, 38, 37
//9, 36672, Note_off_c, 9, 38, 0
//9, 37248, Note_on_c, 9, 38, 37
//9, 37440, Note_off_c, 9, 38, 0
//9, 38016, Note_on_c, 9, 38, 37
//9, 38208, Note_off_c, 9, 38, 0
//9, 38784, Note_on_c, 9, 38, 31
//9, 38832, Note_off_c, 9, 38, 0
//9, 38832, Note_on_c, 9, 38, 37
//9, 39024, Note_off_c, 9, 38, 0
//9, 39552, Note_on_c, 9, 38, 37
//9, 39744, Note_off_c, 9, 38, 0
//9, 40320, Note_on_c, 9, 38, 37
//9, 40512, Note_off_c, 9, 38, 0
//9, 41088, Note_on_c, 9, 38, 37
//9, 41280, Note_off_c, 9, 38, 0
//9, 41856, Note_on_c, 9, 38, 31
//9, 41904, Note_off_c, 9, 38, 0
//9, 41904, Note_on_c, 9, 38, 37
//9, 42096, Note_off_c, 9, 38, 0
//9, 42096, Note_on_c, 9, 38, 31
//9, 42144, Note_off_c, 9, 38, 0
//9, 42144, Note_on_c, 9, 38, 37
//9, 42336, Note_off_c, 9, 38, 0
//9, 42624, Note_on_c, 9, 38, 37
//9, 42816, Note_off_c, 9, 38, 0
//9, 43392, Note_on_c, 9, 38, 37
//9, 43584, Note_off_c, 9, 38, 0
//9, 44160, Note_on_c, 9, 38, 37
//9, 44352, Note_off_c, 9, 38, 0
//9, 44928, Note_on_c, 9, 38, 37
//9, 45120, Note_off_c, 9, 38, 0
//9, 45696, Note_on_c, 9, 38, 37
//9, 45888, Note_off_c, 9, 38, 0
//9, 46464, Note_on_c, 9, 38, 37
//9, 46656, Note_off_c, 9, 38, 0
//9, 48000, Note_on_c, 9, 38, 31
//9, 48048, Note_off_c, 9, 38, 0
//9, 48048, Note_on_c, 9, 38, 37
//9, 48240, Note_off_c, 9, 38, 0
//9, 48768, Note_on_c, 9, 38, 37
//9, 48960, Note_off_c, 9, 38, 0
//9, 49536, Note_on_c, 9, 38, 37
//9, 49728, Note_off_c, 9, 38, 0
//9, 50304, Note_on_c, 9, 38, 37
//9, 50496, Note_off_c, 9, 38, 0
//9, 51072, Note_on_c, 9, 38, 37
//9, 51264, Note_off_c, 9, 38, 0
//9, 51840, Note_on_c, 9, 38, 37
//9, 52032, Note_off_c, 9, 38, 0
//9, 52608, Note_on_c, 9, 38, 37
//9, 52800, Note_off_c, 9, 38, 0
//9, 53376, Note_on_c, 9, 38, 37
//9, 53568, Note_off_c, 9, 38, 0
//9, 54144, Note_on_c, 9, 38, 37
//9, 54336, Note_off_c, 9, 38, 0
//9, 54336, Note_on_c, 9, 38, 37
//9, 54432, Note_off_c, 9, 38, 0
//9, 54432, Note_on_c, 9, 38, 37
//9, 54528, Note_off_c, 9, 38, 0
//9, 54912, Note_on_c, 9, 38, 37
//9, 55104, Note_off_c, 9, 38, 0
//9, 55680, Note_on_c, 9, 38, 37
//9, 55872, Note_off_c, 9, 38, 0
//9, 56448, Note_on_c, 9, 38, 37
//9, 56640, Note_off_c, 9, 38, 0
//9, 57216, Note_on_c, 9, 38, 37
//9, 57408, Note_off_c, 9, 38, 0
//9, 57984, Note_on_c, 9, 38, 37
//9, 58176, Note_off_c, 9, 38, 0
//9, 58752, Note_on_c, 9, 38, 37
//9, 58944, Note_off_c, 9, 38, 0
//9, 59520, Note_on_c, 9, 38, 37
//9, 59712, Note_off_c, 9, 38, 0
//9, 60288, Note_on_c, 9, 38, 31
//9, 60336, Note_off_c, 9, 38, 0
//9, 60336, Note_on_c, 9, 38, 37
//9, 60528, Note_off_c, 9, 38, 0
//9, 61056, Note_on_c, 9, 38, 37
//9, 61248, Note_off_c, 9, 38, 0
//9, 61824, Note_on_c, 9, 38, 37
//9, 62016, Note_off_c, 9, 38, 0
//9, 62592, Note_on_c, 9, 38, 37
//9, 62784, Note_off_c, 9, 38, 0
//9, 63360, Note_on_c, 9, 38, 31
//9, 63408, Note_off_c, 9, 38, 0
//9, 63408, Note_on_c, 9, 38, 37
//9, 63600, Note_off_c, 9, 38, 0
//9, 63600, Note_on_c, 9, 38, 31
//9, 63648, Note_off_c, 9, 38, 0
//9, 63648, Note_on_c, 9, 38, 37
//9, 63840, Note_off_c, 9, 38, 0
//9, 64128, Note_on_c, 9, 38, 37
//9, 64320, Note_off_c, 9, 38, 0
//9, 64896, Note_on_c, 9, 38, 37
//9, 65088, Note_off_c, 9, 38, 0
//9, 65664, Note_on_c, 9, 38, 37
//9, 65856, Note_off_c, 9, 38, 0
//9, 66432, Note_on_c, 9, 38, 37
//9, 66624, Note_off_c, 9, 38, 0
//9, 67200, Note_on_c, 9, 38, 37
//9, 67392, Note_off_c, 9, 38, 0
//9, 67968, Note_on_c, 9, 38, 37
//9, 68160, Note_off_c, 9, 38, 0
//9, 69504, Note_on_c, 9, 38, 31
//9, 69552, Note_off_c, 9, 38, 0
//9, 69552, Note_on_c, 9, 38, 37
//9, 69744, Note_off_c, 9, 38, 0
//9, 70272, Note_on_c, 9, 38, 37
//9, 70464, Note_off_c, 9, 38, 0
//9, 71040, Note_on_c, 9, 38, 37
//9, 71232, Note_off_c, 9, 38, 0
//9, 71808, Note_on_c, 9, 38, 37
//9, 72000, Note_off_c, 9, 38, 0
//9, 72576, Note_on_c, 9, 38, 37
//9, 72768, Note_off_c, 9, 38, 0
//9, 73344, Note_on_c, 9, 38, 37
//9, 73536, Note_off_c, 9, 38, 0
//9, 74112, Note_on_c, 9, 38, 37
//9, 74304, Note_off_c, 9, 38, 0
//9, 74880, Note_on_c, 9, 38, 37
//9, 75072, Note_off_c, 9, 38, 0
//9, 75648, Note_on_c, 9, 38, 37
//9, 75840, Note_off_c, 9, 38, 0
//9, 76416, Note_on_c, 9, 38, 37
//9, 76608, Note_off_c, 9, 38, 0
//9, 77184, Note_on_c, 9, 38, 37
//9, 77376, Note_off_c, 9, 38, 0
//9, 77952, Note_on_c, 9, 38, 37
//9, 78144, Note_off_c, 9, 38, 0
//9, 78720, Note_on_c, 9, 38, 37
//9, 78912, Note_off_c, 9, 38, 0
//9, 79488, Note_on_c, 9, 38, 37
//9, 79680, Note_off_c, 9, 38, 0
//9, 80256, Note_on_c, 9, 38, 37
//9, 80448, Note_off_c, 9, 38, 0
//9, 81024, Note_on_c, 9, 38, 37
//9, 81216, Note_off_c, 9, 38, 0
//9, 81792, Note_on_c, 9, 38, 37
//9, 81984, Note_off_c, 9, 38, 0
//9, 81984, Note_on_c, 9, 38, 37
//9, 82080, Note_off_c, 9, 38, 0
//9, 82080, Note_on_c, 9, 38, 37
//9, 82176, Note_off_c, 9, 38, 0
//9, 82560, Note_on_c, 9, 38, 37
//9, 82752, Note_off_c, 9, 38, 0
//9, 83328, Note_on_c, 9, 38, 37
//9, 83520, Note_off_c, 9, 38, 0
//9, 84096, Note_on_c, 9, 38, 37
//9, 84288, Note_off_c, 9, 38, 0
//9, 84864, Note_on_c, 9, 38, 37
//9, 85056, Note_off_c, 9, 38, 0
//9, 85632, Note_on_c, 9, 38, 37
//9, 85824, Note_off_c, 9, 38, 0
//9, 86400, Note_on_c, 9, 38, 37
//9, 86592, Note_off_c, 9, 38, 0
//9, 87168, Note_on_c, 9, 38, 37
//9, 87360, Note_off_c, 9, 38, 0
//9, 87936, Note_on_c, 9, 38, 37
//9, 88128, Note_off_c, 9, 38, 0
//9, 88128, Note_on_c, 9, 38, 37
//9, 88224, Note_off_c, 9, 38, 0
//9, 88224, Note_on_c, 9, 38, 37
//9, 88320, Note_off_c, 9, 38, 0
//9, 88704, Note_on_c, 9, 38, 37
//9, 88896, Note_off_c, 9, 38, 0
//9, 89472, Note_on_c, 9, 38, 37
//9, 89664, Note_off_c, 9, 38, 0
//9, 90240, Note_on_c, 9, 38, 37
//9, 90432, Note_off_c, 9, 38, 0
//9, 91008, Note_on_c, 9, 38, 37
//9, 91200, Note_off_c, 9, 38, 0
//9, 91776, Note_on_c, 9, 38, 37
//9, 91968, Note_off_c, 9, 38, 0
//9, 92544, Note_on_c, 9, 38, 37
//9, 92736, Note_off_c, 9, 38, 0
//9, 93312, Note_on_c, 9, 38, 37
//9, 93504, Note_off_c, 9, 38, 0
//9, 94080, Note_on_c, 9, 38, 31
//9, 94128, Note_off_c, 9, 38, 0
//9, 94128, Note_on_c, 9, 38, 37
//9, 94320, Note_off_c, 9, 38, 0
//9, 94848, Note_on_c, 9, 38, 37
//9, 95040, Note_off_c, 9, 38, 0
//9, 95616, Note_on_c, 9, 38, 37
//9, 95808, Note_off_c, 9, 38, 0
//9, 96384, Note_on_c, 9, 38, 37
//9, 96576, Note_off_c, 9, 38, 0
//9, 97152, Note_on_c, 9, 38, 31
//9, 97200, Note_off_c, 9, 38, 0
//9, 97200, Note_on_c, 9, 38, 37
//9, 97392, Note_off_c, 9, 38, 0
//9, 97392, Note_on_c, 9, 38, 31
//9, 97440, Note_off_c, 9, 38, 0
//9, 97440, Note_on_c, 9, 38, 37
//9, 97632, Note_off_c, 9, 38, 0
//9, 97920, Note_on_c, 9, 38, 37
//9, 98112, Note_off_c, 9, 38, 0
//9, 98688, Note_on_c, 9, 38, 37
//9, 98880, Note_off_c, 9, 38, 0
//9, 99456, Note_on_c, 9, 38, 37
//9, 99648, Note_off_c, 9, 38, 0
//9, 100224, Note_on_c, 9, 38, 37
//9, 100416, Note_off_c, 9, 38, 0
//9, 100992, Note_on_c, 9, 38, 37
//9, 101184, Note_off_c, 9, 38, 0
//9, 101760, Note_on_c, 9, 38, 37
//9, 101952, Note_off_c, 9, 38, 0
//9, 103296, Note_on_c, 9, 38, 31
//9, 103344, Note_off_c, 9, 38, 0
//9, 103344, Note_on_c, 9, 38, 37
//9, 103536, Note_off_c, 9, 38, 0
//9, 104064, Note_on_c, 9, 38, 37
//9, 104448, Note_off_c, 9, 38, 0
//9, 104832, Note_on_c, 9, 38, 37
//9, 105216, Note_off_c, 9, 38, 0
//9, 105600, Note_on_c, 9, 38, 37
//9, 105792, Note_off_c, 9, 38, 0
//9, 106368, Note_on_c, 9, 38, 37
//9, 106752, Note_off_c, 9, 38, 0
//9, 107136, Note_on_c, 9, 38, 37
//9, 107520, Note_off_c, 9, 38, 0
//9, 107904, Note_on_c, 9, 38, 37
//9, 108288, Note_off_c, 9, 38, 0
//9, 108672, Note_on_c, 9, 38, 37
//9, 108864, Note_off_c, 9, 38, 0
//9, 109440, Note_on_c, 9, 38, 37
//9, 109824, Note_off_c, 9, 38, 0
//9, 110208, Note_on_c, 9, 38, 37
//9, 110592, Note_off_c, 9, 38, 0
//9, 110976, Note_on_c, 9, 38, 37
//9, 111360, Note_off_c, 9, 38, 0
//9, 111744, Note_on_c, 9, 38, 37
//9, 111936, Note_off_c, 9, 38, 0
//9, 112512, Note_on_c, 9, 38, 37
//9, 112896, Note_off_c, 9, 38, 0
//9, 113280, Note_on_c, 9, 38, 37
//9, 113664, Note_off_c, 9, 38, 0
//9, 114048, Note_on_c, 9, 38, 31
//9, 114096, Note_off_c, 9, 38, 0
//9, 114096, Note_on_c, 9, 38, 37
//9, 114480, Note_off_c, 9, 38, 0
//9, 114816, Note_on_c, 9, 38, 37
//9, 115008, Note_off_c, 9, 38, 0
//9, 115584, Note_on_c, 9, 38, 31
//9, 115632, Note_off_c, 9, 38, 0
//9, 115632, Note_on_c, 9, 38, 37
//9, 116016, Note_off_c, 9, 38, 0
//9, 116352, Note_on_c, 9, 38, 37
//9, 116544, Note_off_c, 9, 38, 0
//9, 117120, Note_on_c, 9, 38, 31
//9, 117168, Note_off_c, 9, 38, 0
//9, 117168, Note_on_c, 9, 38, 37
//9, 117552, Note_off_c, 9, 38, 0
//9, 117888, Note_on_c, 9, 38, 37
//9, 118080, Note_off_c, 9, 38, 0
//9, 118656, Note_on_c, 9, 38, 31
//9, 118704, Note_off_c, 9, 38, 0
//9, 119040, Note_on_c, 9, 38, 31
//9, 119040, Note_on_c, 9, 38, 31
//9, 119088, Note_off_c, 9, 38, 0
//9, 119232, Note_off_c, 9, 38, 0
//9, 119232, Note_on_c, 9, 38, 31
//9, 119424, Note_off_c, 9, 38, 0
//9, 119424, Note_on_c, 9, 38, 31
//9, 119616, Note_off_c, 9, 38, 0
//9, 119616, Note_on_c, 9, 38, 31
//9, 119808, Note_off_c, 9, 38, 0
//9, 119808, Note_on_c, 9, 38, 37
//9, 120000, Note_off_c, 9, 38, 0
//9, 120192, Note_on_c, 9, 38, 31
//9, 120240, Note_off_c, 9, 38, 0
//9, 120240, Note_on_c, 9, 38, 37
//9, 120432, Note_off_c, 9, 38, 0
//9, 120960, Note_on_c, 9, 38, 37
//9, 121152, Note_off_c, 9, 38, 0
//9, 121728, Note_on_c, 9, 38, 37
//9, 121920, Note_off_c, 9, 38, 0
//9, 122496, Note_on_c, 9, 38, 37
//9, 122688, Note_off_c, 9, 38, 0
//9, 123264, Note_on_c, 9, 38, 37
//9, 123456, Note_off_c, 9, 38, 0
//9, 124032, Note_on_c, 9, 38, 37
//9, 124224, Note_off_c, 9, 38, 0
//9, 124800, Note_on_c, 9, 38, 37
//9, 124992, Note_off_c, 9, 38, 0
//9, 125568, Note_on_c, 9, 38, 37
//9, 125760, Note_off_c, 9, 38, 0
//9, 125952, Note_on_c, 9, 38, 31
//9, 126000, Note_off_c, 9, 38, 0
//9, 126000, Note_on_c, 9, 38, 37
//9, 126192, Note_off_c, 9, 38, 0
//9, 126384, Note_on_c, 9, 38, 31
//9, 126432, Note_off_c, 9, 38, 0
//9, 126432, Note_on_c, 9, 38, 37
//9, 126624, Note_off_c, 9, 38, 0
//9, 127104, Note_on_c, 9, 38, 37
//9, 127296, Note_off_c, 9, 38, 0
//9, 127872, Note_on_c, 9, 38, 37
//9, 128064, Note_off_c, 9, 38, 0
//9, 128640, Note_on_c, 9, 38, 37
//9, 128832, Note_off_c, 9, 38, 0
//9, 129408, Note_on_c, 9, 38, 37
//9, 129600, Note_off_c, 9, 38, 0
//9, 130176, Note_on_c, 9, 38, 37
//9, 130368, Note_off_c, 9, 38, 0
//9, 130944, Note_on_c, 9, 38, 37
//9, 131136, Note_off_c, 9, 38, 0
//9, 131712, Note_on_c, 9, 38, 37
//9, 131904, Note_off_c, 9, 38, 0
//9, 132480, Note_on_c, 9, 38, 37
//9, 132672, Note_off_c, 9, 38, 0
//9, 133248, Note_on_c, 9, 38, 37
//9, 133440, Note_off_c, 9, 38, 0
//9, 134016, Note_on_c, 9, 38, 37
//9, 134208, Note_off_c, 9, 38, 0
//9, 134784, Note_on_c, 9, 38, 37
//9, 134976, Note_off_c, 9, 38, 0
//9, 135168, Note_on_c, 9, 38, 31
//9, 135216, Note_off_c, 9, 38, 0
//9, 135216, Note_on_c, 9, 38, 37
//9, 135408, Note_off_c, 9, 38, 0
//9, 136320, Note_on_c, 9, 38, 37
//9, 136512, Note_off_c, 9, 38, 0
//9, 137088, Note_on_c, 9, 38, 37
//9, 137280, Note_off_c, 9, 38, 0
//9, 137856, Note_on_c, 9, 38, 37
//9, 138048, Note_off_c, 9, 38, 0
//9, 139392, Note_on_c, 9, 38, 37
//9, 139584, Note_off_c, 9, 38, 0
//9, 140160, Note_on_c, 9, 38, 37
//9, 140352, Note_off_c, 9, 38, 0
//9, 140928, Note_on_c, 9, 38, 37
//9, 141120, Note_off_c, 9, 38, 0
//9, 141696, Note_on_c, 9, 38, 37
//9, 141888, Note_off_c, 9, 38, 0
//9, 142464, Note_on_c, 9, 38, 37
//9, 142656, Note_off_c, 9, 38, 0
//9, 143232, Note_on_c, 9, 38, 37
//9, 143424, Note_off_c, 9, 38, 0
//9, 144000, Note_on_c, 9, 38, 37
//9, 144192, Note_off_c, 9, 38, 0
//9, 144768, Note_on_c, 9, 38, 37
//9, 144960, Note_off_c, 9, 38, 0
//9, 145536, Note_on_c, 9, 38, 37
//9, 145728, Note_off_c, 9, 38, 0
//9, 146304, Note_on_c, 9, 38, 37
//9, 146496, Note_off_c, 9, 38, 0
//9, 147072, Note_on_c, 9, 38, 37
//9, 147264, Note_off_c, 9, 38, 0
//9, 147456, Note_on_c, 9, 38, 37
//9, 147840, Note_off_c, 9, 38, 0
//9, 147840, Note_on_c, 9, 38, 37
//9, 148224, Note_off_c, 9, 38, 0
//9, 148224, Note_on_c, 9, 38, 31
//9, 148416, Note_off_c, 9, 38, 0
//9, 148416, Note_on_c, 9, 38, 31
//9, 148608, Note_off_c, 9, 38, 0
//9, 148608, Note_on_c, 9, 38, 31
//9, 148800, Note_off_c, 9, 38, 0
//9, 148800, Note_on_c, 9, 38, 31
//9, 148992, Note_off_c, 9, 38, 0
//9, 148992, Note_on_c, 9, 38, 31
//9, 149184, Note_off_c, 9, 38, 0
//9, 149184, Note_on_c, 9, 38, 31
//9, 149376, Note_off_c, 9, 38, 0
//9, 149376, Note_on_c, 9, 38, 31
//9, 149568, Note_off_c, 9, 38, 0
//9, 149568, Note_on_c, 9, 38, 31
//9, 149760, Note_off_c, 9, 38, 0
//9, 149760, Note_on_c, 9, 38, 31
//9, 149952, Note_off_c, 9, 38, 0
//9, 149952, Note_on_c, 9, 38, 31
//9, 150144, Note_off_c, 9, 38, 0
//9, 150144, Note_on_c, 9, 38, 31
//9, 150336, Note_off_c, 9, 38, 0
//9, 150336, Note_on_c, 9, 38, 31
//9, 150528, Note_off_c, 9, 38, 0
//9, 150528, Note_on_c, 9, 38, 31
//9, 150720, Note_off_c, 9, 38, 0
//9, 150720, Note_on_c, 9, 38, 31
//9, 150912, Note_off_c, 9, 38, 0
//9, 150912, Note_on_c, 9, 38, 31
//9, 150960, Note_off_c, 9, 38, 0
//9, 150960, Note_on_c, 9, 38, 37
//9, 151152, Note_off_c, 9, 38, 0
//9, 172416, Note_on_c, 9, 38, 31
//9, 172464, Note_off_c, 9, 38, 0
//9, 172464, Note_on_c, 9, 38, 37
//9, 172656, Note_off_c, 9, 38, 0
//9, 172656, Note_on_c, 9, 38, 31
//9, 172704, Note_off_c, 9, 38, 0
//9, 172704, Note_on_c, 9, 38, 37
//9, 172896, Note_off_c, 9, 38, 0
//9, 175488, Note_on_c, 9, 38, 31
//9, 175536, Note_off_c, 9, 38, 0
//9, 175536, Note_on_c, 9, 38, 37
//9, 175728, Note_off_c, 9, 38, 0
//9, 176256, Note_on_c, 9, 38, 37
//9, 176448, Note_off_c, 9, 38, 0
//9, 177024, Note_on_c, 9, 38, 37
//9, 177216, Note_off_c, 9, 38, 0
//9, 177792, Note_on_c, 9, 38, 37
//9, 177984, Note_off_c, 9, 38, 0
//9, 178560, Note_on_c, 9, 38, 31
//9, 178608, Note_off_c, 9, 38, 0
//9, 178608, Note_on_c, 9, 38, 37
//9, 178800, Note_off_c, 9, 38, 0
//9, 178800, Note_on_c, 9, 38, 31
//9, 178848, Note_off_c, 9, 38, 0
//9, 178848, Note_on_c, 9, 38, 37
//9, 179040, Note_off_c, 9, 38, 0
//9, 179328, Note_on_c, 9, 38, 37
//9, 179520, Note_off_c, 9, 38, 0
//9, 180096, Note_on_c, 9, 38, 37
//9, 180288, Note_off_c, 9, 38, 0
//9, 180864, Note_on_c, 9, 38, 37
//9, 181056, Note_off_c, 9, 38, 0
//9, 181632, Note_on_c, 9, 38, 37
//9, 181824, Note_off_c, 9, 38, 0
//9, 182400, Note_on_c, 9, 38, 37
//9, 182592, Note_off_c, 9, 38, 0
//9, 183168, Note_on_c, 9, 38, 37
//9, 183360, Note_off_c, 9, 38, 0
//9, 183936, Note_on_c, 9, 38, 37
//9, 184128, Note_off_c, 9, 38, 0
//9, 184704, Note_on_c, 9, 38, 31
//9, 184752, Note_off_c, 9, 38, 0
//9, 184752, Note_on_c, 9, 38, 37
//9, 184944, Note_off_c, 9, 38, 0
//9, 184944, Note_on_c, 9, 38, 31
//9, 184992, Note_off_c, 9, 38, 0
//9, 184992, Note_on_c, 9, 38, 37
//9, 185184, Note_off_c, 9, 38, 0
//9, 185472, Note_on_c, 9, 38, 37
//9, 185664, Note_off_c, 9, 38, 0
//9, 186240, Note_on_c, 9, 38, 37
//9, 186432, Note_off_c, 9, 38, 0
//9, 187008, Note_on_c, 9, 38, 37
//9, 187200, Note_off_c, 9, 38, 0
//9, 187776, Note_on_c, 9, 38, 37
//9, 187968, Note_off_c, 9, 38, 0
//9, 188544, Note_on_c, 9, 38, 37
//9, 188736, Note_off_c, 9, 38, 0
//9, 189312, Note_on_c, 9, 38, 37
//9, 189504, Note_off_c, 9, 38, 0
//9, 190080, Note_on_c, 9, 38, 37
//9, 190272, Note_off_c, 9, 38, 0
//9, 190848, Note_on_c, 9, 38, 31
//9, 190896, Note_off_c, 9, 38, 0
//9, 190896, Note_on_c, 9, 38, 37
//9, 191088, Note_off_c, 9, 38, 0
//9, 191088, Note_on_c, 9, 38, 31
//9, 191136, Note_off_c, 9, 38, 0
//9, 191136, Note_on_c, 9, 38, 37
//9, 191328, Note_off_c, 9, 38, 0
//9, 191616, Note_on_c, 9, 38, 37
//9, 191808, Note_off_c, 9, 38, 0
//9, 192384, Note_on_c, 9, 38, 37
//9, 192576, Note_off_c, 9, 38, 0
//9, 193152, Note_on_c, 9, 38, 37
//9, 193344, Note_off_c, 9, 38, 0
//9, 193920, Note_on_c, 9, 38, 37
//9, 194112, Note_off_c, 9, 38, 0
//9, 194688, Note_on_c, 9, 38, 37
//9, 194880, Note_off_c, 9, 38, 0
//9, 195456, Note_on_c, 9, 38, 37
//9, 195648, Note_off_c, 9, 38, 0
//9, 196224, Note_on_c, 9, 38, 37
//9, 196416, Note_off_c, 9, 38, 0
//9, 196992, Note_on_c, 9, 38, 31
//9, 197040, Note_off_c, 9, 38, 0
//9, 197040, Note_on_c, 9, 38, 37
//9, 197232, Note_off_c, 9, 38, 0
//9, 197232, Note_on_c, 9, 38, 31
//9, 197280, Note_off_c, 9, 38, 0
//9, 197280, Note_on_c, 9, 38, 37
//9, 197472, Note_off_c, 9, 38, 0
//9, 197472, End_track
//
//10, 0, Start_track
//10, 0, Title_t, "Toms"
//10, 0, Program_c, 9, 118
//10, 151968, Note_on_c, 9, 48, 31
//10, 152064, Note_off_c, 9, 48, 0
//10, 152160, Note_on_c, 9, 48, 31
//10, 152256, Note_off_c, 9, 48, 0
//10, 152736, Note_on_c, 9, 48, 31
//10, 152832, Note_off_c, 9, 48, 0
//10, 153504, Note_on_c, 9, 48, 31
//10, 153600, Note_off_c, 9, 48, 0
//10, 153696, Note_on_c, 9, 48, 31
//10, 153792, Note_off_c, 9, 48, 0
//10, 154752, Note_on_c, 9, 43, 37
//10, 154848, Note_off_c, 9, 43, 0
//10, 155040, Note_on_c, 9, 62, 37
//10, 155040, Note_on_c, 9, 62, 37
//10, 155040, Note_on_c, 9, 48, 37
//10, 155136, Note_off_c, 9, 48, 0
//10, 155136, Note_off_c, 9, 62, 0
//10, 155136, Note_off_c, 9, 62, 0
//10, 155136, Note_on_c, 9, 43, 37
//10, 155232, Note_off_c, 9, 43, 0
//10, 155232, Note_on_c, 9, 62, 37
//10, 155232, Note_on_c, 9, 62, 37
//10, 155232, Note_on_c, 9, 48, 37
//10, 155328, Note_off_c, 9, 48, 0
//10, 155328, Note_off_c, 9, 62, 0
//10, 155328, Note_off_c, 9, 62, 0
//10, 155520, Note_on_c, 9, 43, 37
//10, 155520, Note_on_c, 9, 48, 37
//10, 155616, Note_off_c, 9, 48, 0
//10, 155616, Note_off_c, 9, 43, 0
//10, 155808, Note_on_c, 9, 62, 37
//10, 155808, Note_on_c, 9, 62, 37
//10, 155904, Note_off_c, 9, 62, 0
//10, 155904, Note_off_c, 9, 62, 0
//10, 155904, Note_on_c, 9, 43, 37
//10, 156000, Note_off_c, 9, 43, 0
//10, 156288, Note_on_c, 9, 43, 37
//10, 156384, Note_off_c, 9, 43, 0
//10, 156576, Note_on_c, 9, 62, 37
//10, 156576, Note_on_c, 9, 62, 37
//10, 156576, Note_on_c, 9, 48, 37
//10, 156672, Note_off_c, 9, 48, 0
//10, 156672, Note_off_c, 9, 62, 0
//10, 156672, Note_off_c, 9, 62, 0
//10, 156672, Note_on_c, 9, 43, 37
//10, 156768, Note_off_c, 9, 43, 0
//10, 156768, Note_on_c, 9, 62, 37
//10, 156768, Note_on_c, 9, 62, 37
//10, 156768, Note_on_c, 9, 48, 37
//10, 156864, Note_off_c, 9, 48, 0
//10, 156864, Note_off_c, 9, 62, 0
//10, 156864, Note_off_c, 9, 62, 0
//10, 157056, Note_on_c, 9, 43, 37
//10, 157056, Note_on_c, 9, 48, 37
//10, 157152, Note_off_c, 9, 48, 0
//10, 157152, Note_off_c, 9, 43, 0
//10, 157344, Note_on_c, 9, 62, 37
//10, 157344, Note_on_c, 9, 62, 37
//10, 157440, Note_off_c, 9, 62, 0
//10, 157440, Note_off_c, 9, 62, 0
//10, 157440, Note_on_c, 9, 43, 37
//10, 157536, Note_off_c, 9, 43, 0
//10, 157824, Note_on_c, 9, 43, 37
//10, 157920, Note_off_c, 9, 43, 0
//10, 158112, Note_on_c, 9, 62, 37
//10, 158112, Note_on_c, 9, 62, 37
//10, 158112, Note_on_c, 9, 48, 37
//10, 158208, Note_off_c, 9, 48, 0
//10, 158208, Note_off_c, 9, 62, 0
//10, 158208, Note_off_c, 9, 62, 0
//10, 158208, Note_on_c, 9, 43, 37
//10, 158304, Note_off_c, 9, 43, 0
//10, 158304, Note_on_c, 9, 62, 37
//10, 158304, Note_on_c, 9, 62, 37
//10, 158304, Note_on_c, 9, 48, 37
//10, 158400, Note_off_c, 9, 48, 0
//10, 158400, Note_off_c, 9, 62, 0
//10, 158400, Note_off_c, 9, 62, 0
//10, 158592, Note_on_c, 9, 43, 37
//10, 158592, Note_on_c, 9, 48, 37
//10, 158688, Note_off_c, 9, 48, 0
//10, 158688, Note_off_c, 9, 43, 0
//10, 158880, Note_on_c, 9, 62, 37
//10, 158880, Note_on_c, 9, 62, 37
//10, 158976, Note_off_c, 9, 62, 0
//10, 158976, Note_off_c, 9, 62, 0
//10, 158976, Note_on_c, 9, 43, 37
//10, 159072, Note_off_c, 9, 43, 0
//10, 159360, Note_on_c, 9, 43, 37
//10, 159456, Note_off_c, 9, 43, 0
//10, 159648, Note_on_c, 9, 62, 37
//10, 159648, Note_on_c, 9, 62, 37
//10, 159648, Note_on_c, 9, 48, 37
//10, 159744, Note_off_c, 9, 48, 0
//10, 159744, Note_off_c, 9, 62, 0
//10, 159744, Note_off_c, 9, 62, 0
//10, 159744, Note_on_c, 9, 43, 37
//10, 159840, Note_off_c, 9, 43, 0
//10, 159840, Note_on_c, 9, 62, 37
//10, 159840, Note_on_c, 9, 62, 37
//10, 159840, Note_on_c, 9, 48, 37
//10, 159936, Note_off_c, 9, 48, 0
//10, 159936, Note_off_c, 9, 62, 0
//10, 159936, Note_off_c, 9, 62, 0
//10, 160128, Note_on_c, 9, 43, 37
//10, 160128, Note_on_c, 9, 48, 37
//10, 160224, Note_off_c, 9, 48, 0
//10, 160224, Note_off_c, 9, 43, 0
//10, 160416, Note_on_c, 9, 62, 37
//10, 160416, Note_on_c, 9, 62, 37
//10, 160512, Note_off_c, 9, 62, 0
//10, 160512, Note_off_c, 9, 62, 0
//10, 160512, Note_on_c, 9, 43, 37
//10, 160608, Note_off_c, 9, 43, 0
//10, 160896, Note_on_c, 9, 43, 37
//10, 160992, Note_off_c, 9, 43, 0
//10, 161184, Note_on_c, 9, 62, 37
//10, 161184, Note_on_c, 9, 62, 37
//10, 161184, Note_on_c, 9, 48, 37
//10, 161280, Note_off_c, 9, 48, 0
//10, 161280, Note_off_c, 9, 62, 0
//10, 161280, Note_off_c, 9, 62, 0
//10, 161280, Note_on_c, 9, 43, 37
//10, 161376, Note_off_c, 9, 43, 0
//10, 161376, Note_on_c, 9, 62, 37
//10, 161376, Note_on_c, 9, 62, 37
//10, 161376, Note_on_c, 9, 48, 37
//10, 161472, Note_off_c, 9, 48, 0
//10, 161472, Note_off_c, 9, 62, 0
//10, 161472, Note_off_c, 9, 62, 0
//10, 161664, Note_on_c, 9, 43, 37
//10, 161664, Note_on_c, 9, 48, 37
//10, 161760, Note_off_c, 9, 48, 0
//10, 161760, Note_off_c, 9, 43, 0
//10, 161952, Note_on_c, 9, 62, 37
//10, 161952, Note_on_c, 9, 62, 37
//10, 162048, Note_off_c, 9, 62, 0
//10, 162048, Note_off_c, 9, 62, 0
//10, 162048, Note_on_c, 9, 43, 37
//10, 162144, Note_off_c, 9, 43, 0
//10, 162432, Note_on_c, 9, 43, 37
//10, 162528, Note_off_c, 9, 43, 0
//10, 162720, Note_on_c, 9, 62, 37
//10, 162720, Note_on_c, 9, 62, 37
//10, 162720, Note_on_c, 9, 48, 37
//10, 162816, Note_off_c, 9, 48, 0
//10, 162816, Note_off_c, 9, 62, 0
//10, 162816, Note_off_c, 9, 62, 0
//10, 162816, Note_on_c, 9, 43, 37
//10, 162912, Note_off_c, 9, 43, 0
//10, 162912, Note_on_c, 9, 62, 37
//10, 162912, Note_on_c, 9, 62, 37
//10, 162912, Note_on_c, 9, 48, 37
//10, 163008, Note_off_c, 9, 48, 0
//10, 163008, Note_off_c, 9, 62, 0
//10, 163008, Note_off_c, 9, 62, 0
//10, 163200, Note_on_c, 9, 43, 37
//10, 163200, Note_on_c, 9, 48, 37
//10, 163296, Note_off_c, 9, 48, 0
//10, 163296, Note_off_c, 9, 43, 0
//10, 163488, Note_on_c, 9, 62, 37
//10, 163488, Note_on_c, 9, 62, 37
//10, 163584, Note_off_c, 9, 62, 0
//10, 163584, Note_off_c, 9, 62, 0
//10, 163584, Note_on_c, 9, 43, 37
//10, 163680, Note_off_c, 9, 43, 0
//10, 163968, Note_on_c, 9, 43, 37
//10, 164064, Note_off_c, 9, 43, 0
//10, 164256, Note_on_c, 9, 62, 37
//10, 164256, Note_on_c, 9, 62, 37
//10, 164256, Note_on_c, 9, 48, 37
//10, 164352, Note_off_c, 9, 48, 0
//10, 164352, Note_off_c, 9, 62, 0
//10, 164352, Note_off_c, 9, 62, 0
//10, 164352, Note_on_c, 9, 43, 37
//10, 164448, Note_off_c, 9, 43, 0
//10, 164448, Note_on_c, 9, 62, 37
//10, 164448, Note_on_c, 9, 62, 37
//10, 164448, Note_on_c, 9, 48, 37
//10, 164544, Note_off_c, 9, 48, 0
//10, 164544, Note_off_c, 9, 62, 0
//10, 164544, Note_off_c, 9, 62, 0
//10, 164736, Note_on_c, 9, 43, 37
//10, 164736, Note_on_c, 9, 48, 37
//10, 164832, Note_off_c, 9, 48, 0
//10, 164832, Note_off_c, 9, 43, 0
//10, 165024, Note_on_c, 9, 62, 37
//10, 165024, Note_on_c, 9, 62, 37
//10, 165120, Note_off_c, 9, 62, 0
//10, 165120, Note_off_c, 9, 62, 0
//10, 165120, Note_on_c, 9, 43, 37
//10, 165216, Note_off_c, 9, 43, 0
//10, 165504, Note_on_c, 9, 43, 37
//10, 165600, Note_off_c, 9, 43, 0
//10, 165792, Note_on_c, 9, 62, 37
//10, 165792, Note_on_c, 9, 62, 37
//10, 165792, Note_on_c, 9, 48, 37
//10, 165888, Note_off_c, 9, 48, 0
//10, 165888, Note_off_c, 9, 62, 0
//10, 165888, Note_off_c, 9, 62, 0
//10, 165888, Note_on_c, 9, 43, 37
//10, 165984, Note_off_c, 9, 43, 0
//10, 165984, Note_on_c, 9, 62, 37
//10, 165984, Note_on_c, 9, 62, 37
//10, 165984, Note_on_c, 9, 48, 37
//10, 166080, Note_off_c, 9, 48, 0
//10, 166080, Note_off_c, 9, 62, 0
//10, 166080, Note_off_c, 9, 62, 0
//10, 166272, Note_on_c, 9, 43, 37
//10, 166272, Note_on_c, 9, 48, 37
//10, 166368, Note_off_c, 9, 48, 0
//10, 166368, Note_off_c, 9, 43, 0
//10, 166560, Note_on_c, 9, 62, 37
//10, 166560, Note_on_c, 9, 62, 37
//10, 166656, Note_off_c, 9, 62, 0
//10, 166656, Note_off_c, 9, 62, 0
//10, 166656, Note_on_c, 9, 43, 37
//10, 166752, Note_off_c, 9, 43, 0
//10, 167040, Note_on_c, 9, 43, 37
//10, 167136, Note_off_c, 9, 43, 0
//10, 167328, Note_on_c, 9, 62, 37
//10, 167328, Note_on_c, 9, 62, 37
//10, 167328, Note_on_c, 9, 48, 37
//10, 167424, Note_off_c, 9, 48, 0
//10, 167424, Note_off_c, 9, 62, 0
//10, 167424, Note_off_c, 9, 62, 0
//10, 167424, Note_on_c, 9, 43, 37
//10, 167520, Note_off_c, 9, 43, 0
//10, 167520, Note_on_c, 9, 62, 37
//10, 167520, Note_on_c, 9, 62, 37
//10, 167520, Note_on_c, 9, 48, 37
//10, 167616, Note_off_c, 9, 48, 0
//10, 167616, Note_off_c, 9, 62, 0
//10, 167616, Note_off_c, 9, 62, 0
//10, 167808, Note_on_c, 9, 43, 37
//10, 167808, Note_on_c, 9, 48, 37
//10, 167904, Note_off_c, 9, 48, 0
//10, 167904, Note_off_c, 9, 43, 0
//10, 168096, Note_on_c, 9, 62, 37
//10, 168096, Note_on_c, 9, 62, 37
//10, 168192, Note_off_c, 9, 62, 0
//10, 168192, Note_off_c, 9, 62, 0
//10, 168192, Note_on_c, 9, 43, 37
//10, 168288, Note_off_c, 9, 43, 0
//10, 168576, Note_on_c, 9, 43, 37
//10, 168672, Note_off_c, 9, 43, 0
//10, 168864, Note_on_c, 9, 62, 37
//10, 168864, Note_on_c, 9, 62, 37
//10, 168864, Note_on_c, 9, 48, 37
//10, 168960, Note_off_c, 9, 48, 0
//10, 168960, Note_off_c, 9, 62, 0
//10, 168960, Note_off_c, 9, 62, 0
//10, 168960, Note_on_c, 9, 43, 37
//10, 169056, Note_off_c, 9, 43, 0
//10, 169056, Note_on_c, 9, 62, 37
//10, 169056, Note_on_c, 9, 62, 37
//10, 169056, Note_on_c, 9, 48, 37
//10, 169152, Note_off_c, 9, 48, 0
//10, 169152, Note_off_c, 9, 62, 0
//10, 169152, Note_off_c, 9, 62, 0
//10, 169344, Note_on_c, 9, 43, 37
//10, 169344, Note_on_c, 9, 48, 37
//10, 169440, Note_off_c, 9, 48, 0
//10, 169440, Note_off_c, 9, 43, 0
//10, 169632, Note_on_c, 9, 62, 37
//10, 169632, Note_on_c, 9, 62, 37
//10, 169728, Note_off_c, 9, 62, 0
//10, 169728, Note_off_c, 9, 62, 0
//10, 169728, Note_on_c, 9, 43, 37
//10, 169824, Note_off_c, 9, 43, 0
//10, 170112, Note_on_c, 9, 43, 37
//10, 170208, Note_off_c, 9, 43, 0
//10, 170400, Note_on_c, 9, 62, 37
//10, 170400, Note_on_c, 9, 62, 37
//10, 170400, Note_on_c, 9, 48, 37
//10, 170496, Note_off_c, 9, 48, 0
//10, 170496, Note_off_c, 9, 62, 0
//10, 170496, Note_off_c, 9, 62, 0
//10, 170496, Note_on_c, 9, 43, 37
//10, 170592, Note_off_c, 9, 43, 0
//10, 170592, Note_on_c, 9, 62, 37
//10, 170592, Note_on_c, 9, 62, 37
//10, 170592, Note_on_c, 9, 48, 37
//10, 170688, Note_off_c, 9, 48, 0
//10, 170688, Note_off_c, 9, 62, 0
//10, 170688, Note_off_c, 9, 62, 0
//10, 170880, Note_on_c, 9, 43, 37
//10, 170880, Note_on_c, 9, 48, 37
//10, 170976, Note_off_c, 9, 48, 0
//10, 170976, Note_off_c, 9, 43, 0
//10, 171168, Note_on_c, 9, 62, 37
//10, 171168, Note_on_c, 9, 62, 37
//10, 171264, Note_off_c, 9, 62, 0
//10, 171264, Note_off_c, 9, 62, 0
//10, 171264, Note_on_c, 9, 43, 37
//10, 171360, Note_off_c, 9, 43, 0
//10, 171648, Note_on_c, 9, 43, 37
//10, 171744, Note_off_c, 9, 43, 0
//10, 171936, Note_on_c, 9, 62, 37
//10, 171936, Note_on_c, 9, 62, 37
//10, 171936, Note_on_c, 9, 48, 37
//10, 172032, Note_off_c, 9, 48, 0
//10, 172032, Note_off_c, 9, 62, 0
//10, 172032, Note_off_c, 9, 62, 0
//10, 172032, Note_on_c, 9, 43, 37
//10, 172128, Note_off_c, 9, 43, 0
//10, 172128, Note_on_c, 9, 62, 37
//10, 172128, Note_on_c, 9, 62, 37
//10, 172128, Note_on_c, 9, 48, 37
//10, 172224, Note_off_c, 9, 48, 0
//10, 172224, Note_off_c, 9, 62, 0
//10, 172224, Note_off_c, 9, 62, 0
//10, 172224, End_track
