use jungle_sdk::prelude::*;

use super::{Double, Quad};
use crate::effect::{AtomicDualHit, Dyad, Monad, Rest, Tetrad};
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
type Pick58Tick = Step<Pick<58, 96, 96>>;
type Pluck58Tick = Step<Pluck<58, 58, 96, 96>>;
type Pluck56Tick = Step<Pluck<56, 56, 96, 96>>;
type Pluck53Tick = Step<Pluck<53, 53, 96, 96>>;
type Pluck51Tick = Step<Pluck<51, 51, 96, 96>>;
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
    type Effect = Monad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        RHYTHM_GUITAR_LANE_ID,
        NOTE,
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
        output.expect("join note playback should succeed");
    }
}

pub struct JoinPluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for JoinPluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect = Dyad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        RHYTHM_GUITAR_LANE_ID,
        NOTE_1,
        NOTE_2,
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
    type Effect = Monad<
        Vocals,
        VocalsArticulation,
        RHYTHM_GUITAR_LANE_ID,
        NOTE,
        NOTE_TICK,
        REST_TICK,
    >;
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

pub struct PostMergeRest<const REST_TICK: u32>;
#[jungle::act]
impl<const REST_TICK: u32> Act for PostMergeRest<REST_TICK> {
    type Effect = Rest<RHYTHM_GUITAR_LANE_ID, REST_TICK>;
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

pub struct RhythmRiffLoopRemaining;
impl LoopCondition<RhythmGuitaristState> for RhythmRiffLoopRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitaristState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseRhythmTurnaroundSection;
impl Condition<(RhythmGuitaristState, ())> for UseRhythmTurnaroundSection {
    fn choose((state, _): &(RhythmGuitaristState, ())) -> bool {
        state.riff_loops_remaining <= 1
    }
}

pub struct DecrementRhythmRiffLoop;
#[jungle::act]
impl Act for DecrementRhythmRiffLoop {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("rhythm riff loop decrement should complete");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

pub struct MergeRhythmTurnaroundChoice;
#[jungle::act]
impl Act for MergeRhythmTurnaroundChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(
        _state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("rhythm turnaround branch merge should complete");
    }
}

#[derive(Flow)]
pub struct RhythmRiffLoopNormalTail(
    Transparent<IntroSectionMeta, RhythmSection05>,
    Step<DecrementRhythmRiffLoop>,
);

#[derive(Flow)]
pub struct RhythmRiffLoopFinalTail(
    Transparent<IntroSectionMeta, RhythmSection06>,
    Step<DecrementRhythmRiffLoop>,
);

#[derive(Flow)]
pub struct RhythmRiffLoopBody(
    Transparent<IntroSectionMeta, RhythmSection03>,
    Transparent<IntroSectionMeta, RhythmSection04>,
    Conditional<UseRhythmTurnaroundSection, RhythmRiffLoopFinalTail, RhythmRiffLoopNormalTail>,
    Step<MergeRhythmTurnaroundChoice>,
);

#[derive(Flow)]
pub struct RhythmGuitarFlow(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, RhythmSection01>,
    Transparent<IntroSectionMeta, RhythmSection02>,
    While<RhythmRiffLoopRemaining, RhythmRiffLoopBody>,
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
pub struct RhythmSevenPick58(Quad<Pick58Tick>, Double<Pick58Tick>, Pick58Tick);

#[derive(Flow)]
pub struct RhythmTriplePick58(Double<Pick58Tick>, Pick58Tick);

#[derive(Flow)]
pub struct RhythmTriplePluck58(Double<Pluck58Tick>, Pluck58Tick);

#[derive(Flow)]
pub struct RhythmTriplePluck56(Double<Pluck56Tick>, Pluck56Tick);

#[derive(Flow)]
pub struct RhythmTriplePluck53(Double<Pluck53Tick>, Pluck53Tick);

#[derive(Flow)]
pub struct RhythmTriplePluck51(Double<Pluck51Tick>, Pluck51Tick);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart01(
    Pluck58Tick,
    Pick58Tick,
    Pluck58Tick,
    Transparent<IntroSectionMeta, RhythmSevenPick58>,
    Transparent<IntroSectionMeta, RhythmTriplePluck58>,
    Transparent<IntroSectionMeta, RhythmTriplePick58>,
    Pluck58Tick,
    Transparent<IntroSectionMeta, RhythmSevenPick58>,
);

#[derive(Flow)]
pub struct RhythmPart02Phrase(
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
pub struct RhythmPart02(RhythmPart02Phrase);

#[derive(Flow)]
pub struct RhythmPart03Phrase(
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
pub struct RhythmPart03(RhythmPart03Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart04(RhythmPart02Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart05(RhythmPart03Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart06(
    RhythmTriplePluck58,
    RhythmTriplePluck56,
    RhythmTriplePluck53,
    RhythmTriplePluck51,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Strum<44, 49, 46, 96, 96>>,
    RhythmTriplePluck58,
    RhythmTriplePluck56,
    Double<Pluck53Tick>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart07(
    Pluck53Tick,
    RhythmTriplePluck51,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 49, 46, 96, 96>>,
    Step<Strum<44, 49, 46, 96, 96>>,
    RhythmTriplePluck58,
    RhythmTriplePluck56,
    RhythmTriplePluck53,
    RhythmTriplePluck51,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
    Join<Step<JoinPluck<54, 47, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<42, 384, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPluck<56, 49, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<48, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<46, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinPick<44, 384, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
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
#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmJoinMonad100Flow(
    Join<Step<JoinPluck<54, 47, 100, 0>>, Step<HarmonySing<71, 100, 0>>>,
    Step<MergeUnit>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
    Step<JoinPick<46, 100, 0>>,
);

#[cfg(test)]
pub struct RhythmJoinMonad100Animal;

#[cfg(test)]
#[jungle::animal(id = 79, generation = 0)]
impl Animal for RhythmJoinMonad100Animal {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristState;
    type Journey = RhythmJoinMonad100Flow;
}

#[cfg(test)]
impl From<RhythmGuitaristState> for () {
    fn from(_value: RhythmGuitaristState) -> Self {}
}
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::*;
    use jungle_sdk::{JungleClient, LocalClient, RunnerUpdateOut};

    use super::super::RhythmGuitarist;
    use super::{RhythmGuitaristState, RhythmJoinMonad100Animal};
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
        failed_count: u32,
    }

    async fn collect_stream_stats(
        mut stream: jungle_sdk::client::JourneyUpdateSubscription,
        journey_id: uuid::Uuid,
    ) -> JourneyStreamStats {
        let mut total_events = 0_u32;
        let mut failed_count = 0_u32;
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
                RunnerUpdateOut::EffectInput { uuid, .. }
                | RunnerUpdateOut::EffectSuccessOutput { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
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
            failed_count,
        }
    }

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
        let worker_handle = tokio::spawn(async move { worker.spawn().await });

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

    #[tokio::test]
    async fn join_monad_100_ticks_zero_rest_with_tail_streams_events_and_completes_with_local_client() {
        const PARALLEL_JOURNEYS: usize = 5;

        let namespace = format!("welcome-rhythm-join-monad-100-test-{}", uuid::Uuid::new_v4());
        let client = LocalClient::builder()
            .namespace(&namespace)
            .build()
            .await
            .expect("local client should build");

        let audio_engine = crate::audio::AudioEngine::start_default()
            .await
            .expect("shared real audio engine should start");
        let shared_audio_handle = audio_engine.handle();
        let shared_metronome = crate::metronome::Metronome::spawn(123.0);
        shared_metronome.arm_start_barrier();

        let mut worker_handles = Vec::with_capacity(PARALLEL_JOURNEYS);
        for _ in 0..PARALLEL_JOURNEYS {
            let ecosystem =
                TheJungle::new_with_metronome(shared_audio_handle.clone(), 123.0, shared_metronome.clone());
            let worker = JungleWorker::new(ecosystem, client.clone());
            worker_handles.push(tokio::spawn(async move {
                let _ = worker.spawn().await;
            }));
        }

        let seed = postcard::to_allocvec(&RhythmGuitaristState::default()).expect("seed should serialize");
        let mut journey_ids = Vec::with_capacity(PARALLEL_JOURNEYS);
        for index in 0..PARALLEL_JOURNEYS {
            let journey_id = client
                .start_journey::<RhythmJoinMonad100Animal>(seed.clone())
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

        for (index, stream_task) in stream_tasks.into_iter().enumerate() {
            let stats = stream_task
                .await
                .unwrap_or_else(|err| panic!("stream task {index} should join cleanly: {err}"));
            assert!(
                stats.total_events >= 24,
                "journey {index} stream should emit at least 24 updates, got {}",
                stats.total_events,
            );
            assert_eq!(
                stats.failed_count, 0,
                "journey {index} should not have failed task transitions"
            );
        }

        let _ = release_task.await;
        for worker_handle in worker_handles {
            worker_handle.abort();
            let _ = worker_handle.await;
        }
    }
}
