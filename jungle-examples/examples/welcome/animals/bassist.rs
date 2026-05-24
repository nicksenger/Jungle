use jungle_sdk::prelude::*;

use crate::effect::{Monad, Rest};
use crate::instrumentation::{
    Bass as BassInstrument, BassArticulation, Thump as LaneThump, Vocals, VocalsArticulation,
};

use super::{Bass, Double, Octa, Quad};

const BASS_LANE_ID: u32 = <<Bass as Animal>::Id as AnimalIdValue>::U32;
type Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneThump<NOTE, NOTE_TICK, REST_TICK, BASS_LANE_ID>;
type Thump44Tick = Step<Thump<44, 96, 96>>;
type Thump46Tick = Step<Thump<46, 96, 96>>;
type Thump39Tick = Step<Thump<39, 96, 96>>;
type Thump34Pedal = Step<Thump<34, 192, 192>>;

pub struct JoinThump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for JoinThump<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<
        BassInstrument,
        BassArticulation,
        BASS_LANE_ID,
        NOTE,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join bass playback should succeed");
    }
}

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BassistState {
    #[jungle(focus)]
    articulation: BassArticulation,
    ostinato_loops_remaining: u8,
    riff_loops_remaining: u8,
}

impl Default for BassistState {
    fn default() -> Self {
        Self {
            articulation: BassArticulation::Picked,
            ostinato_loops_remaining: 1,
            riff_loops_remaining: 3,
        }
    }
}

pub type BassistSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 5_376;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<BASS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut BassistState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct HarmonySing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Act
    for HarmonySing<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Vocals, VocalsArticulation, BASS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        VocalsArticulation::GroupHarmony
    }

    fn absorb(
        _state: &mut BassArticulation,
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

    fn emit(_state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join merge should complete");
    }
}

pub struct PostMergeRest<const REST_TICK: u32>;
#[jungle::act]
impl<const REST_TICK: u32> Act for PostMergeRest<REST_TICK> {
    type Effect = Rest<BASS_LANE_ID, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("post-merge rest should complete");
    }
}

pub struct BassRiffLoopRemaining;
impl LoopCondition<BassistState> for BassRiffLoopRemaining {
    type Arg = ();

    fn should_continue(state: &BassistState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseBassTurnaroundSection;
impl Condition<(BassistState, ())> for UseBassTurnaroundSection {
    fn choose((state, _): &(BassistState, ())) -> bool {
        state.riff_loops_remaining <= 1
    }
}

pub struct DecrementBassRiffLoop;
#[jungle::act]
impl Act for DecrementBassRiffLoop {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(state: &mut BassistState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("riff loop decrement should complete");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

pub struct MergeBassTurnaroundChoice;
#[jungle::act]
impl Act for MergeBassTurnaroundChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(_state: &mut BassistState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("bass turnaround branch merge should complete");
    }
}

#[derive(Flow)]
pub struct BassRiffLoopNormalTail(
    Transparent<IntroSectionMeta, BassSection05>,
    Step<DecrementBassRiffLoop>,
);

#[derive(Flow)]
pub struct BassRiffLoopFinalTail(
    Transparent<IntroSectionMeta, BassSection06>,
    Step<DecrementBassRiffLoop>,
);

#[derive(Flow)]
pub struct BassRiffLoopBody(
    Transparent<IntroSectionMeta, BassSection02>,
    Transparent<IntroSectionMeta, BassSection03>,
    Transparent<IntroSectionMeta, BassSection04>,
    Conditional<UseBassTurnaroundSection, BassRiffLoopFinalTail, BassRiffLoopNormalTail>,
    Step<MergeBassTurnaroundChoice>,
);

#[derive(Flow)]
pub struct BassIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, BassSection01>,
    While<BassRiffLoopRemaining, BassRiffLoopBody>,
    Transparent<IntroSectionMeta, BassSection07>,
    Transparent<IntroSectionMeta, BassSection08>,
);

#[derive(Flow)]
pub struct BassSection01(
    Transparent<IntroSectionMeta, BassPart01>,
    Transparent<IntroSectionMeta, BassPart02>,
    Transparent<IntroSectionMeta, BassPart03>,
    Transparent<IntroSectionMeta, BassPart04>,
    Transparent<IntroSectionMeta, BassPart05>,
    Transparent<IntroSectionMeta, BassPart06>,
);

#[derive(Flow)]
pub struct BassSection02(
    Transparent<IntroSectionMeta, BassPart07>,
    Transparent<IntroSectionMeta, BassPart08>,
    Transparent<IntroSectionMeta, BassPart09>,
    Transparent<IntroSectionMeta, BassPart10>,
    Transparent<IntroSectionMeta, BassPart11>,
    Transparent<IntroSectionMeta, BassPart12>,
);

#[derive(Flow)]
pub struct BassSection03(
    Transparent<IntroSectionMeta, BassPart13>,
    Transparent<IntroSectionMeta, BassPart14>,
    Transparent<IntroSectionMeta, BassPart15>,
    Transparent<IntroSectionMeta, BassPart16>,
    Transparent<IntroSectionMeta, BassPart17>,
    Transparent<IntroSectionMeta, BassPart18>,
);

#[derive(Flow)]
pub struct BassSection04(
    Transparent<IntroSectionMeta, BassPart19>,
    Transparent<IntroSectionMeta, BassPart20>,
    Transparent<IntroSectionMeta, BassPart21>,
    Transparent<IntroSectionMeta, BassPart22>,
    Transparent<IntroSectionMeta, BassPart23>,
    Transparent<IntroSectionMeta, BassPart24>,
);

#[derive(Flow)]
pub struct BassSection05(
    Transparent<IntroSectionMeta, BassPart25>,
    Transparent<IntroSectionMeta, BassPart26>,
    Transparent<IntroSectionMeta, BassPart27>,
    Transparent<IntroSectionMeta, BassPart28>,
    Transparent<IntroSectionMeta, BassPart29>,
    Transparent<IntroSectionMeta, BassPart30>,
);

#[derive(Flow)]
pub struct BassSection06(
    Transparent<IntroSectionMeta, BassPart31>,
    Transparent<IntroSectionMeta, BassPart32>,
    Transparent<IntroSectionMeta, BassPart33>,
    Transparent<IntroSectionMeta, BassPart34>,
    Transparent<IntroSectionMeta, BassPart35>,
    Transparent<IntroSectionMeta, BassPart36>,
);

#[derive(Flow)]
pub struct BassSection07(
    Transparent<IntroSectionMeta, BassPart37>,
    Transparent<IntroSectionMeta, BassPart38>,
    Transparent<IntroSectionMeta, BassPart39>,
    Transparent<IntroSectionMeta, BassPart40>,
    Transparent<IntroSectionMeta, BassPart41>,
    Transparent<IntroSectionMeta, BassPart42>,
);

#[derive(Flow)]
pub struct BassSection08(
    Transparent<IntroSectionMeta, BassPart43>,
    Transparent<IntroSectionMeta, BassPart44>,
    Transparent<IntroSectionMeta, BassPart45>,
);

#[derive(Flow)]
pub struct BassPart01DriveTicks(Octa<Thump46Tick>, Quad<Thump46Tick>, Double<Thump46Tick>);

#[derive(Flow)]
pub struct BassPart02HighTicks(
    Octa<Thump44Tick>,
    Quad<Thump44Tick>,
    Double<Thump44Tick>,
    Thump44Tick,
);

#[derive(Flow)]
pub struct BassPart02LowTicks(Quad<Thump39Tick>, Double<Thump39Tick>, Thump39Tick);

#[derive(Flow)]
pub struct BassPart03LeadIn(Octa<Thump39Tick>);

#[derive(Flow)]
pub struct BassPart03PedalTicks(Octa<Thump34Pedal>, Thump34Pedal);

#[derive(Flow)]
pub struct BassDriveCadence(
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
pub struct BassDriveCadenceLead(
    Step<Thump<32, 192, 192>>,
    Transparent<IntroSectionMeta, BassDriveCadence>,
);

#[derive(Flow)]
pub struct BassDriveExit(
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart01(
    Step<Thump<46, 1536, 1536>>,
    Step<Thump<44, 1344, 1344>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<30, 1152, 1344>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 1152, 1344>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<45, 96, 96>>,
    Transparent<IntroSectionMeta, BassPart01DriveTicks>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart02(
    Step<Thump<46, 96, 96>>,
    Transparent<IntroSectionMeta, BassPart02HighTicks>,
    Step<Thump<43, 96, 96>>,
    Transparent<IntroSectionMeta, BassPart02LowTicks>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart03(
    Transparent<IntroSectionMeta, BassPart03LeadIn>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<37, 768, 768>>,
    Step<Thump<32, 768, 768>>,
    Transparent<IntroSectionMeta, BassPart03PedalTicks>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<45, 192, 192>>,
    Step<Thump<46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart04(
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart05(
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Transparent<IntroSectionMeta, BassDriveCadenceLead>,
);

#[derive(Flow)]
pub struct BassPart06Phrase(
    BassDriveCadenceLead,
    BassDriveCadenceLead,
    BassDriveExit,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart06(BassPart06Phrase);

#[derive(Flow)]
pub struct BassPart07Phrase(
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart07(BassPart07Phrase);

#[derive(Flow)]
pub struct BassPart08Phrase(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart08(BassPart08Phrase);

#[derive(Flow)]
pub struct BassPart09Phrase(
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart09(BassPart09Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart10(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 288, 288>>,
    Step<Thump<37, 288, 288>>,
    Step<Thump<33, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<31, 192, 192>>,
    Transparent<IntroSectionMeta, BassDriveCadence>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart11(BassPart06Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart12(BassPart07Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart13(BassPart08Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart14(BassPart09Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart15(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<34, 288, 288>>,
    Step<Thump<34, 288, 288>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<44, 384, 384>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart16(
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart17(
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<42, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<42, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart18(
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart19(
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart20(
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart21(
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart22(
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart23(
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 288, 288>>,
    Step<Thump<37, 288, 288>>,
    Step<Thump<33, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<37, 1536, 1536>>,
    Step<Thump<30, 1536, 1536>>,
    Step<Thump<37, 1536, 1536>>,
    Step<Thump<30, 1152, 1344>>,
    Step<Thump<35, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart24(
    Step<Thump<36, 96, 96>>,
    Step<Thump<37, 1152, 1344>>,
    Step<Thump<40, 96, 96>>,
    Step<Thump<41, 96, 96>>,
    Step<Thump<42, 960, 960>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<35, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 384, 384>>,
    Step<Thump<37, 288, 288>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<35, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 384>>,
    Step<Thump<28, 384, 384>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart25(
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 384>>,
    Step<Thump<28, 384, 384>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<30, 192, 384>>,
    Step<Thump<28, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart26(
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart27(
    Step<Thump<36, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 96, 192>>,
    Step<Thump<30, 384, 384>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart28(
    Step<Thump<27, 96, 96>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart29(
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 384, 384>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart30(
    Step<Thump<33, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<46, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart31(
    Step<Thump<39, 384, 384>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 64, 64>>,
    Step<Thump<27, 63, 63>>,
    Step<Thump<27, 64, 64>>,
    Step<Thump<34, 192, 384>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<29, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart32(
    Step<Thump<29, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<40, 96, 96>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<36, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 1152, 1152>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<46, 1536, 1536>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<29, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart33(
    Step<Thump<29, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<29, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<31, 96, 96>>,
    Step<Thump<31, 96, 96>>,
    Step<Thump<31, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<30, 96, 96>>,
    Step<Thump<31, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart34(
    Step<Thump<31, 96, 96>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
);

#[derive(Flow)]
pub struct BassPart35Phrase(
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<35, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart35(BassPart35Phrase);

#[derive(Flow)]
pub struct BassPart36Phrase(
    Step<Thump<35, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<35, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart36(BassPart36Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart37(BassPart35Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart38(BassPart36Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart39(BassPart35Phrase);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart40(
    Step<Thump<35, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<36, 96, 96>>,
    Step<Thump<35, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<29, 384, 384>>,
    Step<Thump<28, 384, 384>>,
    Step<Thump<29, 384, 384>>,
    Step<Thump<30, 384, 384>>,
    Step<Thump<32, 384, 384>>,
    Step<Thump<31, 384, 384>>,
    Step<Thump<32, 384, 384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart41(
    Step<Thump<33, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart42(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart43(
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart44(
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart45(
    Join<Step<JoinThump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<JoinThump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 288, 288>>,
    Step<Thump<37, 288, 288>>,
    Step<Thump<33, 288, 288>>,
    Step<Thump<32, 288, 288>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 288, 288>>,
    Step<Thump<30, 288, 288>>,
    Step<Thump<27, 192, 576>>,
    Step<Thump<27, 3456, 0>>,
);

#[cfg(test)]
pub struct BassTailStubEffect;

#[cfg(test)]
#[jungle::effect(id = 962)]
impl<J> Effect<J> for BassTailStubEffect {
    type In = ();
    type Out = ();
    type Err = ();

    #[allow(clippy::manual_async_fn)]
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(()) }
    }
}

#[cfg(test)]
pub struct BassTailStub;

#[cfg(test)]
#[jungle::act]
impl Act for BassTailStub {
    type Effect = BassTailStubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut BassArticulation, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("bass tail stub should succeed");
    }
}

#[cfg(test)]
#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassJoinMonad100Flow(
    Join<Step<JoinThump<35, 100, 0>>, Step<HarmonySing<71, 100, 0>>>,
    Step<MergeUnit>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
    Step<BassTailStub>,
);

#[cfg(test)]
pub struct BassJoinMonad100Animal;

#[cfg(test)]
#[jungle::animal(id = 78, generation = 0)]
impl Animal for BassJoinMonad100Animal {
    type State = BassistState;
    type Seed = BassistState;
    type Journey = BassJoinMonad100Flow;
}

#[cfg(test)]
impl From<BassistState> for () {
    fn from(_value: BassistState) -> Self {}
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::*;
    use jungle_sdk::{JungleClient, LocalClient, RunnerUpdateOut};

    use super::super::Bass;
    use super::{BassJoinMonad100Animal, BassistState};
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
            .namespace("welcome-bass-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move { worker.spawn().await });

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<Bass>(seed)
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

        let namespace = format!("welcome-bass-join-monad-100-test-{}", uuid::Uuid::new_v4());
        let client = LocalClient::builder()
            .namespace(&namespace)
            .build()
            .await
            .expect("local client should build");

        let (shared_audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
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

        let seed = postcard::to_allocvec(&BassistState::default()).expect("seed should serialize");
        let mut journey_ids = Vec::with_capacity(PARALLEL_JOURNEYS);
        for index in 0..PARALLEL_JOURNEYS {
            let journey_id = client
                .start_journey::<BassJoinMonad100Animal>(seed.clone())
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
                stats.total_events > 10,
                "journey {index} stream should emit more than 10 updates, got {}",
                stats.total_events,
            );
        }

        let _ = release_task.await;
        for worker_handle in worker_handles {
            worker_handle.abort();
            let _ = worker_handle.await;
        }
    }
}
