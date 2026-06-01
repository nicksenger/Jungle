use jungle_sdk::prelude::*;

use crate::action::{MergeUnit as GenericMergeUnit, Rest as GenericRest};
use crate::effect::{Rest, Sound, SoundInput};
use crate::flow::loop2::Loop2;
use crate::instrumentation::{
    Bass as BassInstrument, BassArticulation, Thump as LaneThump, Vocals, VocalsArticulation,
};

use super::{Bass, BassistState, Double, Octa, Quad};

const BASS_LANE_ID: u8 = <<Bass as Animal>::Id as AnimalIdValue>::U32 as u8;
type Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneThump<NOTE, NOTE_TICK, REST_TICK, BASS_LANE_ID>;
type Thump44Tick = Step<Thump<44, 96, 96>>;
type Thump46Tick = Step<Thump<46, 96, 96>>;
type Thump39Tick = Step<Thump<39, 96, 96>>;
type Thump34Pedal = Step<Thump<34, 192, 192>>;
type MergeUnit = GenericMergeUnit<BassArticulation>;
type PostMergeRest<const TICKS: u32> = GenericRest<BassArticulation, TICKS, BASS_LANE_ID>;

const INTRO_START_DELAY_TICKS: u32 = 5_376;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct HarmonySing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Action
    for HarmonySing<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Sound<Vocals>;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: VocalsArticulation::GroupHarmony,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: BASS_LANE_ID,
        }
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        Ok(())
    }
}

pub struct BassRiffLoopRemaining;
impl Predicate<(&BassistState, &())> for BassRiffLoopRemaining {
    fn eval((state, _): &(&BassistState, &())) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseBassTurnaroundSection;
impl Predicate<(BassistState, ())> for UseBassTurnaroundSection {
    fn eval((state, _): &(BassistState, ())) -> bool {
        state.riff_loops_remaining <= 0
    }
}

pub struct DecrementBassRiffLoop;
#[jungle::action]
impl Action for DecrementBassRiffLoop {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(
        state: &mut BassistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            output.map_err(|_err| Failure::from("riff loop decrement should complete"))?;
            state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
        };
        Ok(__absorb_out_2)
    }
}

pub struct MergeBassTurnaroundChoice;
#[jungle::action]
impl Action for MergeBassTurnaroundChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(
        _state: &mut BassistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            output.map_err(|_err| Failure::from("bass turnaround branch merge should complete"))?;
        };
        Ok(__absorb_out_3)
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
    Transparent<
        IntroSectionMeta,
        Step<GenericRest<BassistState, INTRO_START_DELAY_TICKS, BASS_LANE_ID>>,
    >,
    Transparent<IntroSectionMeta, BassSection01>,
    While<BassRiffLoopRemaining, BassRiffLoopBody>,
    Transparent<IntroSectionMeta, BassSection06>,
    Transparent<IntroSectionMeta, BassSection07>,
    Transparent<IntroSectionMeta, BassSection08>,
);

#[derive(Flow)]
pub struct BassSection01(
    Transparent<
        IntroSectionMeta,
        //BassPart01,
        Loop2<BassArticulation, LoopedBassPart01Left, LoopedBassPart01Right>,
    >,
    Transparent<
        IntroSectionMeta,
        //BassPart02
        Loop2<BassArticulation, LoopedBassPart02Left, LoopedBassPart02Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart03Left, LoopedBassPart03Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart04Left, LoopedBassPart04Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart05Left, LoopedBassPart05Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart06Left, LoopedBassPart06Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection02(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart07Left, LoopedBassPart07Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart08Left, LoopedBassPart08Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart09Left, LoopedBassPart09Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart10Left, LoopedBassPart10Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart11Left, LoopedBassPart11Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart12Left, LoopedBassPart12Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection03(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart13Left, LoopedBassPart13Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart14Left, LoopedBassPart14Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart15Left, LoopedBassPart15Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart16Left, LoopedBassPart16Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart17Left, LoopedBassPart17Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart18Left, LoopedBassPart18Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection04(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart19Left, LoopedBassPart19Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart20Left, LoopedBassPart20Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart21Left, LoopedBassPart21Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart22Left, LoopedBassPart22Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart23Left, LoopedBassPart23Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart24Left, LoopedBassPart24Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection05(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart25Left, LoopedBassPart25Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart26Left, LoopedBassPart26Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart27Left, LoopedBassPart27Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart28Left, LoopedBassPart28Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart29Left, LoopedBassPart29Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart30Left, LoopedBassPart30Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection06(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart31Left, LoopedBassPart31Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart32Left, LoopedBassPart32Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart33Left, LoopedBassPart33Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart34Left, LoopedBassPart34Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart35Left, LoopedBassPart35Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart36Left, LoopedBassPart36Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection07(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart37Left, LoopedBassPart37Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart38Left, LoopedBassPart38Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart39Left, LoopedBassPart39Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart40Left, LoopedBassPart40Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart41Left, LoopedBassPart41Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart42Left, LoopedBassPart42Right>,
    >,
);

#[derive(Flow)]
pub struct BassSection08(
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart43Left, LoopedBassPart43Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart44Left, LoopedBassPart44Right>,
    >,
    Transparent<
        IntroSectionMeta,
        Loop2<BassArticulation, LoopedBassPart45Left, LoopedBassPart45Right>,
    >,
);

#[derive(Flow)]
pub struct BassPart01DriveTicks(
    Octa<Thump46Tick>,
    Quad<Thump46Tick>,
    Double<Thump46Tick>,
    Thump46Tick,
);

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
pub struct LoopedBassPart01Right(Transparent<IntroSectionMeta, BassPart01DriveTicks>);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart01Left(
    Step<Thump<46, 1536, 1536>>,
    Step<Thump<44, 1344, 1344>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<30, 1152, 1344>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 1152, 1344>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<45, 96, 96>>,
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
pub struct LoopedBassPart02Left(
    Step<Thump<46, 96, 96>>,
    Transparent<IntroSectionMeta, BassPart02HighTicks>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart02Right(
    Step<Thump<43, 96, 96>>,
    Transparent<IntroSectionMeta, BassPart02LowTicks>,
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
pub struct BassPart06Phrase(BassDriveCadenceLead, BassDriveCadenceLead, BassDriveExit);

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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart08(BassPart08Phrase);

#[derive(Flow)]
pub struct BassPart09Phrase(
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
    Transparent<IntroSectionMeta, BassDriveCadenceLead>,
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart22(
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
    Step<Thump<27, 127, 63>>,
    Step<Thump<27, 0, 64>>,
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart45(
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart03Left(
    Transparent<IntroSectionMeta, BassPart03LeadIn>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<37, 768, 768>>,
    Step<Thump<32, 768, 768>>,
    Transparent<IntroSectionMeta, BassPart03PedalTicks>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart03Right(
    Step<Thump<41, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<45, 192, 192>>,
    Step<Thump<46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart04Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart04Right(
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
pub struct LoopedBassPart05Left(
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart05Right(
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Step<Thump<42, 96, 192>>,
    Transparent<IntroSectionMeta, BassDriveCadenceLead>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart06Left(BassDriveCadenceLead, BassDriveCadenceLead);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart06Right(BassDriveExit);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart07Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart07Right(
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
pub struct LoopedBassPart08Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart08Right(
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart09Left(
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
pub struct LoopedBassPart09Right(
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
pub struct LoopedBassPart10Left(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart10Right(
    Step<Thump<39, 288, 288>>,
    Step<Thump<37, 288, 288>>,
    Step<Thump<33, 384, 384>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<31, 192, 192>>,
    Transparent<IntroSectionMeta, BassDriveCadenceLead>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart11Left(BassDriveCadenceLead, BassDriveCadenceLead);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart11Right(BassDriveExit);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart12Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart12Right(
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
pub struct LoopedBassPart13Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart13Right(
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart14Left(
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
pub struct LoopedBassPart14Right(
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
pub struct LoopedBassPart15Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart15Right(
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
pub struct LoopedBassPart16Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart16Right(
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
pub struct LoopedBassPart17Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart17Right(
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
pub struct LoopedBassPart18Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart18Right(
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
pub struct LoopedBassPart19Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart19Right(
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
pub struct LoopedBassPart20Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart20Right(
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
pub struct LoopedBassPart21Left(
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
pub struct LoopedBassPart21Right(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<39, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart22Left(
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart22Right(
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
pub struct LoopedBassPart23Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart23Right(
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
pub struct LoopedBassPart24Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart24Right(
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
pub struct LoopedBassPart25Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart25Right(
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
pub struct LoopedBassPart26Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart26Right(
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
pub struct LoopedBassPart27Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart27Right(
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
pub struct LoopedBassPart28Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart28Right(
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
pub struct LoopedBassPart29Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart29Right(
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
pub struct LoopedBassPart30Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart30Right(
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
pub struct LoopedBassPart31Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart31Right(
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<34, 384, 384>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<27, 64, 64>>,
    Step<Thump<27, 127, 63>>,
    Step<Thump<27, 0, 64>>,
    Step<Thump<34, 192, 384>>,
    Step<Thump<39, 384, 384>>,
    Step<Thump<29, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart32Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart32Right(
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
pub struct LoopedBassPart33Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart33Right(
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
pub struct LoopedBassPart34Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart34Right(
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
pub struct LoopedBassPart35Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart35Right(
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
pub struct LoopedBassPart36Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart36Right(
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
pub struct LoopedBassPart37Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart37Right(
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
pub struct LoopedBassPart38Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart38Right(
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
pub struct LoopedBassPart39Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart39Right(
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
pub struct LoopedBassPart40Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart40Right(
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
pub struct LoopedBassPart41Left(
    Step<Thump<33, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart41Right(
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
pub struct LoopedBassPart42Left(
    Step<Thump<39, 192, 192>>,
    Step<Thump<37, 192, 192>>,
    Step<Thump<33, 96, 96>>,
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart42Right(
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
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
pub struct LoopedBassPart43Left(
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
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart43Right(
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart44Left(
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart44Right(
    Step<Thump<33, 192, 192>>,
    Step<Thump<32, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<71, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<35, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart45Left(
    Join<Step<Thump<30, 192, 0>>, Step<HarmonySing<66, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<73, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<72, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 384, 0>>, Step<HarmonySing<70, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<384>>,
    Join<Step<Thump<37, 192, 0>>, Step<HarmonySing<68, 384, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct LoopedBassPart45Right(
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
#[jungle::action]
impl Action for BassTailStub {
    type Effect = BassTailStubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut BassistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            output.map_err(|_err| Failure::from("bass tail stub should succeed"))?;
        };
        Ok(__absorb_out_4)
    }
}

#[cfg(test)]
#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassJoinSound100JoinAndRest(
    Join<Step<Thump<35, 100, 0>>, Step<HarmonySing<71, 100, 0>>>,
    Step<MergeUnit>,
);

#[cfg(test)]
pub struct BassLoopDecrementStub;

#[cfg(test)]
#[jungle::action]
impl Action for BassLoopDecrementStub {
    type Effect = BassTailStubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(
        state: &mut BassistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
            output.map_err(|_err| Failure::from("test loop decrement should succeed"))?;
            state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
        };
        Ok(__absorb_out_5)
    }
}

#[cfg(test)]
#[derive(Flow)]
pub struct BassJoinSound100LoopBody(
    Transparent<IntroSectionMeta, BassJoinSound100JoinAndRest>,
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
    Step<BassLoopDecrementStub>,
);

#[cfg(test)]
#[derive(Flow)]
pub struct BassJoinSound100Flow(While<BassRiffLoopRemaining, BassJoinSound100LoopBody>);

#[cfg(test)]
pub struct BassJoinSound100Animal;

#[cfg(test)]
#[jungle::animal(id = 1, generation = 0)]
impl Animal for BassJoinSound100Animal {
    type State = BassistState;
    type Seed = BassistState;
    type Journey = BassJoinSound100Flow;
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
    use jungle_sdk::{JungleClient, FusedClient, RunnerUpdateOut};

    use super::super::Bass;
    use super::{BassJoinSound100Animal, BassistState};
    use crate::ecosystem::TheJungle;

    async fn await_completion(client: &FusedClient, journey_id: uuid::Uuid) {
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
        let client = FusedClient::builder()
            .namespace("welcome-bass-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
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
    async fn join_Sound_100_ticks_zero_rest_with_tail_streams_events_and_completes_with_local_client(
    ) {
        const PARALLEL_JOURNEYS: usize = 1;

        let namespace = format!("welcome-bass-join-Sound-100-test-{}", uuid::Uuid::new_v4());
        let client = FusedClient::builder()
            .namespace(&namespace)
            .build()
            .await
            .expect("local client should build");

        let (shared_audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
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

        let seed = postcard::to_allocvec(&BassistState::default()).expect("seed should serialize");
        let mut journey_ids = Vec::with_capacity(PARALLEL_JOURNEYS);
        for index in 0..PARALLEL_JOURNEYS {
            let journey_id = client
                .start_journey::<BassJoinSound100Animal>(seed.clone())
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
