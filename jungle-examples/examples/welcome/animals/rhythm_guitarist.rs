use jungle_sdk::prelude::*;

use crate::effect::{AtomicDualHit, ImmediateDyad, ImmediateMonad, Rest, SyncedRest, Tetrad};
use crate::instrumentation::{
    ElectricGuitar, ElectricGuitarArticulation, Pick as LanePick, Pluck as LanePluck,
    Strum as LaneStrum, Vocals, VocalsArticulation,
};

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RhythmGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
    transition_loops_remaining: u8,
    sustain_loops_remaining: u8,
}

impl Default for RhythmGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::default(),
            riff_loops_remaining: 5,
            transition_loops_remaining: 3,
            sustain_loops_remaining: 1,
        }
    }
}

pub type RhythmGuitaristSeed = ();
const RHYTHM_GUITAR_LANE_ID: u32 = <<RhythmGuitarist as Animal>::Id as AnimalIdValue>::U32;
const INTRO_START_DELAY_TICKS: u32 = 0;

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = RhythmGuitarFlow;
}

type Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePick<NOTE, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;
type Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;
type Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
> = LaneStrum<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;

pub struct JoinPick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for JoinPick<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = ImmediateMonad<ElectricGuitar, ElectricGuitarArticulation, NOTE, NOTE_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join note playback should succeed");
    }
}

pub struct JoinPluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for JoinPluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect =
        ImmediateDyad<ElectricGuitar, ElectricGuitarArticulation, NOTE_1, NOTE_2, NOTE_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join chord playback should succeed");
    }
}

pub struct Chord<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > Act for Chord<NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_TICK, REST_TICK>
{
    type Effect = Tetrad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        RHYTHM_GUITAR_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_3,
        NOTE_4,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct SplitPluck<
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
    > Act for SplitPluck<NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
{
    type Effect = AtomicDualHit<
        ElectricGuitar,
        ElectricGuitar,
        ElectricGuitarArticulation,
        ElectricGuitarArticulation,
        RHYTHM_GUITAR_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK_1,
        NOTE_TICK_2,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        (*state, *state)
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<RHYTHM_GUITAR_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct HarmonySing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for HarmonySing<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = ImmediateMonad<Vocals, VocalsArticulation, NOTE, NOTE_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        VocalsArticulation::GroupHarmony
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("backup vocal playback should succeed");
    }
}

pub struct MergeUnit;
#[jungle::act]
impl Act for MergeUnit {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join merge should complete");
    }
}

pub struct PostMergeRest;
#[jungle::act]
impl Act for PostMergeRest {
    type Effect = SyncedRest<RHYTHM_GUITAR_LANE_ID, 384, 384>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("post-merge rest should complete");
    }
}

#[derive(Flow)]
pub struct RhythmGuitarFlow(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, RhythmSection01>,
    Transparent<IntroSectionMeta, RhythmSection02>,
    Transparent<IntroSectionMeta, RhythmSection03>,
    Transparent<IntroSectionMeta, RhythmSection04>,
    Transparent<IntroSectionMeta, RhythmSection05>,
    Transparent<IntroSectionMeta, RhythmSection06>,
    Transparent<IntroSectionMeta, RhythmSection07>,
);

#[derive(Flow)]
pub struct RhythmSection01(
    Transparent<IntroSectionMeta, RhythmPart01>,
    Transparent<IntroSectionMeta, RhythmPart02>,
    Transparent<IntroSectionMeta, RhythmPart03>,
    Transparent<IntroSectionMeta, RhythmPart04>,
    Transparent<IntroSectionMeta, RhythmPart05>,
    Transparent<IntroSectionMeta, RhythmPart06>,
);

#[derive(Flow)]
pub struct RhythmSection02(
    Transparent<IntroSectionMeta, RhythmPart07>,
    Transparent<IntroSectionMeta, RhythmPart08>,
    Transparent<IntroSectionMeta, RhythmPart09>,
    Transparent<IntroSectionMeta, RhythmPart10>,
    Transparent<IntroSectionMeta, RhythmPart11>,
    Transparent<IntroSectionMeta, RhythmPart12>,
);

#[derive(Flow)]
pub struct RhythmSection03(
    Transparent<IntroSectionMeta, RhythmPart13>,
    Transparent<IntroSectionMeta, RhythmPart14>,
    Transparent<IntroSectionMeta, RhythmPart15>,
    Transparent<IntroSectionMeta, RhythmPart16>,
    Transparent<IntroSectionMeta, RhythmPart17>,
    Transparent<IntroSectionMeta, RhythmPart18>,
);

#[derive(Flow)]
pub struct RhythmSection04(
    Transparent<IntroSectionMeta, RhythmPart19>,
    Transparent<IntroSectionMeta, RhythmPart20>,
    Transparent<IntroSectionMeta, RhythmPart21>,
    Transparent<IntroSectionMeta, RhythmPart22>,
    Transparent<IntroSectionMeta, RhythmPart23>,
    Transparent<IntroSectionMeta, RhythmPart24>,
);

#[derive(Flow)]
pub struct RhythmSection05(
    Transparent<IntroSectionMeta, RhythmPart25>,
    Transparent<IntroSectionMeta, RhythmPart26>,
    Transparent<IntroSectionMeta, RhythmPart27>,
    Transparent<IntroSectionMeta, RhythmPart28>,
    Transparent<IntroSectionMeta, RhythmPart29>,
    Transparent<IntroSectionMeta, RhythmPart30>,
);

#[derive(Flow)]
pub struct RhythmSection06(
    Transparent<IntroSectionMeta, RhythmPart31>,
    Transparent<IntroSectionMeta, RhythmPart32>,
    Transparent<IntroSectionMeta, RhythmPart33>,
    Transparent<IntroSectionMeta, RhythmPart34>,
    Transparent<IntroSectionMeta, RhythmPart35>,
    Transparent<IntroSectionMeta, RhythmPart36>,
);

#[derive(Flow)]
pub struct RhythmSection07(
    Transparent<IntroSectionMeta, RhythmPart37>,
    Transparent<IntroSectionMeta, RhythmPart38>,
    Transparent<IntroSectionMeta, RhythmPart39>,
    Transparent<IntroSectionMeta, RhythmPart40>,
    Transparent<IntroSectionMeta, RhythmPart41>,
    Transparent<IntroSectionMeta, RhythmPart42>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart01(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart02(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart03(
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart04(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart05(
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Pluck<49, 46, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart06(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Strum<44, 49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart07(
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Strum<44, 49, 46, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Strum<44, 49, 46, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart08(
    Step<Pluck<56, 49, 768, 768>>,
    Step<Pluck<51, 44, 768, 768>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 192, 192>>,
    Step<Pluck<58, 46, 384, 384>>,
    Step<Pluck<58, 46, 384, 384>>,
    Step<Pluck<51, 44, 384, 384>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pluck<54, 49, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart09(
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<51, 44, 96, 96>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<54, 49, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<51, 44, 96, 96>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<54, 49, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<63, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart10(
    Step<Pick<68, 480, 480>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<41, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart11(
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<58, 51, 384, 384>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart12(
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart13(
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart14(
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 288>>,
    Step<Pick<49, 96, 288>>,
    Step<Pick<45, 96, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart15(
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<43, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart16(
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<58, 51, 384, 384>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart17(
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart18(
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart19(
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<53, 46, 288, 288>>,
    Step<Pluck<53, 46, 96, 96>>,
    Step<Pluck<53, 46, 576, 576>>,
    Step<Pluck<53, 46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart20(
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pick<39, 384, 384>>,
    Step<Pluck<51, 39, 192, 192>>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<54, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<46, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<42, 96, 96>>,
    Step<Pick<41, 96, 96>>,
    Step<Pick<39, 192, 192>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<39, 384, 384>>,
    Step<Strum<63, 58, 51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart21(
    Step<Strum<61, 56, 49, 96, 96>>,
    Step<Strum<63, 58, 51, 96, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Strum<63, 58, 51, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Strum<63, 58, 51, 192, 192>>,
    Step<Pick<58, 192, 192>>,
    Step<Strum<63, 58, 51, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<63, 58, 96, 96>>,
    Step<Pluck<66, 60, 192, 192>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<42, 96, 96>>,
    Step<Pick<41, 96, 96>>,
    Step<Pick<39, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<58, 51, 384, 384>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<63, 58, 384, 384>>,
    Step<Pick<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart22(
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<65, 96, 96>>,
    Step<Pick<68, 96, 96>>,
    Step<Pick<61, 192, 192>>,
    Step<Pluck<63, 58, 384, 384>>,
    Step<Pluck<58, 54, 96, 96>>,
    Step<Pluck<58, 54, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<57, 50, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<57, 50, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<57, 50, 192, 192>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<57, 50, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 960, 960>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart23(
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart24(
    Step<Pick<39, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 192>>,
    Step<Pick<42, 96, 96>>,
    Step<SplitPluck<42, 49, 96, 192, 192>>,
    Step<SplitPluck<41, 48, 96, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<58, 51, 384, 384>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Strum<54, 49, 44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart25(
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<56, 49, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<49, 44, 96, 96>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 49, 96, 96>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart26(
    Step<Pick<46, 192, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart27(
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 288, 288>>,
    Step<Pick<49, 288, 288>>,
    Step<Pick<45, 384, 384>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<43, 192, 192>>,
    Step<Pick<49, 1152, 1152>>,
    Step<Pluck<59, 52, 192, 384>>,
    Step<Pluck<49, 42, 1152, 1152>>,
    Step<Pluck<64, 58, 192, 384>>,
    Step<Pluck<61, 49, 576, 576>>,
    Step<Pluck<61, 56, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart28(
    Step<Pick<65, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pluck<59, 52, 192, 384>>,
    Step<Pluck<70, 66, 576, 576>>,
    Step<Pick<71, 192, 192>>,
    Step<Pick<70, 192, 192>>,
    Step<Pick<61, 96, 96>>,
    Step<Pick<59, 96, 96>>,
    Step<Pick<59, 192, 192>>,
    Step<Pick<66, 192, 192>>,
    Step<Pick<65, 192, 192>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pick<55, 192, 192>>,
    Step<Pick<56, 384, 384>>,
    Step<Pluck<59, 52, 192, 384>>,
    Step<Pick<70, 576, 576>>,
    Step<Pick<71, 192, 192>>,
    Step<Pick<70, 192, 192>>,
    Step<Pick<61, 96, 96>>,
    Step<Pick<59, 96, 96>>,
    Step<Pluck<68, 59, 192, 192>>,
    Step<Pluck<65, 56, 192, 192>>,
    Step<Pluck<65, 61, 1152, 1152>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart29(
    Step<Pluck<59, 52, 192, 384>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 192, 192>>,
    Step<Pluck<66, 61, 96, 96>>,
    Step<Pluck<66, 61, 96, 96>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pluck<61, 54, 192, 192>>,
    Step<Pluck<61, 54, 192, 192>>,
    Step<Pluck<61, 54, 192, 192>>,
    Step<Pluck<61, 54, 192, 192>>,
    Step<Pluck<61, 54, 384, 384>>,
    Step<Pluck<59, 52, 384, 384>>,
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<63, 56, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart30(
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<63, 56, 192, 192>>,
    Step<Pluck<61, 54, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<60, 54, 192, 192>>,
    Step<Pluck<61, 55, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pluck<60, 54, 192, 192>>,
    Step<Pluck<61, 55, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pick<66, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<60, 54, 192, 192>>,
    Step<Pluck<61, 55, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart31(
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pluck<60, 54, 192, 192>>,
    Step<Pluck<61, 55, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<66, 61, 384, 384>>,
    Step<Pick<66, 192, 192>>,
    Step<Pluck<55, 48, 384, 384>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<51, 46, 192, 192>>,
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<54, 47, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart32(
    Step<Pluck<55, 48, 384, 384>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<51, 46, 192, 192>>,
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<54, 47, 192, 192>>,
    Step<Pluck<55, 48, 384, 384>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<55, 48, 192, 192>>,
    Step<Pluck<51, 46, 192, 192>>,
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart33(
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<46, 39, 384, 384>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<44, 39, 96, 192>>,
    Step<Pluck<46, 39, 192, 192>>,
    Step<Pick<43, 192, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Strum<58, 53, 46, 384, 384>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart34(
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<49, 46, 96, 192>>,
    Step<Pluck<51, 44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Strum<51, 46, 39, 384, 384>>,
    Step<Strum<63, 58, 51, 192, 192>>,
    Step<Strum<61, 56, 51, 192, 192>>,
    Step<Pick<53, 192, 192>>,
    Step<Pluck<61, 56, 192, 192>>,
    Step<Pluck<63, 58, 192, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Chord<63, 58, 51, 39, 384, 384>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<56, 51, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<53, 46, 576, 576>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart35(
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<53, 46, 192, 192>>,
    Step<Pluck<53, 46, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<46, 39, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<46, 39, 192, 192>>,
    Step<Pluck<48, 41, 192, 192>>,
    Step<Pluck<46, 39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart36(
    Step<Pluck<48, 41, 384, 384>>,
    Step<Pluck<50, 46, 2688, 3072>>,
    Step<Pluck<61, 57, 1344, 1344>>,
    Step<Pluck<66, 78, 1728, 96>>,
    Step<Pluck<61, 73, 1632, 96>>,
    Step<Pluck<57, 69, 1536, 2880>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<60, 56, 288, 288>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pluck<60, 56, 192, 768>>,
    Step<Pick<57, 96, 1344>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart37(
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<60, 56, 288, 288>>,
    Step<Pluck<58, 54, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pluck<60, 56, 192, 768>>,
    Step<Pick<57, 96, 1344>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Strum<54, 49, 44, 96, 96>>,
    Step<Pluck<60, 56, 288, 288>>,
    Step<Pluck<58, 54, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pluck<60, 56, 192, 768>>,
    Step<Pick<51, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart38(
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<48, 96, 96>>,
    Step<Pick<48, 96, 96>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<46, 96, 96>>,
    Step<Pick<46, 96, 96>>,
    Step<Pluck<48, 41, 384, 384>>,
    Step<Pluck<47, 40, 384, 384>>,
    Step<Pluck<48, 41, 384, 384>>,
    Step<Pluck<49, 42, 384, 384>>,
    Step<Pluck<51, 44, 384, 384>>,
    Step<Pluck<50, 43, 384, 384>>,
    Step<Pluck<51, 44, 384, 384>>,
    Step<Pluck<52, 45, 192, 192>>,
    Step<Pick<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart39(
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart40(
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart41(
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pluck<58, 51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart42(
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<Step<JoinPluck<54, 47, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<42, 384, 384>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPluck<56, 49, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<48, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<46, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Join<Step<JoinPick<44, 384, 384>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeUnit>,
    Step<PostMergeRest>,
    Step<Pluck<58, 51, 288, 288>>,
    Step<Pluck<56, 49, 288, 288>>,
    Step<Pluck<52, 45, 288, 288>>,
    Step<Pluck<51, 44, 288, 288>>,
    Step<Pluck<49, 42, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<51, 44, 288, 288>>,
    Step<Pluck<49, 42, 288, 288>>,
    Step<Pluck<46, 39, 192, 576>>,
    Step<Pluck<46, 39, 3456, 0>>,
);
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::RhythmGuitarist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-rhythm-intro-test")
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
            .start_journey::<RhythmGuitarist>(seed)
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

