use jungle_sdk::prelude::*;

use crate::effect::{AtomicDualHit, Rest, Tetrad};
use crate::instrumentation::{
    ElectricGuitar, ElectricGuitarArticulation, Pick as LanePick, Pluck as LanePluck,
};

use super::{Double, LeadGuitarist};

const LEAD_GUITAR_LANE_ID: u32 = <<LeadGuitarist as Animal>::Id as AnimalIdValue>::U32;
type Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePick<NOTE, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;
type Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;
type Pick44Tick = Step<Pick<44, 96, 96>>;
type Pick39Tick = Step<Pick<39, 96, 96>>;
type Pluck4451Hold = Step<Pluck<44, 51, 192, 192>>;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LeadGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
}

impl Default for LeadGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::Sustained,
            riff_loops_remaining: 1,
        }
    }
}

pub type LeadGuitaristSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 5_376;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<LEAD_GUITAR_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &LeadGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut LeadGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
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
        LEAD_GUITAR_LANE_ID,
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
        output.expect("split pluck playback should succeed");
    }
}

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

pub struct QuadHit<
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
    > Act for QuadHit<NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_TICK, REST_TICK>
{
    type Effect = Tetrad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        LEAD_GUITAR_LANE_ID,
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
        output.expect("quad playback should succeed");
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
    type Effect = Rest<LEAD_GUITAR_LANE_ID, REST_TICK>;
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

pub struct LeadRiffLoopRemaining;
impl LoopCondition<LeadGuitaristState> for LeadRiffLoopRemaining {
    type Arg = ();

    fn should_continue(state: &LeadGuitaristState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseLeadTurnaroundSection;
impl Condition<(LeadGuitaristState, ())> for UseLeadTurnaroundSection {
    fn choose((state, _): &(LeadGuitaristState, ())) -> bool {
        state.riff_loops_remaining <= 0
    }
}

pub struct DecrementLeadRiffLoop;
#[jungle::act]
impl Act for DecrementLeadRiffLoop {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &LeadGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        state: &mut LeadGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead riff loop decrement should complete");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

pub struct MergeLeadTurnaroundChoice;
#[jungle::act]
impl Act for MergeLeadTurnaroundChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(
        _state: &LeadGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        _state: &mut LeadGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead turnaround branch merge should complete");
    }
}

#[derive(Flow)]
pub struct LeadRiffLoopNormalTail(
    Transparent<IntroSectionMeta, LeadSection05>,
    Step<DecrementLeadRiffLoop>,
);

#[derive(Flow)]
pub struct LeadRiffLoopFinalTail(
    Transparent<IntroSectionMeta, LeadSection06>,
    Step<DecrementLeadRiffLoop>,
);

#[derive(Flow)]
pub struct LeadRiffLoopBody(
    Transparent<IntroSectionMeta, LeadSection02>,
    Transparent<IntroSectionMeta, LeadSection03>,
    Transparent<IntroSectionMeta, LeadSection04>,
    Conditional<UseLeadTurnaroundSection, LeadRiffLoopFinalTail, LeadRiffLoopNormalTail>,
    Step<MergeLeadTurnaroundChoice>,
);

#[derive(Flow)]
pub struct LeadGuitarIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, LeadSection01>,
    While<LeadRiffLoopRemaining, LeadRiffLoopBody>,
    Transparent<IntroSectionMeta, LeadSection06>,
    Transparent<IntroSectionMeta, LeadSection07>,
);

#[derive(Flow)]
pub struct LeadSection01(
    Transparent<IntroSectionMeta, LeadPart01>,
    Transparent<IntroSectionMeta, LeadPart02>,
    Transparent<IntroSectionMeta, LeadPart03>,
    Transparent<IntroSectionMeta, LeadPart04>,
    Transparent<IntroSectionMeta, LeadPart05>,
    Transparent<IntroSectionMeta, LeadPart06>,
);

#[derive(Flow)]
pub struct LeadSection02(
    Transparent<IntroSectionMeta, LeadPart07>,
    Transparent<IntroSectionMeta, LeadPart08>,
    Transparent<IntroSectionMeta, LeadPart09>,
    Transparent<IntroSectionMeta, LeadPart10>,
    Transparent<IntroSectionMeta, LeadPart11>,
    Transparent<IntroSectionMeta, LeadPart12>,
);

#[derive(Flow)]
pub struct LeadSection03(
    Transparent<IntroSectionMeta, LeadPart13>,
    Transparent<IntroSectionMeta, LeadPart14>,
    Transparent<IntroSectionMeta, LeadPart15>,
    Transparent<IntroSectionMeta, LeadPart16>,
    Transparent<IntroSectionMeta, LeadPart17>,
    Transparent<IntroSectionMeta, LeadPart18>,
);

#[derive(Flow)]
pub struct LeadSection04(
    Transparent<IntroSectionMeta, LeadPart19>,
    Transparent<IntroSectionMeta, LeadPart20>,
    Transparent<IntroSectionMeta, LeadPart21>,
    Transparent<IntroSectionMeta, LeadPart22>,
    Transparent<IntroSectionMeta, LeadPart23>,
    Transparent<IntroSectionMeta, LeadPart24>,
);

#[derive(Flow)]
pub struct LeadSection05(
    Transparent<IntroSectionMeta, LeadPart25>,
    Transparent<IntroSectionMeta, LeadPart26>,
    Transparent<IntroSectionMeta, LeadPart27>,
    Transparent<IntroSectionMeta, LeadPart28>,
    Transparent<IntroSectionMeta, LeadPart29>,
    Transparent<IntroSectionMeta, LeadPart30>,
);

#[derive(Flow)]
pub struct LeadSection06(
    Transparent<IntroSectionMeta, LeadPart31>,
    Transparent<IntroSectionMeta, LeadPart32>,
    Transparent<IntroSectionMeta, LeadPart33>,
    Transparent<IntroSectionMeta, LeadPart34>,
    Transparent<IntroSectionMeta, LeadPart35>,
    Transparent<IntroSectionMeta, LeadPart36>,
);

#[derive(Flow)]
pub struct LeadSection07(Transparent<IntroSectionMeta, LeadPart37>);

#[derive(Flow)]
pub struct LeadPick44Pair(Double<Pick44Tick>);

#[derive(Flow)]
pub struct LeadPick39Pair(Double<Pick39Tick>);

#[derive(Flow)]
pub struct LeadPluck4451Pair(Double<Pluck4451Hold>);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart01(
    Step<Pluck<46, 53, 1536, 1152>>,
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
    Step<Pluck<58, 65, 1152, 1344>>,
    Step<Pluck<58, 65, 192, 192>>,
    Step<Pluck<44, 51, 1536, 1344>>,
    Step<Pick<56, 192, 192>>,
    Step<Pluck<39, 46, 1536, 1536>>,
    Step<Pluck<49, 56, 768, 768>>,
    Step<Pluck<44, 51, 768, 768>>,
    Step<Pick<46, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<53, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pick<58, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart02(
    Step<Pick<61, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<68, 192, 192>>,
    Step<Pluck<63, 68, 384, 384>>,
    Step<Pick<61, 384, 384>>,
    Step<Pick<58, 384, 384>>,
    LeadPluck4451Pair,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<41, 48, 192, 192>>,
    Step<Pluck<39, 46, 192, 192>>,
    LeadPluck4451Pair,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<41, 48, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart03(
    Step<Pluck<39, 46, 192, 192>>,
    LeadPluck4451Pair,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<41, 48, 192, 192>>,
    Step<Pluck<39, 46, 192, 192>>,
    Step<Pluck<61, 73, 192, 192>>,
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
pub struct LeadPart04(
    Step<Pick<42, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 288, 288>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<41, 192, 192>>,
    LeadPick39Pair,
    LeadPick44Pair,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    LeadPick39Pair,
    LeadPick44Pair,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart05(
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    LeadPick39Pair,
    LeadPick44Pair,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart06(
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart07(
    Step<Pluck<44, 49, 96, 96>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
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
pub struct LeadPart08(
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
pub struct LeadPart09(
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
pub struct LeadPart10(
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
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart11(
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart12(
    Step<Pluck<51, 58, 96, 96>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
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
pub struct LeadPart13(
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
    Step<Pluck<63, 68, 96, 96>>,
    Step<Pluck<65, 70, 96, 96>>,
    Step<Pluck<63, 68, 96, 96>>,
    Step<Pluck<65, 70, 288, 288>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<65, 480, 480>>,
    Step<Pluck<54, 58, 96, 384>>,
    Step<Pick<39, 384, 192>>,
    Step<Pluck<60, 66, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart14(
    Step<Pluck<39, 51, 192, 192>>,
    Step<SplitPluck<39, 56, 192, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<SplitPluck<39, 61, 192, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<54, 66, 192, 192>>,
    Step<Pluck<49, 63, 192, 192>>,
    TriadHit<39, 49, 54, 96, 96>,
    Step<Pick<46, 96, 96>>,
    Step<Pluck<49, 54, 96, 192>>,
    Step<Pluck<70, 73, 576, 384>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<51, 96, 288>>,
    Step<Pluck<60, 66, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<51, 58, 96, 288>>,
    Step<Pluck<51, 58, 192, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pick<58, 192, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart15(
    Step<Pick<70, 96, 96>>,
    Step<Pluck<69, 72, 192, 192>>,
    Step<Pluck<70, 73, 384, 192>>,
    Step<Pluck<58, 63, 96, 96>>,
    Step<Pluck<60, 66, 192, 192>>,
    Step<Pick<63, 96, 96>>,
    Step<Pluck<73, 78, 192, 192>>,
    Step<Pluck<73, 78, 192, 192>>,
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
pub struct LeadPart16(
    Step<Pick<68, 96, 96>>,
    Step<Pick<66, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<57, 96, 96>>,
    Step<Pick<57, 96, 96>>,
    Step<SplitPluck<56, 56, 192, 288, 192>>,
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
pub struct LeadPart17(
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
pub struct LeadPart18(
    Step<Pick<44, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    TriadHit<49, 56, 61, 192, 192>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    TriadHit<44, 49, 54, 96, 96>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart19(
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<Pluck<48, 55, 96, 192>>,
    Step<Pluck<46, 53, 96, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<SplitPluck<49, 56, 96, 192, 192>>,
    Step<SplitPluck<51, 58, 96, 192, 192>>,
    Step<Pick<44, 96, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart20(
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
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
pub struct LeadPart21(
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
    Step<Pluck<44, 49, 1536, 1536>>,
    Step<Pluck<61, 66, 1536, 1536>>,
    Step<Pluck<61, 65, 1536, 1536>>,
    Step<Pluck<42, 46, 1536, 1536>>,
    Step<Pluck<44, 49, 1536, 1536>>,
    Step<Pluck<42, 46, 1536, 1536>>,
    Step<Pluck<44, 49, 1152, 1152>>,
    TriadHit<49, 56, 59, 384, 384>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 384, 384>>,
    Step<Pluck<40, 64, 384, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart22(
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<SplitPluck<42, 49, 96, 192, 96>>,
    Step<Pick<42, 96, 96>>,
    Step<Pluck<42, 49, 384, 384>>,
    Step<Pluck<40, 64, 384, 384>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<42, 49, 384, 384>>,
    Step<Pluck<40, 64, 384, 384>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<42, 66, 192, 192>>,
    Step<SplitPluck<39, 73, 192, 960, 576>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart23(
    Step<Pick<73, 288, 192>>,
    Step<Pick<39, 96, 96>>,
    Step<Pick<73, 288, 288>>,
    Step<Pick<73, 960, 576>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<70, 192, 192>>,
    Step<Pick<68, 192, 192>>,
    Step<Pick<73, 192, 192>>,
    Step<SplitPluck<39, 68, 192, 384, 384>>,
    Step<Pick<70, 192, 192>>,
    Step<SplitPluck<39, 75, 96, 192, 192>>,
    Step<SplitPluck<39, 73, 96, 192, 192>>,
    Step<Pick<68, 384, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<66, 192, 192>>,
    Step<Pick<73, 384, 384>>,
    Step<Pick<78, 192, 192>>,
    Step<SplitPluck<39, 73, 96, 960, 192>>,
    Step<Pick<39, 96, 768>>,
    Step<Pick<48, 768, 768>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<55, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart24(
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
    Step<Pluck<67, 72, 960, 960>>,
    Step<Pluck<67, 70, 192, 192>>,
    Step<Pick<65, 96, 96>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<60, 192, 192>>,
    Step<Pick<58, 960, 960>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<60, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart25(
    Step<Pick<63, 192, 192>>,
    Step<Pick<60, 192, 192>>,
    Step<Pluck<63, 63, 576, 576>>,
    Step<Pluck<61, 63, 384, 384>>,
    Step<Pluck<60, 63, 384, 384>>,
    Step<Pluck<58, 63, 384, 384>>,
    Step<Pluck<56, 60, 384, 384>>,
    Step<Pluck<56, 60, 192, 192>>,
    Step<Pluck<54, 58, 192, 192>>,
    Step<Pick<55, 192, 192>>,
    Step<Pluck<55, 58, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<QuadHit<53, 58, 58, 58, 384, 384>>,
    Step<Pluck<58, 58, 192, 192>>,
    Step<Pluck<56, 58, 192, 192>>,
    Step<Pluck<56, 58, 192, 192>>,
    Step<Pluck<55, 58, 384, 384>>,
    Step<Pluck<56, 60, 192, 192>>,
    Step<Pluck<58, 62, 192, 192>>,
    Step<Pick<51, 288, 288>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<46, 96, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart26(
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
    Step<Pluck<63, 72, 96, 96>>,
    Step<Pluck<61, 70, 480, 672>>,
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
    Step<Pluck<58, 62, 1536, 1536>>,
    Step<Pluck<58, 62, 1536, 1536>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart27(
    Step<Pluck<49, 54, 96, 2880>>,
    Step<Pick<85, 768, 768>>,
    Step<Pick<82, 192, 192>>,
    Step<Pick<82, 192, 192>>,
    Step<Pick<81, 576, 576>>,
    Step<Pick<79, 384, 384>>,
    Step<Pluck<44, 79, 96, 48>>,
    Step<Pick<49, 96, 48>>,
    Step<SplitPluck<61, 77, 672, 288, 288>>,
    Step<Pick<87, 768, 384>>,
    Step<Pick<61, 384, 384>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    TriadHit<47, 54, 54, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart28(
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
);

#[derive(Flow)]
pub struct LeadPart29Phrase(
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    TriadHit<47, 54, 54, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart29(LeadPart29Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart30(
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<47, 53, 96, 96>>,
    Step<Pluck<47, 54, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart31(LeadPart29Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart32(
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<51, 58, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    TriadHit<47, 54, 54, 96, 96>,
    TriadHit<47, 54, 54, 96, 96>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<46, 53, 96, 96>>,
    Step<Pluck<41, 48, 384, 384>>,
    Step<Pluck<40, 47, 384, 384>>,
    Step<Pluck<41, 48, 384, 384>>,
    Step<Pluck<42, 49, 384, 384>>,
    Step<Pluck<44, 51, 384, 384>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart33(
    Step<Pluck<43, 50, 384, 384>>,
    Step<Pluck<44, 51, 384, 384>>,
    Step<Pluck<45, 52, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
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
pub struct LeadPart34(
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
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
pub struct LeadPart35(
    Step<Pick<51, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 192, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 96, 192>>,
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pluck<51, 58, 192, 192>>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 192, 192>>,
    Step<Pick<42, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart36(
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
    Step<Pluck<47, 54, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pick<42, 384, 384>>,
    Step<Pluck<49, 56, 384, 384>>,
    Step<Pick<48, 384, 384>>,
    Step<Pick<46, 384, 384>>,
    Step<Pick<44, 384, 384>>,
    Step<Pluck<58, 63, 288, 288>>,
    Step<Pluck<56, 61, 288, 288>>,
    Step<Pluck<52, 57, 288, 288>>,
    Step<Pluck<51, 56, 288, 288>>,
    Step<Pluck<49, 54, 192, 384>>,
    Step<Pluck<56, 60, 288, 288>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart37(
    Step<Pluck<54, 58, 288, 288>>,
    Step<Pluck<51, 55, 192, 576>>,
    Step<Pluck<61, 66, 3456, 3456>>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::LeadGuitarist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-lead-guitar-intro-test")
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
            .start_journey::<LeadGuitarist>(seed)
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
