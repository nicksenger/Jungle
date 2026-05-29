use jungle_sdk::prelude::*;

use super::{Double, LeadGuitarist, LeadGuitaristState, Quad};
use crate::action::{MergeUnit as GenericMergeUnit, Rest as GenericRest};
use crate::effect::{Rest, Sound, SoundInput};
use crate::instrumentation::{
    ElectricGuitar, ElectricGuitarArticulation, Pick as LanePick, Pluck as LanePluck,
    Strum as LaneStrum, Vocals, VocalsArticulation,
};

const RHYTHM_GUITAR_LANE_ID: u8 = <<LeadGuitarist as Animal>::Id as AnimalIdValue>::U32 as u8;
const INTRO_START_DELAY_TICKS: u32 = 0;
type MergeUnit = GenericMergeUnit<ElectricGuitarArticulation>;
type PostMergeRest<const TICKS: u32> =
    GenericRest<ElectricGuitarArticulation, TICKS, RHYTHM_GUITAR_LANE_ID>;

type Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePick<NOTE, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;
type Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;
type Pick58Tick = Step<Pick<58, 96, 96>>;
type Pluck58Tick = Pluck<58, 58, 96, 96>;
type Pluck56Tick = Pluck<56, 56, 96, 96>;
type Pluck53Tick = Pluck<53, 53, 96, 96>;
type Pluck51Tick = Pluck<51, 51, 96, 96>;
type Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
> = LaneStrum<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK, RHYTHM_GUITAR_LANE_ID>;

pub struct JoinPick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>;
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> Action
    for JoinPick<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Sound<ElectricGuitar>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: *state,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: RHYTHM_GUITAR_LANE_ID,
        }
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join note playback should succeed");
    }
}

#[derive(Flow)]
pub struct JoinPluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32>(
    Join<Step<JoinPick<NOTE_1, NOTE_TICK, 0>>, Step<JoinPick<NOTE_2, NOTE_TICK, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct Chord<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(
    Join<JoinPluck<NOTE_1, NOTE_2, NOTE_TICK, 0>, JoinPluck<NOTE_3, NOTE_4, NOTE_TICK, 0>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

#[derive(Flow)]
pub struct SplitPluck<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(
    Join<Step<JoinPick<NOTE_1, NOTE_TICK_1, 0>>, Step<JoinPick<NOTE_2, NOTE_TICK_2, 0>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK>>,
);

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

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: VocalsArticulation::GroupHarmony,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: RHYTHM_GUITAR_LANE_ID,
        }
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("backup vocal playback should succeed");
    }
}

pub struct LeadRiffLoopRemaining;
impl Predicate<(&LeadGuitaristState, &())> for LeadRiffLoopRemaining {
    fn eval((state, _): &(&LeadGuitaristState, &())) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct UseLeadTurnaroundSection;
impl Predicate<(LeadGuitaristState, ())> for UseLeadTurnaroundSection {
    fn eval((state, _): &(LeadGuitaristState, ())) -> bool {
        state.riff_loops_remaining <= 0
    }
}

pub struct DecrementLeadRiffLoop;
#[jungle::action]
impl Action for DecrementLeadRiffLoop {
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
        output.expect("rhythm riff loop decrement should complete");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

pub struct MergeLeadTurnaroundChoice;
#[jungle::action]
impl Action for MergeLeadTurnaroundChoice {
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
        output.expect("rhythm turnaround branch merge should complete");
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
    Transparent<IntroSectionMeta, LeadSection03>,
    Transparent<IntroSectionMeta, LeadSection04>,
    Conditional<UseLeadTurnaroundSection, LeadRiffLoopFinalTail, LeadRiffLoopNormalTail>,
    Step<MergeLeadTurnaroundChoice>,
);

#[derive(Flow)]
pub struct LeadGuitarFlow(
    Transparent<
        IntroSectionMeta,
        Step<GenericRest<LeadGuitaristState, INTRO_START_DELAY_TICKS, RHYTHM_GUITAR_LANE_ID>>,
    >,
    Transparent<IntroSectionMeta, LeadSection01>,
    Transparent<IntroSectionMeta, LeadSection02>,
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
pub struct LeadSection07(
    Transparent<IntroSectionMeta, LeadPart37>,
    Transparent<IntroSectionMeta, LeadPart38>,
    Transparent<IntroSectionMeta, LeadPart39>,
    Transparent<IntroSectionMeta, LeadPart40>,
    Transparent<IntroSectionMeta, LeadPart41>,
    Transparent<IntroSectionMeta, LeadPart42>,
);

#[derive(Flow)]
pub struct LeadSevenPick58(Quad<Pick58Tick>, Double<Pick58Tick>, Pick58Tick);

#[derive(Flow)]
pub struct LeadTriplePick58(Double<Pick58Tick>, Pick58Tick);

#[derive(Flow)]
pub struct LeadTriplePluck58(Double<Pluck58Tick>, Pluck58Tick);

#[derive(Flow)]
pub struct LeadTriplePluck56(Double<Pluck56Tick>, Pluck56Tick);

#[derive(Flow)]
pub struct LeadTriplePluck53(Double<Pluck53Tick>, Pluck53Tick);

#[derive(Flow)]
pub struct LeadTriplePluck51(Double<Pluck51Tick>, Pluck51Tick);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart01(
    Pluck58Tick,
    Pick58Tick,
    Pluck58Tick,
    Transparent<IntroSectionMeta, LeadSevenPick58>,
    Transparent<IntroSectionMeta, LeadTriplePluck58>,
    Transparent<IntroSectionMeta, LeadTriplePick58>,
    Pluck58Tick,
    Transparent<IntroSectionMeta, LeadSevenPick58>,
);

#[derive(Flow)]
pub struct LeadPart02Phrase(
    Pluck<58, 58, 96, 96>,
    Step<Pick<58, 96, 96>>,
    Pluck<58, 58, 96, 96>,
    Pluck<56, 56, 96, 96>,
    Step<Pick<56, 96, 96>>,
    Pluck<56, 56, 96, 96>,
    Pluck<53, 53, 96, 96>,
    Step<Pick<53, 96, 96>>,
    Pluck<53, 53, 96, 96>,
    Pluck<51, 51, 96, 96>,
    Step<Pick<51, 96, 96>>,
    Pluck<51, 51, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Step<Pick<49, 96, 96>>,
    Strum<46, 49, 46, 96, 96>,
    Pluck<49, 46, 96, 96>,
    Pluck<58, 58, 96, 96>,
    Step<Pick<58, 96, 96>>,
    Pluck<58, 58, 96, 96>,
    Pluck<56, 56, 96, 96>,
    Step<Pick<56, 96, 96>>,
    Pluck<56, 56, 96, 96>,
    Pluck<53, 53, 96, 96>,
    Step<Pick<53, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart02(LeadPart02Phrase);

#[derive(Flow)]
pub struct LeadPart03Phrase(
    Pluck<53, 53, 96, 96>,
    Pluck<51, 51, 96, 96>,
    Step<Pick<51, 96, 96>>,
    Pluck<51, 51, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Step<Pick<49, 96, 96>>,
    Strum<46, 49, 46, 96, 96>,
    Pluck<49, 46, 96, 96>,
    Pluck<58, 58, 96, 96>,
    Step<Pick<58, 96, 96>>,
    Pluck<58, 58, 96, 96>,
    Pluck<56, 56, 96, 96>,
    Step<Pick<56, 96, 96>>,
    Pluck<56, 56, 96, 96>,
    Pluck<53, 53, 96, 96>,
    Step<Pick<53, 96, 96>>,
    Pluck<53, 53, 96, 96>,
    Pluck<51, 51, 96, 96>,
    Step<Pick<51, 96, 96>>,
    Pluck<51, 51, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Step<Pick<49, 96, 96>>,
    Strum<46, 49, 46, 96, 96>,
    Pluck<49, 46, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart03(LeadPart03Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart04(LeadPart02Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart05(LeadPart03Phrase);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart06(
    LeadTriplePluck58,
    LeadTriplePluck56,
    LeadTriplePluck53,
    LeadTriplePluck51,
    Pluck<49, 49, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Strum<46, 49, 46, 96, 96>,
    Strum<44, 49, 46, 96, 96>,
    LeadTriplePluck58,
    LeadTriplePluck56,
    Double<Pluck53Tick>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart07(
    Pluck53Tick,
    LeadTriplePluck51,
    Pluck<49, 49, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Strum<46, 49, 46, 96, 96>,
    Strum<44, 49, 46, 96, 96>,
    LeadTriplePluck58,
    LeadTriplePluck56,
    LeadTriplePluck53,
    LeadTriplePluck51,
    Pluck<49, 49, 96, 96>,
    Pluck<49, 49, 96, 96>,
    Strum<46, 49, 46, 96, 96>,
    Strum<44, 49, 46, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart08(
    Pluck<56, 49, 768, 768>,
    Pluck<51, 44, 768, 768>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 192, 192>,
    Pluck<58, 46, 384, 384>,
    Pluck<58, 46, 384, 384>,
    Pluck<51, 44, 384, 384>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Pluck<54, 49, 96, 96>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart09(
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<51, 44, 96, 96>,
    Pluck<51, 44, 192, 192>,
    Pluck<54, 49, 96, 96>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<51, 44, 96, 96>,
    Pluck<51, 44, 192, 192>,
    Pluck<54, 49, 96, 96>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Pluck<51, 44, 192, 192>,
    Step<Pick<44, 192, 192>>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<63, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart10(
    Step<Pick<68, 480, 480>>,
    Step<Pick<47, 96, 96>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<41, 192, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart11(
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<58, 51, 384, 384>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart12(
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Strum<54, 49, 44, 96, 96>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart13(
    Pluck<58, 51, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart14(
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
pub struct LeadPart15(
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<43, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart16(
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<58, 51, 384, 384>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart17(
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Pluck<56, 49, 192, 192>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart18(
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart19(
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
    Pluck<53, 46, 288, 288>,
    Pluck<53, 46, 96, 96>,
    Pluck<53, 46, 576, 576>,
    Pluck<53, 46, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart20(
    Pluck<51, 44, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Step<Pick<39, 384, 384>>,
    Pluck<51, 39, 192, 192>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<39, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
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
    Strum<63, 58, 51, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart21(
    Strum<61, 56, 49, 96, 96>,
    Strum<63, 58, 51, 96, 192>,
    Step<Pick<39, 96, 96>>,
    Strum<63, 58, 51, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Strum<63, 58, 51, 192, 192>,
    Step<Pick<58, 192, 192>>,
    Strum<63, 58, 51, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<63, 58, 96, 96>,
    Pluck<66, 60, 192, 192>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<42, 96, 96>>,
    Step<Pick<41, 96, 96>>,
    Step<Pick<39, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Pluck<58, 51, 384, 384>,
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Pluck<63, 58, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<63, 58, 384, 384>,
    Step<Pick<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart22(
    Pluck<58, 51, 192, 192>,
    Step<Pick<65, 96, 96>>,
    Step<Pick<68, 96, 96>>,
    Step<Pick<61, 192, 192>>,
    Pluck<63, 58, 384, 384>,
    Pluck<58, 54, 96, 96>,
    Pluck<58, 54, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Pluck<57, 50, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 192, 192>,
    Pluck<57, 50, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 192, 192>,
    Pluck<57, 50, 192, 192>,
    Pluck<56, 49, 192, 192>,
    Pluck<57, 50, 192, 192>,
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 960, 960>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart23(
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart24(
    Step<Pick<39, 96, 192>>,
    Pluck<51, 44, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Step<Pick<42, 192, 192>>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 192>,
    Step<Pick<42, 96, 96>>,
    SplitPluck<42, 49, 96, 192, 192>,
    SplitPluck<41, 48, 96, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<58, 51, 384, 384>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Pluck<56, 49, 192, 192>,
    Strum<54, 49, 44, 96, 96>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart25(
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Pluck<56, 49, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<46, 192, 192>>,
    Pluck<58, 51, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 44, 96, 96>,
    Pluck<49, 56, 192, 192>,
    Pluck<49, 44, 96, 96>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 49, 96, 96>,
    Step<Pick<49, 192, 192>>,
    Step<Pick<48, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart26(
    Step<Pick<46, 192, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
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
pub struct LeadPart27(
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
    Pluck<59, 52, 192, 384>,
    Pluck<49, 42, 1152, 1152>,
    Pluck<64, 58, 192, 384>,
    Pluck<61, 49, 576, 576>,
    Pluck<61, 56, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart28(
    Step<Pick<65, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Pluck<59, 52, 192, 384>,
    Pluck<70, 66, 576, 576>,
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
    Pluck<59, 52, 192, 384>,
    Step<Pick<70, 576, 576>>,
    Step<Pick<71, 192, 192>>,
    Step<Pick<70, 192, 192>>,
    Step<Pick<61, 96, 96>>,
    Step<Pick<59, 96, 96>>,
    Pluck<68, 59, 192, 192>,
    Pluck<65, 56, 192, 192>,
    Pluck<65, 61, 1152, 1152>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart29(
    Pluck<59, 52, 192, 384>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 384, 384>,
    Pluck<66, 61, 384, 384>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 192, 192>,
    Pluck<66, 61, 96, 96>,
    Pluck<66, 61, 96, 96>,
    Pluck<66, 61, 384, 384>,
    Pluck<66, 61, 384, 384>,
    Pluck<61, 54, 192, 192>,
    Pluck<61, 54, 192, 192>,
    Pluck<61, 54, 192, 192>,
    Pluck<61, 54, 192, 192>,
    Pluck<61, 54, 384, 384>,
    Pluck<59, 52, 384, 384>,
    Pluck<63, 56, 192, 192>,
    Pluck<63, 56, 192, 192>,
    Pluck<63, 56, 192, 192>,
    Pluck<63, 56, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart30(
    Pluck<63, 56, 192, 192>,
    Pluck<63, 56, 192, 192>,
    Pluck<63, 56, 192, 192>,
    Pluck<61, 54, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<60, 54, 192, 192>,
    Pluck<61, 55, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<63, 58, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<63, 58, 192, 192>,
    Pluck<63, 58, 192, 192>,
    Pluck<60, 54, 192, 192>,
    Pluck<61, 55, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<66, 61, 384, 384>,
    Step<Pick<66, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Pluck<60, 54, 192, 192>,
    Pluck<61, 55, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart31(
    Pluck<63, 58, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Pluck<63, 58, 192, 192>,
    Pluck<63, 58, 192, 192>,
    Pluck<60, 54, 192, 192>,
    Pluck<61, 55, 192, 192>,
    Step<Pick<39, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Pluck<66, 61, 384, 384>,
    Step<Pick<66, 192, 192>>,
    Pluck<55, 48, 384, 384>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<51, 46, 192, 192>,
    Pluck<53, 46, 384, 384>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<54, 47, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart32(
    Pluck<55, 48, 384, 384>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<51, 46, 192, 192>,
    Pluck<53, 46, 384, 384>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<54, 47, 192, 192>,
    Pluck<55, 48, 384, 384>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<55, 48, 192, 192>,
    Pluck<51, 46, 192, 192>,
    Pluck<53, 46, 384, 384>,
    Pluck<51, 44, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<53, 46, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart33(
    Pluck<53, 46, 192, 192>,
    Pluck<51, 44, 192, 192>,
    Step<Pick<42, 192, 192>>,
    Pluck<46, 39, 384, 384>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<44, 39, 96, 192>,
    Pluck<46, 39, 192, 192>,
    Step<Pick<43, 192, 192>>,
    Step<Pick<44, 192, 192>>,
    Strum<58, 53, 46, 384, 384>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart34(
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<49, 46, 96, 192>,
    Pluck<51, 44, 192, 192>,
    Step<Pick<42, 192, 192>>,
    Strum<51, 46, 39, 384, 384>,
    Strum<63, 58, 51, 192, 192>,
    Strum<61, 56, 51, 192, 192>,
    Step<Pick<53, 192, 192>>,
    Pluck<61, 56, 192, 192>,
    Pluck<63, 58, 192, 192>,
    Step<Pick<44, 192, 192>>,
    Chord<63, 58, 51, 39, 384, 384>,
    Pluck<58, 51, 192, 192>,
    Pluck<56, 51, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<39, 192, 192>>,
    Pluck<53, 46, 576, 576>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart35(
    Pluck<53, 46, 384, 384>,
    Pluck<53, 46, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<53, 46, 384, 384>,
    Pluck<53, 46, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<53, 46, 192, 192>,
    Pluck<53, 46, 384, 384>,
    Step<Pick<51, 384, 384>>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<46, 39, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<46, 39, 192, 192>,
    Pluck<48, 41, 192, 192>,
    Pluck<46, 39, 192, 192>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart36(
    Pluck<48, 41, 384, 384>,
    Pluck<50, 46, 2688, 3072>,
    Pluck<61, 57, 1344, 1344>,
    Pluck<66, 78, 1728, 96>,
    Pluck<61, 73, 1632, 96>,
    Pluck<57, 69, 1536, 2880>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
    Strum<54, 49, 44, 96, 96>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<60, 56, 288, 288>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Pluck<60, 56, 192, 768>,
    Step<Pick<57, 96, 1344>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart37(
    Strum<54, 49, 44, 96, 96>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<60, 56, 288, 288>,
    Pluck<58, 54, 96, 96>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Pluck<60, 56, 192, 768>,
    Step<Pick<57, 96, 1344>>,
    Step<Pick<42, 192, 192>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<51, 384, 384>>,
    Step<Pick<49, 576, 576>>,
    Strum<54, 49, 44, 96, 96>,
    Strum<54, 49, 44, 96, 96>,
    Pluck<60, 56, 288, 288>,
    Pluck<58, 54, 96, 96>,
    Step<Pick<51, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<50, 96, 96>>,
    Pluck<60, 56, 192, 768>,
    Step<Pick<51, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart38(
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
    Pluck<48, 41, 384, 384>,
    Pluck<47, 40, 384, 384>,
    Pluck<48, 41, 384, 384>,
    Pluck<49, 42, 384, 384>,
    Pluck<51, 44, 384, 384>,
    Pluck<50, 43, 384, 384>,
    Pluck<51, 44, 384, 384>,
    Pluck<52, 45, 192, 192>,
    Step<Pick<39, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart39(
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
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
pub struct LeadPart40(
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
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
pub struct LeadPart41(
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 192, 192>,
    Pluck<58, 51, 192, 192>,
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
pub struct LeadPart42(
    Step<Pick<45, 96, 96>>,
    Step<Pick<45, 96, 192>>,
    Step<Pick<44, 96, 96>>,
    Step<Pick<44, 96, 192>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<39, 96, 192>>,
    Join<JoinPluck<54, 47, 384, 0>, Step<HarmonySing<71, 384, 0>>>,
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
    Join<JoinPluck<56, 49, 384, 0>, Step<HarmonySing<73, 384, 0>>>,
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
    Pluck<58, 51, 288, 288>,
    Pluck<56, 49, 288, 288>,
    Pluck<52, 45, 288, 288>,
    Pluck<51, 44, 288, 288>,
    Pluck<49, 42, 192, 192>,
    Step<Pick<39, 192, 192>>,
    Pluck<51, 44, 288, 288>,
    Pluck<49, 42, 288, 288>,
    Pluck<46, 39, 192, 576>,
    Pluck<46, 39, 3456, 0>,
);

#[cfg(test)]
pub struct LeadTailStubEffect;

#[cfg(test)]
#[jungle::effect(id = 963)]
impl<J> Effect<J> for LeadTailStubEffect {
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
pub struct LeadTailStub;

#[cfg(test)]
#[jungle::action]
impl Action for LeadTailStub {
    type Effect = LeadTailStubEffect;
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
        output.expect("rhythm tail stub should succeed");
    }
}

#[cfg(test)]
#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadJoinSound100JoinAndRest(
    Join<JoinPluck<54, 47, 100, 0>, Step<HarmonySing<71, 100, 0>>>,
    Step<MergeUnit>,
);

#[cfg(test)]
pub struct LeadLoopDecrementStub;

#[cfg(test)]
#[jungle::action]
impl Action for LeadLoopDecrementStub {
    type Effect = LeadTailStubEffect;
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
        output.expect("test loop decrement should succeed");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

#[cfg(test)]
#[derive(Flow)]
pub struct LeadJoinSound100LoopBody(
    Transparent<IntroSectionMeta, LeadJoinSound100JoinAndRest>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadTailStub>,
    Step<LeadLoopDecrementStub>,
);

#[cfg(test)]
#[derive(Flow)]
pub struct LeadJoinSound100Flow(While<LeadRiffLoopRemaining, LeadJoinSound100LoopBody>);

#[cfg(test)]
pub struct LeadJoinSound100Animal;

#[cfg(test)]
#[jungle::animal(id = 2, generation = 0)]
impl Animal for LeadJoinSound100Animal {
    type State = LeadGuitaristState;
    type Seed = LeadGuitaristState;
    type Journey = LeadJoinSound100Flow;
}

#[cfg(test)]
impl From<LeadGuitaristState> for () {
    fn from(_value: LeadGuitaristState) -> Self {}
}
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::StreamExt;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::*;
    use jungle_sdk::{JungleClient, LocalClient, RunnerUpdateOut};

    use super::super::LeadGuitarist;
    use super::{LeadGuitaristState, LeadJoinSound100Animal};
    use crate::ecosystem::TheJungle;
    const DUPLICATE_EXECUTION_WORKER_COUNT: usize = 5;
    const DUPLICATE_EXECUTION_TEST_BPM: f32 = 123.0;
    // Deterministic default LeadGuitarFlow path executes 1,221 effect steps:
    // each step emits one EffectInput and one EffectSuccessOutput.
    const EXPECTED_DUPLICATE_EXECUTION_INPUT_EVENTS: u32 = 1_221;
    const EXPECTED_DUPLICATE_EXECUTION_SUCCESS_EVENTS: u32 = 1_221;
    const EXPECTED_DUPLICATE_EXECUTION_TOTAL_EVENTS: u32 = 2_442;

    async fn await_completion_with_timeout(
        client: &LocalClient,
        journey_id: uuid::Uuid,
        timeout: Duration,
    ) {
        let completion = tokio::time::timeout(timeout, async {
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

    async fn await_completion(client: &LocalClient, journey_id: uuid::Uuid) {
        await_completion_with_timeout(client, journey_id, Duration::from_secs(8)).await;
    }

    struct JourneyStreamStats {
        total_events: u32,
        input_count: u32,
        success_count: u32,
        failed_count: u32,
        sleep_scheduled_count: u32,
        sleep_fired_count: u32,
    }

    #[derive(Clone, Copy)]
    struct JourneyEventCeilings {
        total_events: u32,
        input_count: u32,
        success_count: u32,
    }

    async fn collect_stream_stats(
        mut stream: jungle_sdk::client::JourneyUpdateSubscription,
        journey_id: uuid::Uuid,
        ceilings: Option<JourneyEventCeilings>,
    ) -> JourneyStreamStats {
        let mut total_events = 0_u32;
        let mut input_count = 0_u32;
        let mut success_count = 0_u32;
        let failed_count = 0_u32;
        let sleep_scheduled_count = 0_u32;
        let sleep_fired_count = 0_u32;
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
                RunnerUpdateOut::EffectInput { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    total_events += 1;
                    input_count += 1;
                }
                RunnerUpdateOut::EffectSuccessOutput { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    total_events += 1;
                    success_count += 1;
                }
                RunnerUpdateOut::EffectFailureOutput { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    panic!("unexpected effect-failure event emitted for rhythm guitarist flow");
                }
                RunnerUpdateOut::SleepScheduled { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    panic!("unexpected sleep-scheduled event emitted for rhythm guitarist flow");
                }
                RunnerUpdateOut::SleepFired { uuid, .. } => {
                    assert_eq!(uuid, journey_id, "stream update should match journey");
                    panic!("unexpected sleep-fired event emitted for rhythm guitarist flow");
                }
            }

            if let Some(ceilings) = ceilings {
                assert!(
                    input_count <= ceilings.input_count,
                    "unexpected extra effect-input events; expected at most {}, got {}",
                    ceilings.input_count,
                    input_count
                );
                assert!(
                    success_count <= ceilings.success_count,
                    "unexpected extra effect-success events; expected at most {}, got {}",
                    ceilings.success_count,
                    success_count
                );
                assert!(
                    total_events <= ceilings.total_events,
                    "unexpected extra total events; expected at most {}, got {}",
                    ceilings.total_events,
                    total_events
                );
            }
        }

        JourneyStreamStats {
            total_events,
            input_count,
            success_count,
            failed_count,
            sleep_scheduled_count,
            sleep_fired_count,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 16)]
    async fn lead_guitarist_flow_multi_worker_local_client_has_exact_event_counts() {
        let namespace = format!("welcome-rhythm-dup-exec-{}", uuid::Uuid::new_v4());
        let client = LocalClient::builder()
            .namespace(&namespace)
            .build()
            .await
            .expect("local client should build");

        let (shared_audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
        let shared_metronome = crate::metronome::Metronome::spawn(DUPLICATE_EXECUTION_TEST_BPM);

        let mut worker_handles = Vec::with_capacity(DUPLICATE_EXECUTION_WORKER_COUNT);
        for _ in 0..DUPLICATE_EXECUTION_WORKER_COUNT {
            let ecosystem = TheJungle::new_with_metronome(
                shared_audio_handle.clone(),
                DUPLICATE_EXECUTION_TEST_BPM,
                shared_metronome.clone(),
            );
            let worker = JungleWorker::new(ecosystem, client.clone());
            worker_handles.push(tokio::spawn(async move {
                let _ = worker.spawn().await;
            }));
        }

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<LeadGuitarist>(seed)
            .await
            .expect("journey should start");

        let stream = client
            .subscribe_step_updates(journey_id, None)
            .await
            .expect("subscribe_step_updates should succeed");
        let stream_task = tokio::spawn(async move {
            collect_stream_stats(
                stream,
                journey_id,
                Some(JourneyEventCeilings {
                    total_events: EXPECTED_DUPLICATE_EXECUTION_TOTAL_EVENTS,
                    input_count: EXPECTED_DUPLICATE_EXECUTION_INPUT_EVENTS,
                    success_count: EXPECTED_DUPLICATE_EXECUTION_SUCCESS_EVENTS,
                }),
            )
            .await
        });

        await_completion_with_timeout(&client, journey_id, Duration::from_secs(300)).await;
        let stats = stream_task
            .await
            .expect("stream task should join cleanly after completion");

        assert_eq!(
            stats.failed_count, 0,
            "rhythm guitarist flow should not emit failure events"
        );
        assert_eq!(
            stats.sleep_scheduled_count, 0,
            "rhythm guitarist flow should not schedule sleep events"
        );
        assert_eq!(
            stats.sleep_fired_count, 0,
            "rhythm guitarist flow should not fire sleep events"
        );
        assert_eq!(
            stats.input_count, EXPECTED_DUPLICATE_EXECUTION_INPUT_EVENTS,
            "unexpected effect-input event count; this can indicate duplicate journey execution"
        );
        assert_eq!(
            stats.success_count, EXPECTED_DUPLICATE_EXECUTION_SUCCESS_EVENTS,
            "unexpected effect-success event count; this can indicate duplicate journey execution"
        );
        assert_eq!(
            stats.total_events, EXPECTED_DUPLICATE_EXECUTION_TOTAL_EVENTS,
            "unexpected total event count; this can indicate duplicate journey execution"
        );

        for worker_handle in worker_handles {
            worker_handle.abort();
            let _ = worker_handle.await;
        }
    }

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-rhythm-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = welcome_audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move { worker.spawn().await });

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

    #[tokio::test]
    async fn join_Sound_100_ticks_zero_rest_with_tail_streams_events_and_completes_with_local_client(
    ) {
        const PARALLEL_JOURNEYS: usize = 1;

        let namespace = format!(
            "welcome-rhythm-join-Sound-100-test-{}",
            uuid::Uuid::new_v4()
        );
        let client = LocalClient::builder()
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

        let seed =
            postcard::to_allocvec(&LeadGuitaristState::default()).expect("seed should serialize");
        let mut journey_ids = Vec::with_capacity(PARALLEL_JOURNEYS);
        for index in 0..PARALLEL_JOURNEYS {
            let journey_id = client
                .start_journey::<LeadJoinSound100Animal>(seed.clone())
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
                collect_stream_stats(stream, stream_journey_id, None).await
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
