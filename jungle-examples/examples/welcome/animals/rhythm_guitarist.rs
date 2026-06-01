use jungle_sdk::prelude::*;

use crate::action::{MergeEither, MergeUnit as GenericMergeUnit, Rest as GenericRest};
use crate::effect::Rest;
use crate::instrumentation::{ElectricGuitarArticulation, Pick as LanePick, Pluck as LanePluck};

use super::{Double, RhythmGuitarist, RhythmGuitaristState};

const LEAD_GUITAR_LANE_ID: u8 = <<RhythmGuitarist as Animal>::Id as AnimalIdValue>::U32 as u8;
type Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePick<NOTE, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;
type Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;
type Pick44Tick = Step<Pick<44, 96, 96>>;
type Pick39Tick = Step<Pick<39, 96, 96>>;
type Pluck4451Hold = Pluck<44, 51, 192, 192>;
type MergeUnit = GenericMergeUnit<ElectricGuitarArticulation>;
type PostMergeRest<const TICKS: u32> =
    GenericRest<ElectricGuitarArticulation, TICKS, LEAD_GUITAR_LANE_ID>;

const INTRO_START_DELAY_TICKS: u32 = 5_376;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

#[derive(Flow)]
pub struct SplitPluck<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<Pick<NOTE_1, NOTE_TICK_1, 0>>, Step<Pick<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct TriadHitPair<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0>>, Step<Pick<NOTE_2, NOTE_TICK, 0>>>,
    Step<MergeUnit>,
);

#[derive(Flow)]
pub struct TriadHit<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(
    Join<TriadHitPair<NOTE_1, NOTE_2, NOTE_TICK>, Step<Pick<NOTE_3, NOTE_TICK, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct QuadHit<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(
    Join<TriadHitPair<NOTE_1, NOTE_2, NOTE_TICK>, TriadHitPair<NOTE_3, NOTE_4, NOTE_TICK>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

pub struct RhythmRiffLoopRemaining;
impl Predicate<(&RhythmGuitaristState, &())> for RhythmRiffLoopRemaining {
    fn eval((state, _): &(&RhythmGuitaristState, &())) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseRhythmTurnaroundSection;
impl Predicate<(RhythmGuitaristState, ())> for UseRhythmTurnaroundSection {
    fn eval((state, _): &(RhythmGuitaristState, ())) -> bool {
        state.riff_loops_remaining <= 0
    }
}

pub struct DecrementRhythmRiffLoop;
#[jungle::action]
impl Action for DecrementRhythmRiffLoop {
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
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("lead riff loop decrement should complete"))?;
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
        Ok(())
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
    Transparent<IntroSectionMeta, RhythmSection02>,
    Transparent<IntroSectionMeta, RhythmSection03>,
    Transparent<IntroSectionMeta, RhythmSection04>,
    Conditional<UseRhythmTurnaroundSection, RhythmRiffLoopFinalTail, RhythmRiffLoopNormalTail>,
    Step<MergeEither<(), RhythmGuitaristState>>,
);

#[derive(Flow)]
pub struct RhythmGuitarIntro(
    Transparent<
        IntroSectionMeta,
        Step<GenericRest<RhythmGuitaristState, INTRO_START_DELAY_TICKS, LEAD_GUITAR_LANE_ID>>,
    >,
    Transparent<IntroSectionMeta, RhythmSection01>,
    While<RhythmRiffLoopRemaining, RhythmRiffLoopBody>,
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
pub struct RhythmSection07(Transparent<IntroSectionMeta, RhythmPart37>);

#[derive(Flow)]
pub struct RhythmPick44Pair(Double<Pick44Tick>);

#[derive(Flow)]
pub struct RhythmPick39Pair(Double<Pick39Tick>);

#[derive(Flow)]
pub struct RhythmPluck4451Pair(Double<Pluck4451Hold>);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart01(
    Pluck<46, 53, 1536, 1152>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 288, 288>>,
    Step<Pick<61, 672, 672>>,
    Step<Pick<61, 192, 576>>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<58, 1056, 1248>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<51, 1536, 1344>>,
    Step<Pick<53, 192, 192>>,
    Pluck<58, 65, 1152, 1344>,
    Pluck<58, 65, 192, 192>,
    Pluck<44, 51, 1536, 1344>,
    Step<Pick<56, 192, 192>>,
    Pluck<39, 46, 1536, 1536>,
    Pluck<49, 56, 768, 768>,
    Pluck<44, 51, 768, 768>,
    Step<Pick<46, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<53, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pick<58, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart02(
    Step<Pick<61, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<68, 192, 192>>,
    Pluck<63, 68, 384, 384>,
    Step<Pick<61, 384, 384>>,
    Step<Pick<58, 384, 384>>,
    RhythmPluck4451Pair,
    Pluck<42, 49, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<42, 49, 192, 192>,
    Pluck<41, 48, 192, 192>,
    Pluck<39, 46, 192, 192>,
    RhythmPluck4451Pair,
    Pluck<42, 49, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<42, 49, 192, 192>,
    Pluck<41, 48, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart03(
    Pluck<39, 46, 192, 192>,
    RhythmPluck4451Pair,
    Pluck<42, 49, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 96, 96>,
    Pluck<42, 49, 192, 192>,
    Pluck<41, 48, 192, 192>,
    Pluck<39, 46, 192, 192>,
    Pluck<61, 73, 192, 192>,
    Step<Pick<59, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<61, 96, 96>>,
    Step<Pick<59, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<54, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart04(
    Step<Pick<42, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 288, 288>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<41, 192, 192>>,
    RhythmPick39Pair,
    RhythmPick44Pair,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    RhythmPick39Pair,
    RhythmPick44Pair,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart05(
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    RhythmPick39Pair,
    RhythmPick44Pair,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Pluck<49, 56, 192, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart06(
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<49, 56, 96, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart07(
    Pluck<44, 49, 96, 96>,
    SplitPluck<51, 58, 96, 192, 192>,
    Pluck<51, 58, 96, 96>,
    SplitPluck<49, 56, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    Step<Pick<44, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart08(
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 288, 288>>,
    Step<Pick<49, 288, 288>>,
    Step<Pick<45, 384, 384>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<43, 192, 192>>,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart09(
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 288, 288>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<41, 192, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart10(
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Pluck<49, 56, 192, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart11(
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<49, 56, 96, 192, 192>,
    Pluck<44, 49, 96, 96>,
    SplitPluck<51, 58, 96, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart12(
    Pluck<51, 58, 96, 96>,
    SplitPluck<49, 56, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    Step<Pick<44, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<51, 96, 192>>,
    Step<Pick<49, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart13(
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
    Pluck<63, 68, 96, 96>,
    Pluck<65, 70, 96, 96>,
    Pluck<63, 68, 96, 96>,
    Pluck<65, 70, 288, 288>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<65, 480, 480>>,
    Pluck<54, 58, 96, 384>,
    Step<Pick<39, 384, 192>>,
    Pluck<60, 66, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart14(
    Pluck<39, 51, 192, 192>,
    SplitPluck<39, 56, 192, 96, 96>,
    Step<Pick<58, 96, 96>>,
    SplitPluck<39, 61, 192, 96, 96>,
    Step<Pick<63, 96, 96>>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Pluck<54, 66, 192, 192>,
    Pluck<49, 63, 192, 192>,
    TriadHit<39, 49, 54, 96, 96>,
    Step<Pick<46, 96, 96>>,
    Pluck<49, 54, 96, 192>,
    Pluck<70, 73, 576, 384>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<51, 96, 288>>,
    Pluck<60, 66, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Pluck<49, 56, 96, 96>,
    Pluck<51, 58, 96, 288>,
    Pluck<51, 58, 192, 384>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<58, 192, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart15(
    Step<Pick<70, 96, 96>>,
    Pluck<69, 72, 192, 192>,
    Pluck<70, 73, 384, 192>,
    Pluck<58, 63, 96, 96>,
    Pluck<60, 66, 192, 192>,
    Step<Pick<63, 96, 96>>,
    Pluck<73, 78, 192, 192>,
    Pluck<73, 78, 192, 192>,
    Step<Pick<60, 192, 192>>,
    Step<Pick<72, 128, 128>>,
    Step<Pick<75, 129, 129>>,
    Step<Pick<82, 896, 896>>,
    Step<Pick<82, 128, 128>>,
    Step<Pick<81, 129, 129>>,
    Step<Pick<80, 704, 704>>,
    Step<Pick<78, 96, 96>>,
    Step<Pick<79, 96, 96>>,
    Step<Pick<73, 672, 672>>,
    Step<Pick<73, 224, 224>>,
    Step<Pick<73, 129, 129>>,
    Step<Pick<70, 128, 128>>,
    Step<Pick<68, 192, 192>>,
    Step<Pick<68, 288, 288>>,
    Step<Pick<66, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart16(
    Step<Pick<68, 96, 96>>,
    Step<Pick<66, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<57, 96, 96>>,
    Step<Pick<57, 96, 96>>,
    SplitPluck<56, 56, 192, 288, 192>,
    Step<Pick<62, 192, 96>>,
    Step<Pick<54, 96, 96>>,
    TriadHit<51, 51, 63, 192, 192>,
    Step<Pick<62, 576, 576>>,
    Step<Pick<60, 384, 384>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 288, 288>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<41, 192, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart17(
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart18(
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    TriadHit<49, 56, 61, 192, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    TriadHit<44, 49, 54, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    Pluck<51, 58, 96, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart19(
    Pluck<51, 58, 96, 192>,
    Pluck<49, 56, 96, 192>,
    Pluck<44, 49, 96, 96>,
    Pluck<51, 58, 96, 192>,
    Pluck<51, 58, 96, 96>,
    Pluck<49, 56, 96, 192>,
    Pluck<48, 55, 96, 192>,
    Pluck<46, 53, 96, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    SplitPluck<49, 56, 96, 192, 192>,
    Pluck<44, 49, 96, 96>,
    SplitPluck<51, 58, 96, 192, 192>,
    Pluck<51, 58, 96, 96>,
    SplitPluck<49, 56, 96, 192, 192>,
    SplitPluck<51, 58, 96, 192, 192>,
    Step<Pick<44, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart20(
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart21(
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 288, 288>>,
    Step<Pick<49, 288, 288>>,
    Step<Pick<45, 384, 384>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<43, 96, 192>>,
    Pluck<44, 49, 1536, 1536>,
    Pluck<61, 66, 1536, 1536>,
    Pluck<61, 65, 1536, 1536>,
    Pluck<42, 46, 1536, 1536>,
    Pluck<44, 49, 1536, 1536>,
    Pluck<42, 46, 1536, 1536>,
    Pluck<44, 49, 1152, 1152>,
    TriadHit<49, 56, 59, 384, 384>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 384, 384>,
    Pluck<40, 64, 384, 384>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart22(
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    SplitPluck<42, 49, 96, 192, 96>,
    Step<Pick<42, 96, 96>>,
    Pluck<42, 49, 384, 384>,
    Pluck<40, 64, 384, 384>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 192, 192>,
    Pluck<42, 49, 384, 384>,
    Pluck<40, 64, 384, 384>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<44, 51, 192, 192>,
    Pluck<42, 66, 192, 192>,
    SplitPluck<39, 73, 192, 960, 576>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart23(
    Step<Pick<73, 288, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<73, 288, 288>>,
    Step<Pick<73, 960, 576>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<70, 192, 192>>,
    Step<Pick<68, 192, 192>>,
    Step<Pick<73, 192, 192>>,
    SplitPluck<39, 68, 192, 384, 384>,
    Step<Pick<70, 192, 192>>,
    SplitPluck<39, 75, 96, 192, 192>,
    SplitPluck<39, 73, 96, 192, 192>,
    Step<Pick<68, 384, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<66, 192, 192>>,
    Step<Pick<73, 384, 384>>,
    Step<Pick<78, 192, 192>>,
    SplitPluck<39, 73, 96, 960, 192>,
    Step<Pick<39, 96, 768>>,
    Step<Pick<48, 768, 768>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<55, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart24(
    Step<Pick<53, 576, 768>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pick<55, 96, 96>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<60, 960, 960>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<60, 192, 192>>,
    Step<Pick<67, 384, 384>>,
    Step<Pick<65, 768, 768>>,
    Step<Pick<65, 192, 192>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<60, 96, 96>>,
    Step<Pick<63, 192, 192>>,
    Pluck<67, 72, 960, 960>,
    Pluck<67, 70, 192, 192>,
    Step<Pick<65, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<60, 192, 192>>,
    Step<Pick<58, 960, 960>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<60, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart25(
    Step<Pick<63, 192, 192>>,
    Step<Pick<60, 192, 192>>,
    Pluck<63, 63, 576, 576>,
    Pluck<61, 63, 384, 384>,
    Pluck<60, 63, 384, 384>,
    Pluck<58, 63, 384, 384>,
    Pluck<56, 60, 384, 384>,
    Pluck<56, 60, 192, 192>,
    Pluck<54, 58, 192, 192>,
    Step<Pick<55, 192, 192>>,
    Pluck<55, 58, 192, 192>,
    Step<Pick<56, 192, 192>>,
    QuadHit<53, 58, 58, 58, 384, 384>,
    Pluck<58, 58, 192, 192>,
    Pluck<56, 58, 192, 192>,
    Pluck<56, 58, 192, 192>,
    Pluck<55, 58, 384, 384>,
    Pluck<56, 60, 192, 192>,
    Pluck<58, 62, 192, 192>,
    Step<Pick<51, 288, 288>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<46, 96, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart26(
    Step<Pick<75, 192, 192>>,
    Step<Pick<75, 576, 576>>,
    Step<Pick<73, 384, 384>>,
    Step<Pick<70, 384, 384>>,
    Step<Pick<68, 384, 384>>,
    Step<Pick<68, 192, 192>>,
    Step<Pick<66, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<61, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Pluck<63, 72, 96, 96>,
    Pluck<61, 70, 480, 672>,
    Step<Pick<60, 192, 192>>,
    Step<Pick<61, 384, 384>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<56, 576, 576>>,
    Step<Pick<53, 192, 192>>,
    Step<Pick<49, 576, 576>>,
    Step<Pick<46, 192, 192>>,
    Step<Pick<43, 576, 576>>,
    Step<Pick<39, 384, 384>>,
    Step<Pick<41, 3072, 3072>>,
    Pluck<58, 62, 1536, 1536>,
    Pluck<58, 62, 1536, 1536>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart27(
    Pluck<49, 54, 96, 2880>,
    Step<Pick<85, 768, 768>>,
    Step<Pick<82, 192, 192>>,
    Step<Pick<82, 192, 192>>,
    Step<Pick<81, 576, 576>>,
    Step<Pick<79, 384, 384>>,
    Pluck<44, 79, 96, 48>,
    Step<Pick<49, 96, 48>>,
    SplitPluck<61, 77, 672, 288, 288>,
    Step<Pick<87, 768, 384>>,
    Step<Pick<61, 384, 384>>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart28(
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
);

#[derive(Flow)]
pub struct RhythmPart29Phrase(
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart29(RhythmPart29Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart30(
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<47, 53, 96, 96>,
    Pluck<47, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart31(RhythmPart29Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart32(
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<51, 58, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<50, 57, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<49, 56, 96, 96>,
    Pluck<48, 55, 96, 96>,
    Pluck<48, 55, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<46, 53, 96, 96>,
    Pluck<41, 48, 384, 384>,
    Pluck<40, 47, 384, 384>,
    Pluck<41, 48, 384, 384>,
    Pluck<42, 49, 384, 384>,
    Pluck<44, 51, 384, 384>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart33(
    Pluck<43, 50, 384, 384>,
    Pluck<44, 51, 384, 384>,
    Pluck<45, 52, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart34(
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart35(
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<51, 58, 192, 192>,
    Pluck<51, 58, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart36(
    Step<Pick<39, 96, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<47, 54, 384, 384>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Pluck<49, 56, 384, 384>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Pluck<58, 63, 288, 288>,
    Pluck<56, 61, 288, 288>,
    Pluck<52, 57, 288, 288>,
    Pluck<51, 56, 288, 288>,
    Pluck<49, 54, 192, 384>,
    Pluck<56, 60, 288, 288>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct RhythmPart37(
    Pluck<54, 58, 288, 288>,
    Pluck<51, 55, 192, 576>,
    Pluck<61, 66, 3456, 3456>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{FusedClient, JungleClient};

    use super::super::RhythmGuitarist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = FusedClient::builder()
            .namespace("welcome-lead-guitar-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = ();
        let journey_id = client
            .spawn::<RhythmGuitarist>(&seed)
            .await
            .expect("journey should start")
            .journey_id;

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
