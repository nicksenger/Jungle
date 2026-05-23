use jungle_sdk::prelude::*;

use crate::effect::{AtomicDualHit, Rest, Tetrad, Triad};
use crate::instrumentation::{
    ElectricGuitar, ElectricGuitarArticulation, Pick as LanePick, Pluck as LanePluck,
};

use super::LeadGuitarist;

const LEAD_GUITAR_LANE_ID: u32 = <<LeadGuitarist as Animal>::Id as AnimalIdValue>::U32;
type Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePick<NOTE, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;
type Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LanePluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, LEAD_GUITAR_LANE_ID>;

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
            riff_loops_remaining: 6,
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

pub struct TriadHit<
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
    > Act for TriadHit<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
{
    type Effect = Triad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        LEAD_GUITAR_LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_3,
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
        output.expect("triad playback should succeed");
    }
}

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

#[derive(Flow)]
pub struct LeadGuitarIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, LeadSection01>,
    Transparent<IntroSectionMeta, LeadSection02>,
    Transparent<IntroSectionMeta, LeadSection03>,
    Transparent<IntroSectionMeta, LeadSection04>,
    Transparent<IntroSectionMeta, LeadSection05>,
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
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<41, 48, 192, 192>>,
    Step<Pluck<39, 46, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
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
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
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
    Step<TriadHit<39, 49, 54, 96, 96>>,
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
    Step<TriadHit<51, 51, 63, 192, 192>>,
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
    Step<TriadHit<49, 56, 61, 192, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<51, 58, 96, 192>>,
    Step<Pluck<49, 56, 96, 192>>,
    Step<TriadHit<44, 49, 54, 96, 96>>,
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
    Step<TriadHit<49, 56, 59, 384, 384>>,
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
    Step<TriadHit<47, 54, 54, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart28(
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
    Step<TriadHit<47, 54, 54, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
pub struct LeadPart29(
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
    Step<TriadHit<47, 54, 54, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart30(
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
pub struct LeadPart31(
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<50, 57, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<49, 56, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<Pluck<48, 55, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
    Step<TriadHit<47, 54, 54, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPart32(
    Step<TriadHit<47, 54, 54, 96, 96>>,
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
    Step<TriadHit<47, 54, 54, 96, 96>>,
    Step<TriadHit<47, 54, 54, 96, 96>>,
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

//2, 0, Title_t, "Electric Guitar"
//2, 0, Program_c, 1, 27
//2, 5376, Note_on_c, 1, 53, 37
//2, 5376, Note_on_c, 1, 46, 37
//2, 6528, Note_on_c, 1, 61, 37
//2, 6720, Note_off_c, 1, 61, 0
//2, 6720, Note_on_c, 1, 63, 37
//2, 6912, Note_off_c, 1, 46, 0
//2, 6912, Note_off_c, 1, 53, 0
//2, 6912, Note_off_c, 1, 63, 0
//2, 6912, Note_on_c, 1, 63, 37
//2, 7200, Note_off_c, 1, 63, 0
//2, 7200, Note_on_c, 1, 61, 37
//2, 7872, Note_off_c, 1, 61, 0
//2, 7872, Note_on_c, 1, 61, 37
//2, 8064, Note_off_c, 1, 61, 0
//2, 8448, Note_on_c, 1, 56, 37
//2, 8544, Note_off_c, 1, 56, 0
//2, 8544, Note_on_c, 1, 58, 27
//2, 9600, Note_off_c, 1, 58, 0
//2, 9792, Note_on_c, 1, 58, 37
//2, 9984, Note_off_c, 1, 58, 0
//2, 9984, Note_on_c, 1, 51, 37
//2, 11328, Note_on_c, 1, 53, 37
//2, 11520, Note_off_c, 1, 51, 0
//2, 11520, Note_off_c, 1, 53, 0
//2, 11520, Note_on_c, 1, 65, 37
//2, 11520, Note_on_c, 1, 58, 37
//2, 12672, Note_off_c, 1, 58, 0
//2, 12672, Note_off_c, 1, 65, 0
//2, 12864, Note_on_c, 1, 65, 37
//2, 12864, Note_on_c, 1, 58, 37
//2, 13056, Note_off_c, 1, 58, 0
//2, 13056, Note_off_c, 1, 65, 0
//2, 13056, Note_on_c, 1, 51, 37
//2, 13056, Note_on_c, 1, 44, 37
//2, 14400, Note_on_c, 1, 56, 37
//2, 14592, Note_off_c, 1, 44, 0
//2, 14592, Note_off_c, 1, 51, 0
//2, 14592, Note_off_c, 1, 56, 0
//2, 14592, Note_on_c, 1, 46, 37
//2, 14592, Note_on_c, 1, 39, 37
//2, 16128, Note_off_c, 1, 39, 0
//2, 16128, Note_off_c, 1, 46, 0
//2, 16128, Note_on_c, 1, 56, 37
//2, 16128, Note_on_c, 1, 49, 37
//2, 16896, Note_off_c, 1, 49, 0
//2, 16896, Note_off_c, 1, 56, 0
//2, 16896, Note_on_c, 1, 51, 37
//2, 16896, Note_on_c, 1, 44, 37
//2, 17664, Note_off_c, 1, 44, 0
//2, 17664, Note_off_c, 1, 51, 0
//2, 17664, Note_on_c, 1, 46, 37
//2, 17856, Note_off_c, 1, 46, 0
//2, 17856, Note_on_c, 1, 48, 37
//2, 18048, Note_off_c, 1, 48, 0
//2, 18048, Note_on_c, 1, 51, 37
//2, 18240, Note_off_c, 1, 51, 0
//2, 18240, Note_on_c, 1, 53, 37
//2, 18432, Note_off_c, 1, 53, 0
//2, 18432, Note_on_c, 1, 56, 37
//2, 18624, Note_off_c, 1, 56, 0
//2, 18624, Note_on_c, 1, 58, 37
//2, 18816, Note_off_c, 1, 58, 0
//2, 18816, Note_on_c, 1, 61, 37
//2, 19008, Note_off_c, 1, 61, 0
//2, 19008, Note_on_c, 1, 63, 37
//2, 19200, Note_off_c, 1, 63, 0
//2, 19200, Note_on_c, 1, 63, 37
//2, 19392, Note_off_c, 1, 63, 0
//2, 19392, Note_on_c, 1, 68, 37
//2, 19584, Note_off_c, 1, 68, 0
//2, 19584, Note_on_c, 1, 68, 37
//2, 19584, Note_on_c, 1, 63, 37
//2, 19968, Note_off_c, 1, 63, 0
//2, 19968, Note_off_c, 1, 68, 0
//2, 19968, Note_on_c, 1, 61, 37
//2, 20352, Note_off_c, 1, 61, 0
//2, 20352, Note_on_c, 1, 58, 37
//2, 20736, Note_off_c, 1, 58, 0
//2, 20736, Note_on_c, 1, 51, 37
//2, 20736, Note_on_c, 1, 44, 37
//2, 20928, Note_off_c, 1, 44, 0
//2, 20928, Note_off_c, 1, 51, 0
//2, 20928, Note_on_c, 1, 51, 37
//2, 20928, Note_on_c, 1, 44, 37
//2, 21120, Note_off_c, 1, 44, 0
//2, 21120, Note_off_c, 1, 51, 0
//2, 21120, Note_on_c, 1, 49, 37
//2, 21120, Note_on_c, 1, 42, 37
//2, 21312, Note_off_c, 1, 42, 0
//2, 21312, Note_off_c, 1, 49, 0
//2, 21312, Note_on_c, 1, 51, 37
//2, 21312, Note_on_c, 1, 44, 37
//2, 21408, Note_off_c, 1, 44, 0
//2, 21408, Note_off_c, 1, 51, 0
//2, 21408, Note_on_c, 1, 51, 37
//2, 21408, Note_on_c, 1, 44, 37
//2, 21600, Note_off_c, 1, 44, 0
//2, 21600, Note_off_c, 1, 51, 0
//2, 21600, Note_on_c, 1, 51, 37
//2, 21600, Note_on_c, 1, 44, 37
//2, 21696, Note_off_c, 1, 44, 0
//2, 21696, Note_off_c, 1, 51, 0
//2, 21696, Note_on_c, 1, 49, 37
//2, 21696, Note_on_c, 1, 42, 37
//2, 21888, Note_off_c, 1, 42, 0
//2, 21888, Note_off_c, 1, 49, 0
//2, 21888, Note_on_c, 1, 48, 37
//2, 21888, Note_on_c, 1, 41, 37
//2, 22080, Note_off_c, 1, 41, 0
//2, 22080, Note_off_c, 1, 48, 0
//2, 22080, Note_on_c, 1, 46, 37
//2, 22080, Note_on_c, 1, 39, 37
//2, 22272, Note_off_c, 1, 39, 0
//2, 22272, Note_off_c, 1, 46, 0
//2, 22272, Note_on_c, 1, 51, 37
//2, 22272, Note_on_c, 1, 44, 37
//2, 22464, Note_off_c, 1, 44, 0
//2, 22464, Note_off_c, 1, 51, 0
//2, 22464, Note_on_c, 1, 51, 37
//2, 22464, Note_on_c, 1, 44, 37
//2, 22656, Note_off_c, 1, 44, 0
//2, 22656, Note_off_c, 1, 51, 0
//2, 22656, Note_on_c, 1, 49, 37
//2, 22656, Note_on_c, 1, 42, 37
//2, 22848, Note_off_c, 1, 42, 0
//2, 22848, Note_off_c, 1, 49, 0
//2, 22848, Note_on_c, 1, 51, 37
//2, 22848, Note_on_c, 1, 44, 37
//2, 22944, Note_off_c, 1, 44, 0
//2, 22944, Note_off_c, 1, 51, 0
//2, 22944, Note_on_c, 1, 51, 37
//2, 22944, Note_on_c, 1, 44, 37
//2, 23136, Note_off_c, 1, 44, 0
//2, 23136, Note_off_c, 1, 51, 0
//2, 23136, Note_on_c, 1, 51, 37
//2, 23136, Note_on_c, 1, 44, 37
//2, 23232, Note_off_c, 1, 44, 0
//2, 23232, Note_off_c, 1, 51, 0
//2, 23232, Note_on_c, 1, 49, 37
//2, 23232, Note_on_c, 1, 42, 37
//2, 23424, Note_off_c, 1, 42, 0
//2, 23424, Note_off_c, 1, 49, 0
//2, 23424, Note_on_c, 1, 48, 37
//2, 23424, Note_on_c, 1, 41, 37
//2, 23616, Note_off_c, 1, 41, 0
//2, 23616, Note_off_c, 1, 48, 0
//2, 23616, Note_on_c, 1, 46, 37
//2, 23616, Note_on_c, 1, 39, 37
//2, 23808, Note_off_c, 1, 39, 0
//2, 23808, Note_off_c, 1, 46, 0
//2, 23808, Note_on_c, 1, 51, 37
//2, 23808, Note_on_c, 1, 44, 37
//2, 24000, Note_off_c, 1, 44, 0
//2, 24000, Note_off_c, 1, 51, 0
//2, 24000, Note_on_c, 1, 51, 37
//2, 24000, Note_on_c, 1, 44, 37
//2, 24192, Note_off_c, 1, 44, 0
//2, 24192, Note_off_c, 1, 51, 0
//2, 24192, Note_on_c, 1, 49, 37
//2, 24192, Note_on_c, 1, 42, 37
//2, 24384, Note_off_c, 1, 42, 0
//2, 24384, Note_off_c, 1, 49, 0
//2, 24384, Note_on_c, 1, 51, 37
//2, 24384, Note_on_c, 1, 44, 37
//2, 24480, Note_off_c, 1, 44, 0
//2, 24480, Note_off_c, 1, 51, 0
//2, 24480, Note_on_c, 1, 51, 37
//2, 24480, Note_on_c, 1, 44, 37
//2, 24672, Note_off_c, 1, 44, 0
//2, 24672, Note_off_c, 1, 51, 0
//2, 24672, Note_on_c, 1, 51, 37
//2, 24672, Note_on_c, 1, 44, 37
//2, 24768, Note_off_c, 1, 44, 0
//2, 24768, Note_off_c, 1, 51, 0
//2, 24768, Note_on_c, 1, 49, 37
//2, 24768, Note_on_c, 1, 42, 37
//2, 24960, Note_off_c, 1, 42, 0
//2, 24960, Note_off_c, 1, 49, 0
//2, 24960, Note_on_c, 1, 48, 37
//2, 24960, Note_on_c, 1, 41, 37
//2, 25152, Note_off_c, 1, 41, 0
//2, 25152, Note_off_c, 1, 48, 0
//2, 25152, Note_on_c, 1, 46, 37
//2, 25152, Note_on_c, 1, 39, 37
//2, 25344, Note_off_c, 1, 39, 0
//2, 25344, Note_off_c, 1, 46, 0
//2, 25344, Note_on_c, 1, 61, 12
//2, 25344, Note_on_c, 1, 73, 37
//2, 25536, Note_off_c, 1, 73, 0
//2, 25536, Note_off_c, 1, 61, 0
//2, 25536, Note_on_c, 1, 59, 37
//2, 25632, Note_off_c, 1, 59, 0
//2, 25632, Note_on_c, 1, 56, 37
//2, 25728, Note_off_c, 1, 56, 0
//2, 25728, Note_on_c, 1, 61, 37
//2, 25824, Note_off_c, 1, 61, 0
//2, 25824, Note_on_c, 1, 59, 37
//2, 25920, Note_off_c, 1, 59, 0
//2, 25920, Note_on_c, 1, 56, 37
//2, 26016, Note_off_c, 1, 56, 0
//2, 26016, Note_on_c, 1, 54, 37
//2, 26112, Note_off_c, 1, 54, 0
//2, 26112, Note_on_c, 1, 51, 37
//2, 26208, Note_off_c, 1, 51, 0
//2, 26208, Note_on_c, 1, 50, 37
//2, 26304, Note_off_c, 1, 50, 0
//2, 26304, Note_on_c, 1, 49, 37
//2, 26400, Note_off_c, 1, 49, 0
//2, 26400, Note_on_c, 1, 47, 37
//2, 26496, Note_off_c, 1, 47, 0
//2, 26496, Note_on_c, 1, 49, 37
//2, 26592, Note_off_c, 1, 49, 0
//2, 26592, Note_on_c, 1, 47, 37
//2, 26688, Note_off_c, 1, 47, 0
//2, 26688, Note_on_c, 1, 44, 37
//2, 26784, Note_off_c, 1, 44, 0
//2, 26784, Note_on_c, 1, 42, 37
//2, 26880, Note_off_c, 1, 42, 0
//2, 26880, Note_on_c, 1, 44, 37
//2, 26976, Note_off_c, 1, 44, 0
//2, 27072, Note_on_c, 1, 44, 37
//2, 27168, Note_off_c, 1, 44, 0
//2, 27264, Note_on_c, 1, 42, 37
//2, 27552, Note_off_c, 1, 42, 0
//2, 27552, Note_on_c, 1, 44, 37
//2, 27648, Note_off_c, 1, 44, 0
//2, 27744, Note_on_c, 1, 44, 37
//2, 27840, Note_off_c, 1, 44, 0
//2, 27840, Note_on_c, 1, 42, 37
//2, 28032, Note_off_c, 1, 42, 0
//2, 28032, Note_on_c, 1, 41, 37
//2, 28224, Note_off_c, 1, 41, 0
//2, 28224, Note_on_c, 1, 39, 37
//2, 28320, Note_off_c, 1, 39, 0
//2, 28320, Note_on_c, 1, 39, 37
//2, 28416, Note_off_c, 1, 39, 0
//2, 28416, Note_on_c, 1, 44, 37
//2, 28512, Note_off_c, 1, 44, 0
//2, 28512, Note_on_c, 1, 44, 37
//2, 28608, Note_off_c, 1, 44, 0
//2, 28608, Note_on_c, 1, 44, 37
//2, 28704, Note_off_c, 1, 44, 0
//2, 28800, Note_on_c, 1, 42, 37
//2, 28992, Note_off_c, 1, 42, 0
//2, 28992, Note_on_c, 1, 44, 37
//2, 29088, Note_off_c, 1, 44, 0
//2, 29088, Note_on_c, 1, 44, 37
//2, 29184, Note_off_c, 1, 44, 0
//2, 29280, Note_on_c, 1, 44, 37
//2, 29376, Note_off_c, 1, 44, 0
//2, 29376, Note_on_c, 1, 42, 37
//2, 29472, Note_off_c, 1, 42, 0
//2, 29568, Note_on_c, 1, 41, 37
//2, 29664, Note_off_c, 1, 41, 0
//2, 29760, Note_on_c, 1, 39, 37
//2, 29856, Note_off_c, 1, 39, 0
//2, 29856, Note_on_c, 1, 39, 37
//2, 29952, Note_off_c, 1, 39, 0
//2, 29952, Note_on_c, 1, 44, 37
//2, 30048, Note_off_c, 1, 44, 0
//2, 30048, Note_on_c, 1, 44, 37
//2, 30144, Note_off_c, 1, 44, 0
//2, 30144, Note_on_c, 1, 44, 37
//2, 30240, Note_off_c, 1, 44, 0
//2, 30336, Note_on_c, 1, 42, 37
//2, 30528, Note_off_c, 1, 42, 0
//2, 30528, Note_on_c, 1, 44, 37
//2, 30624, Note_off_c, 1, 44, 0
//2, 30624, Note_on_c, 1, 44, 37
//2, 30720, Note_off_c, 1, 44, 0
//2, 30816, Note_on_c, 1, 44, 37
//2, 30912, Note_off_c, 1, 44, 0
//2, 30912, Note_on_c, 1, 42, 37
//2, 31008, Note_off_c, 1, 42, 0
//2, 31104, Note_on_c, 1, 41, 37
//2, 31200, Note_off_c, 1, 41, 0
//2, 31296, Note_on_c, 1, 39, 37
//2, 31392, Note_off_c, 1, 39, 0
//2, 31392, Note_on_c, 1, 39, 37
//2, 31488, Note_off_c, 1, 39, 0
//2, 31488, Note_on_c, 1, 44, 37
//2, 31584, Note_off_c, 1, 44, 0
//2, 31584, Note_on_c, 1, 44, 37
//2, 31680, Note_off_c, 1, 44, 0
//2, 31680, Note_on_c, 1, 44, 37
//2, 31776, Note_off_c, 1, 44, 0
//2, 31872, Note_on_c, 1, 42, 37
//2, 32064, Note_off_c, 1, 42, 0
//2, 32064, Note_on_c, 1, 44, 37
//2, 32160, Note_off_c, 1, 44, 0
//2, 32160, Note_on_c, 1, 44, 37
//2, 32256, Note_off_c, 1, 44, 0
//2, 32352, Note_on_c, 1, 44, 37
//2, 32448, Note_off_c, 1, 44, 0
//2, 32448, Note_on_c, 1, 42, 37
//2, 32544, Note_off_c, 1, 42, 0
//2, 32640, Note_on_c, 1, 41, 37
//2, 32736, Note_off_c, 1, 41, 0
//2, 32832, Note_on_c, 1, 56, 37
//2, 32832, Note_on_c, 1, 49, 37
//2, 33024, Note_off_c, 1, 49, 0
//2, 33024, Note_off_c, 1, 56, 0
//2, 33024, Note_on_c, 1, 58, 37
//2, 33024, Note_on_c, 1, 51, 37
//2, 33120, Note_off_c, 1, 51, 0
//2, 33120, Note_off_c, 1, 58, 0
//2, 33216, Note_on_c, 1, 58, 37
//2, 33216, Note_on_c, 1, 51, 37
//2, 33312, Note_off_c, 1, 51, 0
//2, 33312, Note_off_c, 1, 58, 0
//2, 33408, Note_on_c, 1, 56, 37
//2, 33408, Note_on_c, 1, 49, 37
//2, 33504, Note_off_c, 1, 49, 0
//2, 33504, Note_off_c, 1, 56, 0
//2, 33600, Note_on_c, 1, 49, 37
//2, 33600, Note_on_c, 1, 44, 37
//2, 33696, Note_off_c, 1, 44, 0
//2, 33696, Note_off_c, 1, 49, 0
//2, 33696, Note_on_c, 1, 58, 37
//2, 33696, Note_on_c, 1, 51, 37
//2, 33792, Note_off_c, 1, 51, 0
//2, 33792, Note_off_c, 1, 58, 0
//2, 33888, Note_on_c, 1, 58, 37
//2, 33888, Note_on_c, 1, 51, 37
//2, 33984, Note_off_c, 1, 51, 0
//2, 33984, Note_off_c, 1, 58, 0
//2, 33984, Note_on_c, 1, 56, 37
//2, 33984, Note_on_c, 1, 49, 37
//2, 34080, Note_off_c, 1, 49, 0
//2, 34080, Note_off_c, 1, 56, 0
//2, 34176, Note_on_c, 1, 55, 37
//2, 34176, Note_on_c, 1, 48, 37
//2, 34272, Note_off_c, 1, 48, 0
//2, 34272, Note_off_c, 1, 55, 0
//2, 34368, Note_on_c, 1, 53, 37
//2, 34368, Note_on_c, 1, 46, 37
//2, 34464, Note_off_c, 1, 46, 0
//2, 34464, Note_off_c, 1, 53, 0
//2, 34560, Note_on_c, 1, 58, 37
//2, 34560, Note_on_c, 1, 51, 37
//2, 34656, Note_off_c, 1, 51, 0
//2, 34656, Note_off_c, 1, 58, 0
//2, 34752, Note_on_c, 1, 58, 37
//2, 34752, Note_on_c, 1, 51, 37
//2, 34848, Note_off_c, 1, 51, 0
//2, 34848, Note_off_c, 1, 58, 0
//2, 34944, Note_on_c, 1, 56, 37
//2, 34944, Note_on_c, 1, 49, 37
//2, 35040, Note_off_c, 1, 49, 0
//2, 35040, Note_off_c, 1, 56, 0
//2, 35136, Note_on_c, 1, 49, 37
//2, 35136, Note_on_c, 1, 44, 37
//2, 35232, Note_off_c, 1, 44, 0
//2, 35232, Note_off_c, 1, 49, 0
//2, 35232, Note_on_c, 1, 58, 37
//2, 35232, Note_on_c, 1, 51, 37
//2, 35328, Note_off_c, 1, 51, 0
//2, 35328, Note_off_c, 1, 58, 0
//2, 35424, Note_on_c, 1, 58, 37
//2, 35424, Note_on_c, 1, 51, 37
//2, 35520, Note_off_c, 1, 51, 0
//2, 35520, Note_off_c, 1, 58, 0
//2, 35520, Note_on_c, 1, 56, 37
//2, 35520, Note_on_c, 1, 49, 37
//2, 35616, Note_off_c, 1, 49, 0
//2, 35616, Note_off_c, 1, 56, 0
//2, 35712, Note_on_c, 1, 55, 37
//2, 35712, Note_on_c, 1, 48, 37
//2, 35808, Note_off_c, 1, 48, 0
//2, 35808, Note_off_c, 1, 55, 0
//2, 35904, Note_on_c, 1, 53, 37
//2, 35904, Note_on_c, 1, 46, 37
//2, 36000, Note_off_c, 1, 46, 0
//2, 36000, Note_off_c, 1, 53, 0
//2, 36096, Note_on_c, 1, 58, 37
//2, 36096, Note_on_c, 1, 51, 37
//2, 36192, Note_off_c, 1, 51, 0
//2, 36192, Note_off_c, 1, 58, 0
//2, 36288, Note_on_c, 1, 58, 37
//2, 36288, Note_on_c, 1, 51, 37
//2, 36384, Note_off_c, 1, 51, 0
//2, 36384, Note_off_c, 1, 58, 0
//2, 36480, Note_on_c, 1, 56, 37
//2, 36480, Note_on_c, 1, 49, 37
//2, 36576, Note_off_c, 1, 49, 0
//2, 36576, Note_off_c, 1, 56, 0
//2, 36672, Note_on_c, 1, 49, 37
//2, 36672, Note_on_c, 1, 44, 37
//2, 36768, Note_off_c, 1, 44, 0
//2, 36768, Note_off_c, 1, 49, 0
//2, 36768, Note_on_c, 1, 58, 37
//2, 36768, Note_on_c, 1, 51, 37
//2, 36864, Note_off_c, 1, 51, 0
//2, 36864, Note_off_c, 1, 58, 0
//2, 36960, Note_on_c, 1, 58, 37
//2, 36960, Note_on_c, 1, 51, 37
//2, 37056, Note_off_c, 1, 51, 0
//2, 37056, Note_off_c, 1, 58, 0
//2, 37056, Note_on_c, 1, 56, 37
//2, 37056, Note_on_c, 1, 49, 37
//2, 37152, Note_off_c, 1, 49, 0
//2, 37152, Note_off_c, 1, 56, 0
//2, 37248, Note_on_c, 1, 55, 37
//2, 37248, Note_on_c, 1, 48, 37
//2, 37344, Note_off_c, 1, 48, 0
//2, 37344, Note_off_c, 1, 55, 0
//2, 37440, Note_on_c, 1, 53, 37
//2, 37440, Note_on_c, 1, 46, 37
//2, 37536, Note_off_c, 1, 46, 0
//2, 37536, Note_off_c, 1, 53, 0
//2, 37632, Note_on_c, 1, 51, 37
//2, 37632, Note_on_c, 1, 58, 37
//2, 37728, Note_off_c, 1, 51, 0
//2, 37824, Note_off_c, 1, 58, 0
//2, 37824, Note_on_c, 1, 51, 37
//2, 37824, Note_on_c, 1, 58, 37
//2, 37920, Note_off_c, 1, 51, 0
//2, 38016, Note_off_c, 1, 58, 0
//2, 38016, Note_on_c, 1, 49, 37
//2, 38016, Note_on_c, 1, 56, 37
//2, 38112, Note_off_c, 1, 49, 0
//2, 38208, Note_off_c, 1, 56, 0
//2, 38208, Note_on_c, 1, 49, 37
//2, 38208, Note_on_c, 1, 44, 37
//2, 38304, Note_off_c, 1, 44, 0
//2, 38304, Note_off_c, 1, 49, 0
//2, 38304, Note_on_c, 1, 51, 37
//2, 38304, Note_on_c, 1, 58, 37
//2, 38400, Note_off_c, 1, 51, 0
//2, 38496, Note_off_c, 1, 58, 0
//2, 38496, Note_on_c, 1, 51, 37
//2, 38496, Note_on_c, 1, 58, 37
//2, 38592, Note_off_c, 1, 58, 0
//2, 38592, Note_off_c, 1, 51, 0
//2, 38592, Note_on_c, 1, 49, 37
//2, 38592, Note_on_c, 1, 56, 37
//2, 38688, Note_off_c, 1, 49, 0
//2, 38784, Note_off_c, 1, 56, 0
//2, 38784, Note_on_c, 1, 51, 37
//2, 38784, Note_on_c, 1, 58, 37
//2, 38880, Note_off_c, 1, 51, 0
//2, 38976, Note_off_c, 1, 58, 0
//2, 38976, Note_on_c, 1, 44, 37
//2, 39072, Note_off_c, 1, 44, 0
//2, 39168, Note_on_c, 1, 54, 37
//2, 39168, Note_on_c, 1, 47, 37
//2, 39552, Note_off_c, 1, 47, 0
//2, 39552, Note_off_c, 1, 54, 0
//2, 39552, Note_on_c, 1, 46, 37
//2, 39936, Note_off_c, 1, 46, 0
//2, 39936, Note_on_c, 1, 44, 37
//2, 40320, Note_off_c, 1, 44, 0
//2, 40320, Note_on_c, 1, 42, 37
//2, 40704, Note_off_c, 1, 42, 0
//2, 40704, Note_on_c, 1, 56, 37
//2, 40704, Note_on_c, 1, 49, 37
//2, 41088, Note_off_c, 1, 49, 0
//2, 41088, Note_off_c, 1, 56, 0
//2, 41088, Note_on_c, 1, 48, 37
//2, 41472, Note_off_c, 1, 48, 0
//2, 41472, Note_on_c, 1, 46, 37
//2, 41856, Note_off_c, 1, 46, 0
//2, 41856, Note_on_c, 1, 44, 37
//2, 42240, Note_off_c, 1, 44, 0
//2, 42240, Note_on_c, 1, 58, 37
//2, 42240, Note_on_c, 1, 51, 37
//2, 42432, Note_off_c, 1, 51, 0
//2, 42432, Note_off_c, 1, 58, 0
//2, 42432, Note_on_c, 1, 58, 37
//2, 42432, Note_on_c, 1, 51, 37
//2, 42624, Note_off_c, 1, 51, 0
//2, 42624, Note_off_c, 1, 58, 0
//2, 42624, Note_on_c, 1, 49, 37
//2, 42816, Note_off_c, 1, 49, 0
//2, 42816, Note_on_c, 1, 45, 37
//2, 42912, Note_off_c, 1, 45, 0
//2, 42912, Note_on_c, 1, 45, 37
//2, 43008, Note_off_c, 1, 45, 0
//2, 43104, Note_on_c, 1, 44, 37
//2, 43200, Note_off_c, 1, 44, 0
//2, 43200, Note_on_c, 1, 44, 37
//2, 43392, Note_off_c, 1, 44, 0
//2, 43392, Note_on_c, 1, 42, 37
//2, 43584, Note_off_c, 1, 42, 0
//2, 43584, Note_on_c, 1, 39, 37
//2, 43680, Note_off_c, 1, 39, 0
//2, 43776, Note_on_c, 1, 51, 37
//2, 43968, Note_off_c, 1, 51, 0
//2, 43968, Note_on_c, 1, 51, 37
//2, 44160, Note_off_c, 1, 51, 0
//2, 44160, Note_on_c, 1, 49, 37
//2, 44352, Note_off_c, 1, 49, 0
//2, 44352, Note_on_c, 1, 45, 37
//2, 44448, Note_off_c, 1, 45, 0
//2, 44448, Note_on_c, 1, 45, 37
//2, 44640, Note_off_c, 1, 45, 0
//2, 44640, Note_on_c, 1, 44, 37
//2, 44736, Note_off_c, 1, 44, 0
//2, 44736, Note_on_c, 1, 44, 37
//2, 44928, Note_off_c, 1, 44, 0
//2, 44928, Note_on_c, 1, 42, 37
//2, 45120, Note_off_c, 1, 42, 0
//2, 45120, Note_on_c, 1, 39, 37
//2, 45216, Note_off_c, 1, 39, 0
//2, 45312, Note_on_c, 1, 51, 37
//2, 45504, Note_off_c, 1, 51, 0
//2, 45504, Note_on_c, 1, 51, 37
//2, 45696, Note_off_c, 1, 51, 0
//2, 45696, Note_on_c, 1, 49, 37
//2, 45888, Note_off_c, 1, 49, 0
//2, 45888, Note_on_c, 1, 45, 37
//2, 45984, Note_off_c, 1, 45, 0
//2, 45984, Note_on_c, 1, 45, 37
//2, 46176, Note_off_c, 1, 45, 0
//2, 46176, Note_on_c, 1, 44, 37
//2, 46272, Note_off_c, 1, 44, 0
//2, 46272, Note_on_c, 1, 44, 37
//2, 46464, Note_off_c, 1, 44, 0
//2, 46464, Note_on_c, 1, 42, 37
//2, 46560, Note_off_c, 1, 42, 0
//2, 46656, Note_on_c, 1, 39, 37
//2, 46752, Note_off_c, 1, 39, 0
//2, 46848, Note_on_c, 1, 51, 37
//2, 47136, Note_off_c, 1, 51, 0
//2, 47136, Note_on_c, 1, 49, 37
//2, 47424, Note_off_c, 1, 49, 0
//2, 47424, Note_on_c, 1, 45, 37
//2, 47808, Note_off_c, 1, 45, 0
//2, 47808, Note_on_c, 1, 44, 37
//2, 48000, Note_off_c, 1, 44, 0
//2, 48000, Note_on_c, 1, 42, 37
//2, 48192, Note_off_c, 1, 42, 0
//2, 48192, Note_on_c, 1, 43, 37
//2, 48384, Note_off_c, 1, 43, 0
//2, 48384, Note_on_c, 1, 44, 37
//2, 48480, Note_off_c, 1, 44, 0
//2, 48576, Note_on_c, 1, 44, 37
//2, 48672, Note_off_c, 1, 44, 0
//2, 48768, Note_on_c, 1, 42, 37
//2, 49056, Note_off_c, 1, 42, 0
//2, 49056, Note_on_c, 1, 44, 37
//2, 49152, Note_off_c, 1, 44, 0
//2, 49248, Note_on_c, 1, 44, 37
//2, 49344, Note_off_c, 1, 44, 0
//2, 49344, Note_on_c, 1, 42, 37
//2, 49536, Note_off_c, 1, 42, 0
//2, 49536, Note_on_c, 1, 41, 37
//2, 49728, Note_off_c, 1, 41, 0
//2, 49728, Note_on_c, 1, 39, 37
//2, 49824, Note_off_c, 1, 39, 0
//2, 49824, Note_on_c, 1, 39, 37
//2, 49920, Note_off_c, 1, 39, 0
//2, 49920, Note_on_c, 1, 44, 37
//2, 50016, Note_off_c, 1, 44, 0
//2, 50016, Note_on_c, 1, 44, 37
//2, 50112, Note_off_c, 1, 44, 0
//2, 50112, Note_on_c, 1, 44, 37
//2, 50208, Note_off_c, 1, 44, 0
//2, 50304, Note_on_c, 1, 42, 37
//2, 50496, Note_off_c, 1, 42, 0
//2, 50496, Note_on_c, 1, 44, 37
//2, 50592, Note_off_c, 1, 44, 0
//2, 50592, Note_on_c, 1, 44, 37
//2, 50688, Note_off_c, 1, 44, 0
//2, 50784, Note_on_c, 1, 44, 37
//2, 50880, Note_off_c, 1, 44, 0
//2, 50880, Note_on_c, 1, 42, 37
//2, 50976, Note_off_c, 1, 42, 0
//2, 51072, Note_on_c, 1, 41, 37
//2, 51168, Note_off_c, 1, 41, 0
//2, 51264, Note_on_c, 1, 39, 37
//2, 51360, Note_off_c, 1, 39, 0
//2, 51360, Note_on_c, 1, 39, 37
//2, 51456, Note_off_c, 1, 39, 0
//2, 51456, Note_on_c, 1, 44, 37
//2, 51552, Note_off_c, 1, 44, 0
//2, 51552, Note_on_c, 1, 44, 37
//2, 51648, Note_off_c, 1, 44, 0
//2, 51648, Note_on_c, 1, 44, 37
//2, 51744, Note_off_c, 1, 44, 0
//2, 51840, Note_on_c, 1, 42, 37
//2, 52032, Note_off_c, 1, 42, 0
//2, 52032, Note_on_c, 1, 44, 37
//2, 52128, Note_off_c, 1, 44, 0
//2, 52128, Note_on_c, 1, 44, 37
//2, 52224, Note_off_c, 1, 44, 0
//2, 52320, Note_on_c, 1, 44, 37
//2, 52416, Note_off_c, 1, 44, 0
//2, 52416, Note_on_c, 1, 42, 37
//2, 52512, Note_off_c, 1, 42, 0
//2, 52608, Note_on_c, 1, 41, 37
//2, 52704, Note_off_c, 1, 41, 0
//2, 52800, Note_on_c, 1, 39, 37
//2, 52896, Note_off_c, 1, 39, 0
//2, 52896, Note_on_c, 1, 39, 37
//2, 52992, Note_off_c, 1, 39, 0
//2, 52992, Note_on_c, 1, 44, 37
//2, 53088, Note_off_c, 1, 44, 0
//2, 53088, Note_on_c, 1, 44, 37
//2, 53184, Note_off_c, 1, 44, 0
//2, 53184, Note_on_c, 1, 44, 37
//2, 53280, Note_off_c, 1, 44, 0
//2, 53376, Note_on_c, 1, 42, 37
//2, 53568, Note_off_c, 1, 42, 0
//2, 53568, Note_on_c, 1, 44, 37
//2, 53664, Note_off_c, 1, 44, 0
//2, 53664, Note_on_c, 1, 44, 37
//2, 53760, Note_off_c, 1, 44, 0
//2, 53856, Note_on_c, 1, 44, 37
//2, 53952, Note_off_c, 1, 44, 0
//2, 53952, Note_on_c, 1, 42, 37
//2, 54048, Note_off_c, 1, 42, 0
//2, 54144, Note_on_c, 1, 41, 37
//2, 54240, Note_off_c, 1, 41, 0
//2, 54336, Note_on_c, 1, 56, 37
//2, 54336, Note_on_c, 1, 49, 37
//2, 54528, Note_off_c, 1, 49, 0
//2, 54528, Note_off_c, 1, 56, 0
//2, 54528, Note_on_c, 1, 58, 37
//2, 54528, Note_on_c, 1, 51, 37
//2, 54624, Note_off_c, 1, 51, 0
//2, 54624, Note_off_c, 1, 58, 0
//2, 54720, Note_on_c, 1, 58, 37
//2, 54720, Note_on_c, 1, 51, 37
//2, 54816, Note_off_c, 1, 51, 0
//2, 54816, Note_off_c, 1, 58, 0
//2, 54912, Note_on_c, 1, 56, 37
//2, 54912, Note_on_c, 1, 49, 37
//2, 55008, Note_off_c, 1, 49, 0
//2, 55008, Note_off_c, 1, 56, 0
//2, 55104, Note_on_c, 1, 49, 37
//2, 55104, Note_on_c, 1, 44, 37
//2, 55200, Note_off_c, 1, 44, 0
//2, 55200, Note_off_c, 1, 49, 0
//2, 55200, Note_on_c, 1, 58, 37
//2, 55200, Note_on_c, 1, 51, 37
//2, 55296, Note_off_c, 1, 51, 0
//2, 55296, Note_off_c, 1, 58, 0
//2, 55392, Note_on_c, 1, 58, 37
//2, 55392, Note_on_c, 1, 51, 37
//2, 55488, Note_off_c, 1, 51, 0
//2, 55488, Note_off_c, 1, 58, 0
//2, 55488, Note_on_c, 1, 56, 37
//2, 55488, Note_on_c, 1, 49, 37
//2, 55584, Note_off_c, 1, 49, 0
//2, 55584, Note_off_c, 1, 56, 0
//2, 55680, Note_on_c, 1, 55, 37
//2, 55680, Note_on_c, 1, 48, 37
//2, 55776, Note_off_c, 1, 48, 0
//2, 55776, Note_off_c, 1, 55, 0
//2, 55872, Note_on_c, 1, 53, 37
//2, 55872, Note_on_c, 1, 46, 37
//2, 55968, Note_off_c, 1, 46, 0
//2, 55968, Note_off_c, 1, 53, 0
//2, 56064, Note_on_c, 1, 58, 37
//2, 56064, Note_on_c, 1, 51, 37
//2, 56160, Note_off_c, 1, 51, 0
//2, 56160, Note_off_c, 1, 58, 0
//2, 56256, Note_on_c, 1, 58, 37
//2, 56256, Note_on_c, 1, 51, 37
//2, 56352, Note_off_c, 1, 51, 0
//2, 56352, Note_off_c, 1, 58, 0
//2, 56448, Note_on_c, 1, 56, 37
//2, 56448, Note_on_c, 1, 49, 37
//2, 56544, Note_off_c, 1, 49, 0
//2, 56544, Note_off_c, 1, 56, 0
//2, 56640, Note_on_c, 1, 49, 37
//2, 56640, Note_on_c, 1, 44, 37
//2, 56736, Note_off_c, 1, 44, 0
//2, 56736, Note_off_c, 1, 49, 0
//2, 56736, Note_on_c, 1, 58, 37
//2, 56736, Note_on_c, 1, 51, 37
//2, 56832, Note_off_c, 1, 51, 0
//2, 56832, Note_off_c, 1, 58, 0
//2, 56928, Note_on_c, 1, 58, 37
//2, 56928, Note_on_c, 1, 51, 37
//2, 57024, Note_off_c, 1, 51, 0
//2, 57024, Note_off_c, 1, 58, 0
//2, 57024, Note_on_c, 1, 56, 37
//2, 57024, Note_on_c, 1, 49, 37
//2, 57120, Note_off_c, 1, 49, 0
//2, 57120, Note_off_c, 1, 56, 0
//2, 57216, Note_on_c, 1, 55, 37
//2, 57216, Note_on_c, 1, 48, 37
//2, 57312, Note_off_c, 1, 48, 0
//2, 57312, Note_off_c, 1, 55, 0
//2, 57408, Note_on_c, 1, 53, 37
//2, 57408, Note_on_c, 1, 46, 37
//2, 57504, Note_off_c, 1, 46, 0
//2, 57504, Note_off_c, 1, 53, 0
//2, 57600, Note_on_c, 1, 58, 37
//2, 57600, Note_on_c, 1, 51, 37
//2, 57696, Note_off_c, 1, 51, 0
//2, 57696, Note_off_c, 1, 58, 0
//2, 57792, Note_on_c, 1, 58, 37
//2, 57792, Note_on_c, 1, 51, 37
//2, 57888, Note_off_c, 1, 51, 0
//2, 57888, Note_off_c, 1, 58, 0
//2, 57984, Note_on_c, 1, 56, 37
//2, 57984, Note_on_c, 1, 49, 37
//2, 58080, Note_off_c, 1, 49, 0
//2, 58080, Note_off_c, 1, 56, 0
//2, 58176, Note_on_c, 1, 49, 37
//2, 58176, Note_on_c, 1, 44, 37
//2, 58272, Note_off_c, 1, 44, 0
//2, 58272, Note_off_c, 1, 49, 0
//2, 58272, Note_on_c, 1, 58, 37
//2, 58272, Note_on_c, 1, 51, 37
//2, 58368, Note_off_c, 1, 51, 0
//2, 58368, Note_off_c, 1, 58, 0
//2, 58464, Note_on_c, 1, 58, 37
//2, 58464, Note_on_c, 1, 51, 37
//2, 58560, Note_off_c, 1, 51, 0
//2, 58560, Note_off_c, 1, 58, 0
//2, 58560, Note_on_c, 1, 56, 37
//2, 58560, Note_on_c, 1, 49, 37
//2, 58656, Note_off_c, 1, 49, 0
//2, 58656, Note_off_c, 1, 56, 0
//2, 58752, Note_on_c, 1, 55, 37
//2, 58752, Note_on_c, 1, 48, 37
//2, 58848, Note_off_c, 1, 48, 0
//2, 58848, Note_off_c, 1, 55, 0
//2, 58944, Note_on_c, 1, 53, 37
//2, 58944, Note_on_c, 1, 46, 37
//2, 59040, Note_off_c, 1, 46, 0
//2, 59040, Note_off_c, 1, 53, 0
//2, 59136, Note_on_c, 1, 51, 37
//2, 59136, Note_on_c, 1, 58, 37
//2, 59232, Note_off_c, 1, 51, 0
//2, 59328, Note_off_c, 1, 58, 0
//2, 59328, Note_on_c, 1, 51, 37
//2, 59328, Note_on_c, 1, 58, 37
//2, 59424, Note_off_c, 1, 51, 0
//2, 59520, Note_off_c, 1, 58, 0
//2, 59520, Note_on_c, 1, 49, 37
//2, 59520, Note_on_c, 1, 56, 37
//2, 59616, Note_off_c, 1, 49, 0
//2, 59712, Note_off_c, 1, 56, 0
//2, 59712, Note_on_c, 1, 49, 37
//2, 59712, Note_on_c, 1, 44, 37
//2, 59808, Note_off_c, 1, 44, 0
//2, 59808, Note_off_c, 1, 49, 0
//2, 59808, Note_on_c, 1, 51, 37
//2, 59808, Note_on_c, 1, 58, 37
//2, 59904, Note_off_c, 1, 51, 0
//2, 60000, Note_off_c, 1, 58, 0
//2, 60000, Note_on_c, 1, 51, 37
//2, 60000, Note_on_c, 1, 58, 37
//2, 60096, Note_off_c, 1, 58, 0
//2, 60096, Note_off_c, 1, 51, 0
//2, 60096, Note_on_c, 1, 49, 37
//2, 60096, Note_on_c, 1, 56, 37
//2, 60192, Note_off_c, 1, 49, 0
//2, 60288, Note_off_c, 1, 56, 0
//2, 60288, Note_on_c, 1, 51, 37
//2, 60288, Note_on_c, 1, 58, 37
//2, 60384, Note_off_c, 1, 51, 0
//2, 60480, Note_off_c, 1, 58, 0
//2, 60480, Note_on_c, 1, 44, 37
//2, 60576, Note_off_c, 1, 44, 0
//2, 60672, Note_on_c, 1, 54, 37
//2, 60672, Note_on_c, 1, 47, 37
//2, 61056, Note_off_c, 1, 47, 0
//2, 61056, Note_off_c, 1, 54, 0
//2, 61056, Note_on_c, 1, 46, 37
//2, 61440, Note_off_c, 1, 46, 0
//2, 61440, Note_on_c, 1, 44, 37
//2, 61824, Note_off_c, 1, 44, 0
//2, 61824, Note_on_c, 1, 42, 37
//2, 62208, Note_off_c, 1, 42, 0
//2, 62208, Note_on_c, 1, 56, 37
//2, 62208, Note_on_c, 1, 49, 37
//2, 62592, Note_off_c, 1, 49, 0
//2, 62592, Note_off_c, 1, 56, 0
//2, 62592, Note_on_c, 1, 48, 37
//2, 62976, Note_off_c, 1, 48, 0
//2, 62976, Note_on_c, 1, 46, 37
//2, 63360, Note_off_c, 1, 46, 0
//2, 63360, Note_on_c, 1, 44, 37
//2, 63744, Note_off_c, 1, 44, 0
//2, 63744, Note_on_c, 1, 58, 37
//2, 63744, Note_on_c, 1, 51, 37
//2, 63936, Note_off_c, 1, 51, 0
//2, 63936, Note_off_c, 1, 58, 0
//2, 63936, Note_on_c, 1, 58, 37
//2, 63936, Note_on_c, 1, 51, 37
//2, 64128, Note_off_c, 1, 51, 0
//2, 64128, Note_off_c, 1, 58, 0
//2, 64128, Note_on_c, 1, 49, 37
//2, 64320, Note_off_c, 1, 49, 0
//2, 64320, Note_on_c, 1, 45, 37
//2, 64416, Note_off_c, 1, 45, 0
//2, 64416, Note_on_c, 1, 45, 37
//2, 64512, Note_off_c, 1, 45, 0
//2, 64608, Note_on_c, 1, 44, 37
//2, 64704, Note_off_c, 1, 44, 0
//2, 64704, Note_on_c, 1, 44, 37
//2, 64896, Note_off_c, 1, 44, 0
//2, 64896, Note_on_c, 1, 42, 37
//2, 65088, Note_off_c, 1, 42, 0
//2, 65088, Note_on_c, 1, 39, 37
//2, 65184, Note_off_c, 1, 39, 0
//2, 65280, Note_on_c, 1, 51, 37
//2, 65376, Note_off_c, 1, 51, 0
//2, 65472, Note_on_c, 1, 51, 37
//2, 65568, Note_off_c, 1, 51, 0
//2, 65664, Note_on_c, 1, 49, 37
//2, 65760, Note_off_c, 1, 49, 0
//2, 65856, Note_on_c, 1, 45, 37
//2, 65952, Note_off_c, 1, 45, 0
//2, 65952, Note_on_c, 1, 45, 37
//2, 66048, Note_off_c, 1, 45, 0
//2, 66144, Note_on_c, 1, 44, 37
//2, 66240, Note_off_c, 1, 44, 0
//2, 66240, Note_on_c, 1, 44, 37
//2, 66336, Note_off_c, 1, 44, 0
//2, 66432, Note_on_c, 1, 42, 37
//2, 66528, Note_off_c, 1, 42, 0
//2, 66624, Note_on_c, 1, 39, 37
//2, 66720, Note_off_c, 1, 39, 0
//2, 66816, Note_on_c, 1, 51, 37
//2, 66912, Note_off_c, 1, 51, 0
//2, 67008, Note_on_c, 1, 51, 37
//2, 67104, Note_off_c, 1, 51, 0
//2, 67200, Note_on_c, 1, 49, 37
//2, 67296, Note_off_c, 1, 49, 0
//2, 67392, Note_on_c, 1, 45, 37
//2, 67488, Note_off_c, 1, 45, 0
//2, 67488, Note_on_c, 1, 45, 37
//2, 67584, Note_off_c, 1, 45, 0
//2, 67680, Note_on_c, 1, 44, 37
//2, 67776, Note_off_c, 1, 44, 0
//2, 67776, Note_on_c, 1, 44, 37
//2, 67872, Note_off_c, 1, 44, 0
//2, 67968, Note_on_c, 1, 42, 37
//2, 68064, Note_off_c, 1, 42, 0
//2, 68160, Note_on_c, 1, 39, 37
//2, 68256, Note_off_c, 1, 39, 0
//2, 68352, Note_on_c, 1, 68, 37
//2, 68352, Note_on_c, 1, 63, 37
//2, 68448, Note_off_c, 1, 63, 0
//2, 68448, Note_off_c, 1, 68, 0
//2, 68448, Note_on_c, 1, 70, 37
//2, 68448, Note_on_c, 1, 65, 37
//2, 68544, Note_off_c, 1, 65, 0
//2, 68544, Note_off_c, 1, 70, 0
//2, 68544, Note_on_c, 1, 68, 37
//2, 68544, Note_on_c, 1, 63, 37
//2, 68640, Note_off_c, 1, 63, 0
//2, 68640, Note_off_c, 1, 68, 0
//2, 68640, Note_on_c, 1, 70, 37
//2, 68640, Note_on_c, 1, 65, 37
//2, 68928, Note_off_c, 1, 65, 0
//2, 68928, Note_off_c, 1, 70, 0
//2, 68928, Note_on_c, 1, 63, 37
//2, 69024, Note_off_c, 1, 63, 0
//2, 69024, Note_on_c, 1, 65, 37
//2, 69504, Note_off_c, 1, 65, 0
//2, 69504, Note_on_c, 1, 58, 37
//2, 69504, Note_on_c, 1, 54, 37
//2, 69600, Note_off_c, 1, 54, 0
//2, 69600, Note_off_c, 1, 58, 0
//2, 69888, Note_on_c, 1, 39, 37
//2, 70080, Note_on_c, 1, 66, 37
//2, 70080, Note_on_c, 1, 60, 37
//2, 70272, Note_off_c, 1, 39, 0
//2, 70272, Note_off_c, 1, 60, 0
//2, 70272, Note_off_c, 1, 66, 0
//2, 70272, Note_on_c, 1, 51, 37
//2, 70272, Note_on_c, 1, 39, 37
//2, 70464, Note_off_c, 1, 39, 0
//2, 70464, Note_off_c, 1, 51, 0
//2, 70464, Note_on_c, 1, 56, 37
//2, 70464, Note_on_c, 1, 39, 37
//2, 70560, Note_off_c, 1, 56, 0
//2, 70560, Note_on_c, 1, 58, 37
//2, 70656, Note_off_c, 1, 39, 0
//2, 70656, Note_off_c, 1, 58, 0
//2, 70656, Note_on_c, 1, 39, 37
//2, 70656, Note_on_c, 1, 61, 37
//2, 70752, Note_off_c, 1, 61, 0
//2, 70752, Note_on_c, 1, 63, 37
//2, 70848, Note_off_c, 1, 39, 0
//2, 70848, Note_off_c, 1, 63, 0
//2, 70848, Note_on_c, 1, 58, 37
//2, 70848, Note_on_c, 1, 51, 37
//2, 71040, Note_off_c, 1, 51, 0
//2, 71040, Note_off_c, 1, 58, 0
//2, 71040, Note_on_c, 1, 39, 37
//2, 71232, Note_off_c, 1, 39, 0
//2, 71232, Note_on_c, 1, 58, 37
//2, 71232, Note_on_c, 1, 51, 37
//2, 71424, Note_off_c, 1, 51, 0
//2, 71424, Note_off_c, 1, 58, 0
//2, 71424, Note_on_c, 1, 58, 37
//2, 71424, Note_on_c, 1, 51, 37
//2, 71616, Note_off_c, 1, 51, 0
//2, 71616, Note_off_c, 1, 58, 0
//2, 71616, Note_on_c, 1, 54, 37
//2, 71616, Note_on_c, 1, 66, 37
//2, 71808, Note_off_c, 1, 66, 0
//2, 71808, Note_off_c, 1, 54, 0
//2, 71808, Note_on_c, 1, 49, 37
//2, 71808, Note_on_c, 1, 63, 37
//2, 72000, Note_off_c, 1, 63, 0
//2, 72000, Note_off_c, 1, 49, 0
//2, 72000, Note_on_c, 1, 39, 37
//2, 72000, Note_on_c, 1, 54, 37
//2, 72000, Note_on_c, 1, 49, 37
//2, 72096, Note_off_c, 1, 49, 0
//2, 72096, Note_off_c, 1, 54, 0
//2, 72096, Note_off_c, 1, 39, 0
//2, 72096, Note_on_c, 1, 46, 37
//2, 72192, Note_off_c, 1, 46, 0
//2, 72192, Note_on_c, 1, 54, 37
//2, 72192, Note_on_c, 1, 49, 37
//2, 72288, Note_off_c, 1, 49, 0
//2, 72288, Note_off_c, 1, 54, 0
//2, 72384, Note_on_c, 1, 73, 37
//2, 72384, Note_on_c, 1, 70, 37
//2, 72768, Note_on_c, 1, 47, 37
//2, 72864, Note_off_c, 1, 47, 0
//2, 72864, Note_on_c, 1, 51, 37
//2, 72960, Note_off_c, 1, 70, 0
//2, 72960, Note_off_c, 1, 73, 0
//2, 72960, Note_off_c, 1, 51, 0
//2, 73152, Note_on_c, 1, 66, 37
//2, 73152, Note_on_c, 1, 60, 37
//2, 73344, Note_off_c, 1, 60, 0
//2, 73344, Note_off_c, 1, 66, 0
//2, 73344, Note_on_c, 1, 58, 37
//2, 73344, Note_on_c, 1, 51, 37
//2, 73536, Note_off_c, 1, 51, 0
//2, 73536, Note_off_c, 1, 58, 0
//2, 73536, Note_on_c, 1, 56, 37
//2, 73536, Note_on_c, 1, 49, 37
//2, 73632, Note_off_c, 1, 49, 0
//2, 73632, Note_off_c, 1, 56, 0
//2, 73632, Note_on_c, 1, 58, 37
//2, 73632, Note_on_c, 1, 51, 37
//2, 73728, Note_off_c, 1, 51, 0
//2, 73728, Note_off_c, 1, 58, 0
//2, 73920, Note_on_c, 1, 58, 37
//2, 73920, Note_on_c, 1, 51, 37
//2, 74112, Note_off_c, 1, 51, 0
//2, 74112, Note_off_c, 1, 58, 0
//2, 74304, Note_on_c, 1, 58, 37
//2, 74304, Note_on_c, 1, 51, 37
//2, 74496, Note_off_c, 1, 51, 0
//2, 74496, Note_off_c, 1, 58, 0
//2, 74496, Note_on_c, 1, 58, 37
//2, 74592, Note_on_c, 1, 70, 37
//2, 74688, Note_off_c, 1, 58, 0
//2, 74688, Note_off_c, 1, 70, 0
//2, 74688, Note_on_c, 1, 72, 37
//2, 74688, Note_on_c, 1, 69, 37
//2, 74880, Note_off_c, 1, 69, 0
//2, 74880, Note_off_c, 1, 72, 0
//2, 74880, Note_on_c, 1, 73, 37
//2, 74880, Note_on_c, 1, 70, 37
//2, 75072, Note_on_c, 1, 63, 37
//2, 75072, Note_on_c, 1, 58, 37
//2, 75168, Note_off_c, 1, 58, 0
//2, 75168, Note_off_c, 1, 63, 0
//2, 75168, Note_on_c, 1, 66, 37
//2, 75168, Note_on_c, 1, 60, 37
//2, 75264, Note_off_c, 1, 70, 0
//2, 75264, Note_off_c, 1, 73, 0
//2, 75360, Note_off_c, 1, 60, 0
//2, 75360, Note_off_c, 1, 66, 0
//2, 75360, Note_on_c, 1, 63, 37
//2, 75456, Note_off_c, 1, 63, 0
//2, 75456, Note_on_c, 1, 78, 37
//2, 75456, Note_on_c, 1, 73, 37
//2, 75648, Note_off_c, 1, 73, 0
//2, 75648, Note_off_c, 1, 78, 0
//2, 75648, Note_on_c, 1, 78, 37
//2, 75648, Note_on_c, 1, 73, 37
//2, 75840, Note_off_c, 1, 73, 0
//2, 75840, Note_off_c, 1, 78, 0
//2, 75840, Note_on_c, 1, 60, 37
//2, 76032, Note_off_c, 1, 60, 0
//2, 76032, Note_on_c, 1, 72, 37
//2, 76160, Note_off_c, 1, 72, 0
//2, 76160, Note_on_c, 1, 75, 37
//2, 76289, Note_on_c, 1, 82, 37
//2, 76289, Note_off_c, 1, 75, 0
//2, 77185, Note_off_c, 1, 82, 0
//2, 77185, Note_on_c, 1, 82, 37
//2, 77313, Note_off_c, 1, 82, 0
//2, 77313, Note_on_c, 1, 81, 37
//2, 77442, Note_on_c, 1, 80, 37
//2, 77442, Note_off_c, 1, 81, 0
//2, 78146, Note_off_c, 1, 80, 0
//2, 78146, Note_on_c, 1, 78, 37
//2, 78242, Note_off_c, 1, 78, 0
//2, 78242, Note_on_c, 1, 79, 37
//2, 78338, Note_off_c, 1, 79, 0
//2, 78338, Note_on_c, 1, 73, 37
//2, 79010, Note_off_c, 1, 73, 0
//2, 79010, Note_on_c, 1, 73, 37
//2, 79234, Note_off_c, 1, 73, 0
//2, 79234, Note_on_c, 1, 73, 37
//2, 79363, Note_on_c, 1, 70, 27
//2, 79363, Note_off_c, 1, 73, 0
//2, 79491, Note_off_c, 1, 70, 0
//2, 79491, Note_on_c, 1, 68, 37
//2, 79683, Note_off_c, 1, 68, 0
//2, 79683, Note_on_c, 1, 68, 37
//2, 79971, Note_off_c, 1, 68, 0
//2, 79971, Note_on_c, 1, 66, 37
//2, 80067, Note_off_c, 1, 66, 0
//2, 80067, Note_on_c, 1, 68, 37
//2, 80163, Note_off_c, 1, 68, 0
//2, 80163, Note_on_c, 1, 66, 37
//2, 80259, Note_off_c, 1, 66, 0
//2, 80259, Note_on_c, 1, 63, 37
//2, 80355, Note_off_c, 1, 63, 0
//2, 80355, Note_on_c, 1, 63, 37
//2, 80451, Note_off_c, 1, 63, 0
//2, 80451, Note_on_c, 1, 57, 37
//2, 80547, Note_off_c, 1, 57, 0
//2, 80547, Note_on_c, 1, 57, 37
//2, 80643, Note_off_c, 1, 57, 0
//2, 80643, Note_on_c, 1, 56, 37
//2, 80643, Note_on_c, 1, 56, 37
//2, 80835, Note_off_c, 1, 56, 0
//2, 80835, Note_on_c, 1, 62, 37
//2, 80931, Note_off_c, 1, 56, 0
//2, 80931, Note_on_c, 1, 54, 37
//2, 81027, Note_off_c, 1, 62, 0
//2, 81027, Note_off_c, 1, 54, 0
//2, 81027, Note_on_c, 1, 51, 37
//2, 81027, Note_on_c, 1, 51, 37
//2, 81027, Note_on_c, 1, 63, 37
//2, 81219, Note_off_c, 1, 63, 0
//2, 81219, Note_off_c, 1, 51, 0
//2, 81219, Note_off_c, 1, 51, 0
//2, 81219, Note_on_c, 1, 62, 37
//2, 81795, Note_off_c, 1, 62, 0
//2, 81795, Note_on_c, 1, 60, 37
//2, 82179, Note_off_c, 1, 60, 0
//2, 82179, Note_on_c, 1, 44, 37
//2, 82275, Note_off_c, 1, 44, 0
//2, 82371, Note_on_c, 1, 44, 37
//2, 82467, Note_off_c, 1, 44, 0
//2, 82563, Note_on_c, 1, 42, 37
//2, 82851, Note_off_c, 1, 42, 0
//2, 82851, Note_on_c, 1, 44, 37
//2, 82947, Note_off_c, 1, 44, 0
//2, 83043, Note_on_c, 1, 44, 37
//2, 83139, Note_off_c, 1, 44, 0
//2, 83139, Note_on_c, 1, 42, 37
//2, 83331, Note_off_c, 1, 42, 0
//2, 83331, Note_on_c, 1, 41, 37
//2, 83523, Note_off_c, 1, 41, 0
//2, 83523, Note_on_c, 1, 39, 37
//2, 83619, Note_off_c, 1, 39, 0
//2, 83619, Note_on_c, 1, 39, 37
//2, 83715, Note_off_c, 1, 39, 0
//2, 83715, Note_on_c, 1, 44, 37
//2, 83811, Note_off_c, 1, 44, 0
//2, 83811, Note_on_c, 1, 44, 37
//2, 83907, Note_off_c, 1, 44, 0
//2, 83907, Note_on_c, 1, 44, 37
//2, 84003, Note_off_c, 1, 44, 0
//2, 84099, Note_on_c, 1, 42, 37
//2, 84291, Note_off_c, 1, 42, 0
//2, 84291, Note_on_c, 1, 44, 37
//2, 84387, Note_off_c, 1, 44, 0
//2, 84387, Note_on_c, 1, 44, 37
//2, 84483, Note_off_c, 1, 44, 0
//2, 84579, Note_on_c, 1, 44, 37
//2, 84675, Note_off_c, 1, 44, 0
//2, 84675, Note_on_c, 1, 42, 37
//2, 84771, Note_off_c, 1, 42, 0
//2, 84867, Note_on_c, 1, 41, 37
//2, 84963, Note_off_c, 1, 41, 0
//2, 85059, Note_on_c, 1, 39, 37
//2, 85155, Note_off_c, 1, 39, 0
//2, 85155, Note_on_c, 1, 39, 37
//2, 85251, Note_off_c, 1, 39, 0
//2, 85251, Note_on_c, 1, 44, 37
//2, 85347, Note_off_c, 1, 44, 0
//2, 85347, Note_on_c, 1, 44, 37
//2, 85443, Note_off_c, 1, 44, 0
//2, 85443, Note_on_c, 1, 44, 37
//2, 85539, Note_off_c, 1, 44, 0
//2, 85635, Note_on_c, 1, 42, 37
//2, 85827, Note_off_c, 1, 42, 0
//2, 85827, Note_on_c, 1, 44, 37
//2, 85923, Note_off_c, 1, 44, 0
//2, 85923, Note_on_c, 1, 44, 37
//2, 86019, Note_off_c, 1, 44, 0
//2, 86115, Note_on_c, 1, 44, 37
//2, 86211, Note_off_c, 1, 44, 0
//2, 86211, Note_on_c, 1, 42, 37
//2, 86307, Note_off_c, 1, 42, 0
//2, 86403, Note_on_c, 1, 41, 37
//2, 86499, Note_off_c, 1, 41, 0
//2, 86595, Note_on_c, 1, 39, 37
//2, 86691, Note_off_c, 1, 39, 0
//2, 86691, Note_on_c, 1, 39, 37
//2, 86787, Note_off_c, 1, 39, 0
//2, 86787, Note_on_c, 1, 44, 37
//2, 86883, Note_off_c, 1, 44, 0
//2, 86883, Note_on_c, 1, 44, 37
//2, 86979, Note_off_c, 1, 44, 0
//2, 86979, Note_on_c, 1, 44, 37
//2, 87075, Note_off_c, 1, 44, 0
//2, 87171, Note_on_c, 1, 42, 37
//2, 87363, Note_off_c, 1, 42, 0
//2, 87363, Note_on_c, 1, 44, 37
//2, 87459, Note_off_c, 1, 44, 0
//2, 87459, Note_on_c, 1, 44, 37
//2, 87555, Note_off_c, 1, 44, 0
//2, 87651, Note_on_c, 1, 44, 37
//2, 87747, Note_off_c, 1, 44, 0
//2, 87747, Note_on_c, 1, 42, 37
//2, 87843, Note_off_c, 1, 42, 0
//2, 87939, Note_on_c, 1, 41, 37
//2, 88035, Note_off_c, 1, 41, 0
//2, 88131, Note_on_c, 1, 61, 37
//2, 88131, Note_on_c, 1, 56, 37
//2, 88131, Note_on_c, 1, 49, 37
//2, 88323, Note_off_c, 1, 49, 0
//2, 88323, Note_off_c, 1, 56, 0
//2, 88323, Note_off_c, 1, 61, 0
//2, 88323, Note_on_c, 1, 58, 37
//2, 88323, Note_on_c, 1, 51, 37
//2, 88419, Note_off_c, 1, 51, 0
//2, 88419, Note_off_c, 1, 58, 0
//2, 88515, Note_on_c, 1, 58, 37
//2, 88515, Note_on_c, 1, 51, 37
//2, 88611, Note_off_c, 1, 51, 0
//2, 88611, Note_off_c, 1, 58, 0
//2, 88707, Note_on_c, 1, 56, 37
//2, 88707, Note_on_c, 1, 49, 37
//2, 88803, Note_off_c, 1, 49, 0
//2, 88803, Note_off_c, 1, 56, 0
//2, 88899, Note_on_c, 1, 54, 37
//2, 88899, Note_on_c, 1, 49, 37
//2, 88899, Note_on_c, 1, 44, 37
//2, 88995, Note_off_c, 1, 44, 0
//2, 88995, Note_off_c, 1, 49, 0
//2, 88995, Note_off_c, 1, 54, 0
//2, 88995, Note_on_c, 1, 58, 37
//2, 88995, Note_on_c, 1, 51, 37
//2, 89091, Note_off_c, 1, 51, 0
//2, 89091, Note_off_c, 1, 58, 0
//2, 89187, Note_on_c, 1, 58, 37
//2, 89187, Note_on_c, 1, 51, 37
//2, 89283, Note_off_c, 1, 51, 0
//2, 89283, Note_off_c, 1, 58, 0
//2, 89283, Note_on_c, 1, 56, 37
//2, 89283, Note_on_c, 1, 49, 37
//2, 89379, Note_off_c, 1, 49, 0
//2, 89379, Note_off_c, 1, 56, 0
//2, 89475, Note_on_c, 1, 55, 37
//2, 89475, Note_on_c, 1, 48, 37
//2, 89571, Note_off_c, 1, 48, 0
//2, 89571, Note_off_c, 1, 55, 0
//2, 89667, Note_on_c, 1, 53, 37
//2, 89667, Note_on_c, 1, 46, 37
//2, 89763, Note_off_c, 1, 46, 0
//2, 89763, Note_off_c, 1, 53, 0
//2, 89859, Note_on_c, 1, 58, 37
//2, 89859, Note_on_c, 1, 51, 37
//2, 89955, Note_off_c, 1, 51, 0
//2, 89955, Note_off_c, 1, 58, 0
//2, 90051, Note_on_c, 1, 58, 37
//2, 90051, Note_on_c, 1, 51, 37
//2, 90147, Note_off_c, 1, 51, 0
//2, 90147, Note_off_c, 1, 58, 0
//2, 90243, Note_on_c, 1, 56, 37
//2, 90243, Note_on_c, 1, 49, 37
//2, 90339, Note_off_c, 1, 49, 0
//2, 90339, Note_off_c, 1, 56, 0
//2, 90435, Note_on_c, 1, 49, 37
//2, 90435, Note_on_c, 1, 44, 37
//2, 90531, Note_off_c, 1, 44, 0
//2, 90531, Note_off_c, 1, 49, 0
//2, 90531, Note_on_c, 1, 58, 37
//2, 90531, Note_on_c, 1, 51, 37
//2, 90627, Note_off_c, 1, 51, 0
//2, 90627, Note_off_c, 1, 58, 0
//2, 90723, Note_on_c, 1, 58, 37
//2, 90723, Note_on_c, 1, 51, 37
//2, 90819, Note_off_c, 1, 51, 0
//2, 90819, Note_off_c, 1, 58, 0
//2, 90819, Note_on_c, 1, 56, 37
//2, 90819, Note_on_c, 1, 49, 37
//2, 90915, Note_off_c, 1, 49, 0
//2, 90915, Note_off_c, 1, 56, 0
//2, 91011, Note_on_c, 1, 55, 37
//2, 91011, Note_on_c, 1, 48, 37
//2, 91107, Note_off_c, 1, 48, 0
//2, 91107, Note_off_c, 1, 55, 0
//2, 91203, Note_on_c, 1, 53, 37
//2, 91203, Note_on_c, 1, 46, 37
//2, 91299, Note_off_c, 1, 46, 0
//2, 91299, Note_off_c, 1, 53, 0
//2, 91395, Note_on_c, 1, 58, 37
//2, 91395, Note_on_c, 1, 51, 37
//2, 91491, Note_off_c, 1, 51, 0
//2, 91491, Note_off_c, 1, 58, 0
//2, 91587, Note_on_c, 1, 58, 37
//2, 91587, Note_on_c, 1, 51, 37
//2, 91683, Note_off_c, 1, 51, 0
//2, 91683, Note_off_c, 1, 58, 0
//2, 91779, Note_on_c, 1, 56, 37
//2, 91779, Note_on_c, 1, 49, 37
//2, 91875, Note_off_c, 1, 49, 0
//2, 91875, Note_off_c, 1, 56, 0
//2, 91971, Note_on_c, 1, 49, 37
//2, 91971, Note_on_c, 1, 44, 37
//2, 92067, Note_off_c, 1, 44, 0
//2, 92067, Note_off_c, 1, 49, 0
//2, 92067, Note_on_c, 1, 58, 37
//2, 92067, Note_on_c, 1, 51, 37
//2, 92163, Note_off_c, 1, 51, 0
//2, 92163, Note_off_c, 1, 58, 0
//2, 92259, Note_on_c, 1, 58, 37
//2, 92259, Note_on_c, 1, 51, 37
//2, 92355, Note_off_c, 1, 51, 0
//2, 92355, Note_off_c, 1, 58, 0
//2, 92355, Note_on_c, 1, 56, 37
//2, 92355, Note_on_c, 1, 49, 37
//2, 92451, Note_off_c, 1, 49, 0
//2, 92451, Note_off_c, 1, 56, 0
//2, 92547, Note_on_c, 1, 55, 37
//2, 92547, Note_on_c, 1, 48, 37
//2, 92643, Note_off_c, 1, 48, 0
//2, 92643, Note_off_c, 1, 55, 0
//2, 92739, Note_on_c, 1, 53, 37
//2, 92739, Note_on_c, 1, 46, 37
//2, 92835, Note_off_c, 1, 46, 0
//2, 92835, Note_off_c, 1, 53, 0
//2, 92931, Note_on_c, 1, 51, 37
//2, 92931, Note_on_c, 1, 58, 37
//2, 93027, Note_off_c, 1, 51, 0
//2, 93123, Note_off_c, 1, 58, 0
//2, 93123, Note_on_c, 1, 51, 37
//2, 93123, Note_on_c, 1, 58, 37
//2, 93219, Note_off_c, 1, 51, 0
//2, 93315, Note_off_c, 1, 58, 0
//2, 93315, Note_on_c, 1, 49, 37
//2, 93315, Note_on_c, 1, 56, 37
//2, 93411, Note_off_c, 1, 49, 0
//2, 93507, Note_off_c, 1, 56, 0
//2, 93507, Note_on_c, 1, 49, 37
//2, 93507, Note_on_c, 1, 44, 37
//2, 93603, Note_off_c, 1, 44, 0
//2, 93603, Note_off_c, 1, 49, 0
//2, 93603, Note_on_c, 1, 51, 37
//2, 93603, Note_on_c, 1, 58, 37
//2, 93699, Note_off_c, 1, 51, 0
//2, 93795, Note_off_c, 1, 58, 0
//2, 93795, Note_on_c, 1, 51, 37
//2, 93795, Note_on_c, 1, 58, 37
//2, 93891, Note_off_c, 1, 58, 0
//2, 93891, Note_off_c, 1, 51, 0
//2, 93891, Note_on_c, 1, 49, 37
//2, 93891, Note_on_c, 1, 56, 37
//2, 93987, Note_off_c, 1, 49, 0
//2, 94083, Note_off_c, 1, 56, 0
//2, 94083, Note_on_c, 1, 51, 37
//2, 94083, Note_on_c, 1, 58, 37
//2, 94179, Note_off_c, 1, 51, 0
//2, 94275, Note_off_c, 1, 58, 0
//2, 94275, Note_on_c, 1, 44, 37
//2, 94371, Note_off_c, 1, 44, 0
//2, 94467, Note_on_c, 1, 54, 37
//2, 94467, Note_on_c, 1, 47, 37
//2, 94851, Note_off_c, 1, 47, 0
//2, 94851, Note_off_c, 1, 54, 0
//2, 94851, Note_on_c, 1, 46, 37
//2, 95235, Note_off_c, 1, 46, 0
//2, 95235, Note_on_c, 1, 44, 37
//2, 95619, Note_off_c, 1, 44, 0
//2, 95619, Note_on_c, 1, 42, 37
//2, 96003, Note_off_c, 1, 42, 0
//2, 96003, Note_on_c, 1, 56, 37
//2, 96003, Note_on_c, 1, 49, 37
//2, 96387, Note_off_c, 1, 49, 0
//2, 96387, Note_off_c, 1, 56, 0
//2, 96387, Note_on_c, 1, 48, 37
//2, 96771, Note_off_c, 1, 48, 0
//2, 96771, Note_on_c, 1, 46, 37
//2, 97155, Note_off_c, 1, 46, 0
//2, 97155, Note_on_c, 1, 44, 37
//2, 97539, Note_off_c, 1, 44, 0
//2, 97539, Note_on_c, 1, 58, 37
//2, 97539, Note_on_c, 1, 51, 37
//2, 97731, Note_off_c, 1, 51, 0
//2, 97731, Note_off_c, 1, 58, 0
//2, 97731, Note_on_c, 1, 58, 37
//2, 97731, Note_on_c, 1, 51, 37
//2, 97923, Note_off_c, 1, 51, 0
//2, 97923, Note_off_c, 1, 58, 0
//2, 97923, Note_on_c, 1, 49, 37
//2, 98115, Note_off_c, 1, 49, 0
//2, 98115, Note_on_c, 1, 45, 37
//2, 98211, Note_off_c, 1, 45, 0
//2, 98211, Note_on_c, 1, 45, 37
//2, 98307, Note_off_c, 1, 45, 0
//2, 98403, Note_on_c, 1, 44, 37
//2, 98499, Note_off_c, 1, 44, 0
//2, 98499, Note_on_c, 1, 44, 37
//2, 98691, Note_off_c, 1, 44, 0
//2, 98691, Note_on_c, 1, 42, 37
//2, 98883, Note_off_c, 1, 42, 0
//2, 98883, Note_on_c, 1, 39, 37
//2, 98979, Note_off_c, 1, 39, 0
//2, 99075, Note_on_c, 1, 51, 37
//2, 99267, Note_off_c, 1, 51, 0
//2, 99267, Note_on_c, 1, 51, 37
//2, 99459, Note_off_c, 1, 51, 0
//2, 99459, Note_on_c, 1, 49, 37
//2, 99651, Note_off_c, 1, 49, 0
//2, 99651, Note_on_c, 1, 45, 37
//2, 99747, Note_off_c, 1, 45, 0
//2, 99747, Note_on_c, 1, 45, 37
//2, 99939, Note_off_c, 1, 45, 0
//2, 99939, Note_on_c, 1, 44, 37
//2, 100035, Note_off_c, 1, 44, 0
//2, 100035, Note_on_c, 1, 44, 37
//2, 100227, Note_off_c, 1, 44, 0
//2, 100227, Note_on_c, 1, 42, 37
//2, 100419, Note_off_c, 1, 42, 0
//2, 100419, Note_on_c, 1, 39, 37
//2, 100515, Note_off_c, 1, 39, 0
//2, 100611, Note_on_c, 1, 51, 37
//2, 100803, Note_off_c, 1, 51, 0
//2, 100803, Note_on_c, 1, 51, 37
//2, 100995, Note_off_c, 1, 51, 0
//2, 100995, Note_on_c, 1, 49, 37
//2, 101187, Note_off_c, 1, 49, 0
//2, 101187, Note_on_c, 1, 45, 37
//2, 101283, Note_off_c, 1, 45, 0
//2, 101283, Note_on_c, 1, 45, 37
//2, 101475, Note_off_c, 1, 45, 0
//2, 101475, Note_on_c, 1, 44, 37
//2, 101571, Note_off_c, 1, 44, 0
//2, 101571, Note_on_c, 1, 44, 37
//2, 101763, Note_off_c, 1, 44, 0
//2, 101763, Note_on_c, 1, 42, 37
//2, 101955, Note_off_c, 1, 42, 0
//2, 101955, Note_on_c, 1, 39, 37
//2, 102051, Note_off_c, 1, 39, 0
//2, 102147, Note_on_c, 1, 51, 37
//2, 102435, Note_off_c, 1, 51, 0
//2, 102435, Note_on_c, 1, 49, 37
//2, 102723, Note_off_c, 1, 49, 0
//2, 102723, Note_on_c, 1, 45, 37
//2, 103107, Note_off_c, 1, 45, 0
//2, 103107, Note_on_c, 1, 44, 37
//2, 103299, Note_off_c, 1, 44, 0
//2, 103299, Note_on_c, 1, 42, 37
//2, 103395, Note_off_c, 1, 42, 0
//2, 103491, Note_on_c, 1, 43, 37
//2, 103587, Note_off_c, 1, 43, 0
//2, 103683, Note_on_c, 1, 49, 37
//2, 103683, Note_on_c, 1, 44, 37
//2, 105219, Note_off_c, 1, 44, 0
//2, 105219, Note_off_c, 1, 49, 0
//2, 105219, Note_on_c, 1, 66, 37
//2, 105219, Note_on_c, 1, 61, 37
//2, 106755, Note_off_c, 1, 61, 0
//2, 106755, Note_off_c, 1, 66, 0
//2, 106755, Note_on_c, 1, 65, 37
//2, 106755, Note_on_c, 1, 61, 37
//2, 108291, Note_off_c, 1, 61, 0
//2, 108291, Note_off_c, 1, 65, 0
//2, 108291, Note_on_c, 1, 46, 37
//2, 108291, Note_on_c, 1, 42, 37
//2, 109827, Note_off_c, 1, 42, 0
//2, 109827, Note_off_c, 1, 46, 0
//2, 109827, Note_on_c, 1, 49, 37
//2, 109827, Note_on_c, 1, 44, 37
//2, 111363, Note_off_c, 1, 44, 0
//2, 111363, Note_off_c, 1, 49, 0
//2, 111363, Note_on_c, 1, 46, 37
//2, 111363, Note_on_c, 1, 42, 37
//2, 112899, Note_off_c, 1, 42, 0
//2, 112899, Note_off_c, 1, 46, 0
//2, 112899, Note_on_c, 1, 49, 37
//2, 112899, Note_on_c, 1, 44, 37
//2, 114051, Note_off_c, 1, 44, 0
//2, 114051, Note_off_c, 1, 49, 0
//2, 114051, Note_on_c, 1, 59, 37
//2, 114051, Note_on_c, 1, 56, 37
//2, 114051, Note_on_c, 1, 49, 37
//2, 114435, Note_off_c, 1, 49, 0
//2, 114435, Note_off_c, 1, 56, 0
//2, 114435, Note_off_c, 1, 59, 0
//2, 114435, Note_on_c, 1, 49, 37
//2, 114435, Note_on_c, 1, 42, 37
//2, 114627, Note_off_c, 1, 42, 0
//2, 114627, Note_off_c, 1, 49, 0
//2, 114627, Note_on_c, 1, 49, 37
//2, 114627, Note_on_c, 1, 42, 37
//2, 114819, Note_off_c, 1, 42, 0
//2, 114819, Note_off_c, 1, 49, 0
//2, 114819, Note_on_c, 1, 49, 37
//2, 114819, Note_on_c, 1, 42, 37
//2, 115011, Note_off_c, 1, 42, 0
//2, 115011, Note_off_c, 1, 49, 0
//2, 115011, Note_on_c, 1, 49, 37
//2, 115011, Note_on_c, 1, 42, 37
//2, 115203, Note_off_c, 1, 42, 0
//2, 115203, Note_off_c, 1, 49, 0
//2, 115203, Note_on_c, 1, 49, 37
//2, 115203, Note_on_c, 1, 42, 37
//2, 115587, Note_off_c, 1, 42, 0
//2, 115587, Note_off_c, 1, 49, 0
//2, 115587, Note_on_c, 1, 40, 37
//2, 115587, Note_on_c, 1, 64, 37
//2, 115971, Note_off_c, 1, 64, 0
//2, 115971, Note_off_c, 1, 40, 0
//2, 115971, Note_on_c, 1, 42, 37
//2, 115971, Note_on_c, 1, 49, 37
//2, 116163, Note_off_c, 1, 49, 0
//2, 116163, Note_off_c, 1, 42, 0
//2, 116163, Note_on_c, 1, 42, 37
//2, 116163, Note_on_c, 1, 49, 37
//2, 116355, Note_off_c, 1, 49, 0
//2, 116355, Note_off_c, 1, 42, 0
//2, 116355, Note_on_c, 1, 42, 37
//2, 116355, Note_on_c, 1, 49, 37
//2, 116547, Note_off_c, 1, 49, 0
//2, 116547, Note_off_c, 1, 42, 0
//2, 116547, Note_on_c, 1, 42, 37
//2, 116547, Note_on_c, 1, 49, 37
//2, 116643, Note_off_c, 1, 42, 0
//2, 116643, Note_on_c, 1, 42, 37
//2, 116739, Note_off_c, 1, 49, 0
//2, 116739, Note_off_c, 1, 42, 0
//2, 116739, Note_on_c, 1, 42, 37
//2, 116739, Note_on_c, 1, 49, 37
//2, 117123, Note_off_c, 1, 49, 0
//2, 117123, Note_off_c, 1, 42, 0
//2, 117123, Note_on_c, 1, 40, 37
//2, 117123, Note_on_c, 1, 64, 37
//2, 117507, Note_off_c, 1, 64, 0
//2, 117507, Note_off_c, 1, 40, 0
//2, 117507, Note_on_c, 1, 49, 37
//2, 117507, Note_on_c, 1, 42, 37
//2, 117699, Note_off_c, 1, 42, 0
//2, 117699, Note_off_c, 1, 49, 0
//2, 117699, Note_on_c, 1, 49, 37
//2, 117699, Note_on_c, 1, 42, 37
//2, 117891, Note_off_c, 1, 42, 0
//2, 117891, Note_off_c, 1, 49, 0
//2, 117891, Note_on_c, 1, 49, 37
//2, 117891, Note_on_c, 1, 42, 37
//2, 118083, Note_off_c, 1, 42, 0
//2, 118083, Note_off_c, 1, 49, 0
//2, 118083, Note_on_c, 1, 49, 37
//2, 118083, Note_on_c, 1, 42, 37
//2, 118275, Note_off_c, 1, 42, 0
//2, 118275, Note_off_c, 1, 49, 0
//2, 118275, Note_on_c, 1, 49, 37
//2, 118275, Note_on_c, 1, 42, 37
//2, 118659, Note_off_c, 1, 42, 0
//2, 118659, Note_off_c, 1, 49, 0
//2, 118659, Note_on_c, 1, 64, 37
//2, 118659, Note_on_c, 1, 40, 37
//2, 119043, Note_off_c, 1, 40, 0
//2, 119043, Note_off_c, 1, 64, 0
//2, 119043, Note_on_c, 1, 51, 37
//2, 119043, Note_on_c, 1, 44, 37
//2, 119235, Note_off_c, 1, 44, 0
//2, 119235, Note_off_c, 1, 51, 0
//2, 119235, Note_on_c, 1, 51, 37
//2, 119235, Note_on_c, 1, 44, 37
//2, 119427, Note_off_c, 1, 44, 0
//2, 119427, Note_off_c, 1, 51, 0
//2, 119427, Note_on_c, 1, 51, 37
//2, 119427, Note_on_c, 1, 44, 37
//2, 119619, Note_off_c, 1, 44, 0
//2, 119619, Note_off_c, 1, 51, 0
//2, 119619, Note_on_c, 1, 51, 37
//2, 119619, Note_on_c, 1, 44, 37
//2, 119811, Note_off_c, 1, 44, 0
//2, 119811, Note_off_c, 1, 51, 0
//2, 119811, Note_on_c, 1, 51, 37
//2, 119811, Note_on_c, 1, 44, 37
//2, 120003, Note_off_c, 1, 44, 0
//2, 120003, Note_off_c, 1, 51, 0
//2, 120003, Note_on_c, 1, 51, 37
//2, 120003, Note_on_c, 1, 44, 37
//2, 120195, Note_off_c, 1, 44, 0
//2, 120195, Note_off_c, 1, 51, 0
//2, 120195, Note_on_c, 1, 51, 37
//2, 120195, Note_on_c, 1, 44, 37
//2, 120387, Note_off_c, 1, 44, 0
//2, 120387, Note_off_c, 1, 51, 0
//2, 120387, Note_on_c, 1, 66, 37
//2, 120387, Note_on_c, 1, 42, 37
//2, 120579, Note_off_c, 1, 42, 0
//2, 120579, Note_off_c, 1, 66, 0
//2, 120579, Note_on_c, 1, 39, 37
//2, 120579, Note_on_c, 1, 73, 37
//2, 120771, Note_off_c, 1, 39, 0
//2, 121155, Note_on_c, 1, 39, 37
//2, 121251, Note_off_c, 1, 39, 0
//2, 121347, Note_on_c, 1, 39, 37
//2, 121443, Note_off_c, 1, 39, 0
//2, 121539, Note_off_c, 1, 73, 0
//2, 121539, Note_on_c, 1, 73, 37
//2, 121731, Note_on_c, 1, 39, 37
//2, 121827, Note_off_c, 1, 73, 0
//2, 121827, Note_off_c, 1, 39, 0
//2, 121827, Note_on_c, 1, 73, 37
//2, 122115, Note_off_c, 1, 73, 0
//2, 122115, Note_on_c, 1, 73, 37
//2, 122691, Note_on_c, 1, 39, 37
//2, 122787, Note_off_c, 1, 39, 0
//2, 122883, Note_on_c, 1, 39, 37
//2, 122979, Note_off_c, 1, 39, 0
//2, 123075, Note_off_c, 1, 73, 0
//2, 123075, Note_on_c, 1, 70, 37
//2, 123267, Note_off_c, 1, 70, 0
//2, 123267, Note_on_c, 1, 68, 37
//2, 123459, Note_off_c, 1, 68, 0
//2, 123459, Note_on_c, 1, 73, 37
//2, 123651, Note_off_c, 1, 73, 0
//2, 123651, Note_on_c, 1, 39, 37
//2, 123651, Note_on_c, 1, 68, 37
//2, 123843, Note_off_c, 1, 39, 0
//2, 124035, Note_off_c, 1, 68, 0
//2, 124035, Note_on_c, 1, 70, 37
//2, 124227, Note_off_c, 1, 70, 0
//2, 124227, Note_on_c, 1, 39, 37
//2, 124227, Note_on_c, 1, 75, 37
//2, 124323, Note_off_c, 1, 39, 0
//2, 124419, Note_off_c, 1, 75, 0
//2, 124419, Note_on_c, 1, 39, 37
//2, 124419, Note_on_c, 1, 73, 37
//2, 124515, Note_off_c, 1, 39, 0
//2, 124611, Note_off_c, 1, 73, 0
//2, 124611, Note_on_c, 1, 68, 37
//2, 124803, Note_on_c, 1, 39, 37
//2, 124899, Note_off_c, 1, 39, 0
//2, 124995, Note_off_c, 1, 68, 0
//2, 124995, Note_on_c, 1, 66, 37
//2, 125187, Note_off_c, 1, 66, 0
//2, 125187, Note_on_c, 1, 73, 37
//2, 125571, Note_off_c, 1, 73, 0
//2, 125571, Note_on_c, 1, 78, 37
//2, 125763, Note_off_c, 1, 78, 0
//2, 125763, Note_on_c, 1, 39, 37
//2, 125763, Note_on_c, 1, 73, 37
//2, 125859, Note_off_c, 1, 39, 0
//2, 125955, Note_on_c, 1, 39, 37
//2, 126051, Note_off_c, 1, 39, 0
//2, 126723, Note_off_c, 1, 73, 0
//2, 126723, Note_on_c, 1, 48, 37
//2, 127491, Note_off_c, 1, 48, 0
//2, 127491, Note_on_c, 1, 51, 37
//2, 127683, Note_off_c, 1, 51, 0
//2, 127683, Note_on_c, 1, 48, 37
//2, 127875, Note_off_c, 1, 48, 0
//2, 127875, Note_on_c, 1, 55, 37
//2, 128067, Note_off_c, 1, 55, 0
//2, 128067, Note_on_c, 1, 53, 37
//2, 128643, Note_off_c, 1, 53, 0
//2, 128835, Note_on_c, 1, 51, 37
//2, 128931, Note_off_c, 1, 51, 0
//2, 128931, Note_on_c, 1, 51, 37
//2, 129027, Note_off_c, 1, 51, 0
//2, 129027, Note_on_c, 1, 53, 37
//2, 129123, Note_off_c, 1, 53, 0
//2, 129123, Note_on_c, 1, 55, 37
//2, 129219, Note_off_c, 1, 55, 0
//2, 129219, Note_on_c, 1, 58, 37
//2, 129411, Note_off_c, 1, 58, 0
//2, 129411, Note_on_c, 1, 58, 37
//2, 129603, Note_off_c, 1, 58, 0
//2, 129603, Note_on_c, 1, 60, 37
//2, 130563, Note_off_c, 1, 60, 0
//2, 130563, Note_on_c, 1, 63, 37
//2, 130755, Note_off_c, 1, 63, 0
//2, 130755, Note_on_c, 1, 60, 37
//2, 130947, Note_off_c, 1, 60, 0
//2, 130947, Note_on_c, 1, 67, 37
//2, 131331, Note_off_c, 1, 67, 0
//2, 131331, Note_on_c, 1, 65, 37
//2, 132099, Note_off_c, 1, 65, 0
//2, 132099, Note_on_c, 1, 65, 37
//2, 132291, Note_off_c, 1, 65, 0
//2, 132291, Note_on_c, 1, 63, 37
//2, 132387, Note_off_c, 1, 63, 0
//2, 132387, Note_on_c, 1, 60, 37
//2, 132483, Note_off_c, 1, 60, 0
//2, 132483, Note_on_c, 1, 63, 37
//2, 132675, Note_off_c, 1, 63, 0
//2, 132675, Note_on_c, 1, 72, 37
//2, 132675, Note_on_c, 1, 67, 37
//2, 133635, Note_off_c, 1, 67, 0
//2, 133635, Note_off_c, 1, 72, 0
//2, 133635, Note_on_c, 1, 70, 37
//2, 133635, Note_on_c, 1, 67, 37
//2, 133827, Note_off_c, 1, 67, 0
//2, 133827, Note_off_c, 1, 70, 0
//2, 133827, Note_on_c, 1, 65, 37
//2, 133923, Note_off_c, 1, 65, 0
//2, 133923, Note_on_c, 1, 63, 27
//2, 134019, Note_off_c, 1, 63, 0
//2, 134019, Note_on_c, 1, 60, 37
//2, 134211, Note_off_c, 1, 60, 0
//2, 134211, Note_on_c, 1, 58, 37
//2, 135171, Note_off_c, 1, 58, 0
//2, 135171, Note_on_c, 1, 58, 37
//2, 135267, Note_off_c, 1, 58, 0
//2, 135267, Note_on_c, 1, 60, 37
//2, 135363, Note_off_c, 1, 60, 0
//2, 135363, Note_on_c, 1, 63, 37
//2, 135555, Note_off_c, 1, 63, 0
//2, 135555, Note_on_c, 1, 60, 37
//2, 135747, Note_off_c, 1, 60, 0
//2, 135747, Note_on_c, 1, 63, 37
//2, 135747, Note_on_c, 1, 63, 37
//2, 136323, Note_off_c, 1, 63, 0
//2, 136323, Note_off_c, 1, 63, 0
//2, 136323, Note_on_c, 1, 63, 37
//2, 136323, Note_on_c, 1, 61, 37
//2, 136707, Note_off_c, 1, 61, 0
//2, 136707, Note_off_c, 1, 63, 0
//2, 136707, Note_on_c, 1, 63, 37
//2, 136707, Note_on_c, 1, 60, 37
//2, 137091, Note_off_c, 1, 60, 0
//2, 137091, Note_off_c, 1, 63, 0
//2, 137091, Note_on_c, 1, 63, 37
//2, 137091, Note_on_c, 1, 58, 37
//2, 137475, Note_off_c, 1, 58, 0
//2, 137475, Note_off_c, 1, 63, 0
//2, 137475, Note_on_c, 1, 60, 37
//2, 137475, Note_on_c, 1, 56, 37
//2, 137859, Note_off_c, 1, 56, 0
//2, 137859, Note_off_c, 1, 60, 0
//2, 137859, Note_on_c, 1, 60, 37
//2, 137859, Note_on_c, 1, 56, 37
//2, 138051, Note_off_c, 1, 56, 0
//2, 138051, Note_off_c, 1, 60, 0
//2, 138051, Note_on_c, 1, 58, 37
//2, 138051, Note_on_c, 1, 54, 37
//2, 138243, Note_off_c, 1, 54, 0
//2, 138243, Note_off_c, 1, 58, 0
//2, 138243, Note_on_c, 1, 55, 27
//2, 138435, Note_off_c, 1, 55, 0
//2, 138435, Note_on_c, 1, 58, 37
//2, 138435, Note_on_c, 1, 55, 37
//2, 138627, Note_off_c, 1, 55, 0
//2, 138627, Note_off_c, 1, 58, 0
//2, 138627, Note_on_c, 1, 56, 37
//2, 138819, Note_off_c, 1, 56, 0
//2, 138819, Note_on_c, 1, 58, 37
//2, 138819, Note_on_c, 1, 53, 37
//2, 138819, Note_on_c, 1, 58, 37
//2, 138819, Note_on_c, 1, 58, 37
//2, 139203, Note_off_c, 1, 58, 0
//2, 139203, Note_off_c, 1, 58, 0
//2, 139203, Note_off_c, 1, 53, 0
//2, 139203, Note_off_c, 1, 58, 0
//2, 139203, Note_on_c, 1, 58, 37
//2, 139203, Note_on_c, 1, 58, 37
//2, 139395, Note_off_c, 1, 58, 0
//2, 139395, Note_off_c, 1, 58, 0
//2, 139395, Note_on_c, 1, 58, 37
//2, 139395, Note_on_c, 1, 56, 37
//2, 139587, Note_off_c, 1, 56, 0
//2, 139587, Note_off_c, 1, 58, 0
//2, 139587, Note_on_c, 1, 58, 37
//2, 139587, Note_on_c, 1, 56, 37
//2, 139779, Note_off_c, 1, 56, 0
//2, 139779, Note_off_c, 1, 58, 0
//2, 139779, Note_on_c, 1, 58, 37
//2, 139779, Note_on_c, 1, 55, 37
//2, 140163, Note_off_c, 1, 55, 0
//2, 140163, Note_off_c, 1, 58, 0
//2, 140163, Note_on_c, 1, 60, 37
//2, 140163, Note_on_c, 1, 56, 37
//2, 140355, Note_off_c, 1, 56, 0
//2, 140355, Note_off_c, 1, 60, 0
//2, 140355, Note_on_c, 1, 62, 37
//2, 140355, Note_on_c, 1, 58, 37
//2, 140547, Note_off_c, 1, 58, 0
//2, 140547, Note_off_c, 1, 62, 0
//2, 140547, Note_on_c, 1, 51, 37
//2, 140835, Note_off_c, 1, 51, 0
//2, 140835, Note_on_c, 1, 49, 37
//2, 140931, Note_off_c, 1, 49, 0
//2, 140931, Note_on_c, 1, 51, 37
//2, 141123, Note_off_c, 1, 51, 0
//2, 141123, Note_on_c, 1, 49, 37
//2, 141315, Note_off_c, 1, 49, 0
//2, 141315, Note_on_c, 1, 46, 37
//2, 141411, Note_off_c, 1, 46, 0
//2, 141699, Note_on_c, 1, 75, 37
//2, 141891, Note_off_c, 1, 75, 0
//2, 141891, Note_on_c, 1, 75, 37
//2, 142467, Note_off_c, 1, 75, 0
//2, 142467, Note_on_c, 1, 73, 37
//2, 142851, Note_off_c, 1, 73, 0
//2, 142851, Note_on_c, 1, 70, 37
//2, 143235, Note_off_c, 1, 70, 0
//2, 143235, Note_on_c, 1, 68, 37
//2, 143619, Note_off_c, 1, 68, 0
//2, 143619, Note_on_c, 1, 68, 37
//2, 143811, Note_off_c, 1, 68, 0
//2, 143811, Note_on_c, 1, 66, 37
//2, 143907, Note_off_c, 1, 66, 0
//2, 143907, Note_on_c, 1, 63, 37
//2, 144003, Note_off_c, 1, 63, 0
//2, 144003, Note_on_c, 1, 61, 37
//2, 144099, Note_off_c, 1, 61, 0
//2, 144099, Note_on_c, 1, 63, 37
//2, 144195, Note_off_c, 1, 63, 0
//2, 144195, Note_on_c, 1, 72, 37
//2, 144195, Note_on_c, 1, 63, 37
//2, 144291, Note_off_c, 1, 63, 0
//2, 144291, Note_off_c, 1, 72, 0
//2, 144291, Note_on_c, 1, 70, 37
//2, 144291, Note_on_c, 1, 61, 37
//2, 144771, Note_off_c, 1, 61, 0
//2, 144771, Note_off_c, 1, 70, 0
//2, 144963, Note_on_c, 1, 60, 37
//2, 145155, Note_off_c, 1, 60, 0
//2, 145155, Note_on_c, 1, 61, 37
//2, 145539, Note_off_c, 1, 61, 0
//2, 145539, Note_on_c, 1, 58, 37
//2, 145731, Note_off_c, 1, 58, 0
//2, 145731, Note_on_c, 1, 56, 37
//2, 146307, Note_off_c, 1, 56, 0
//2, 146307, Note_on_c, 1, 53, 37
//2, 146499, Note_off_c, 1, 53, 0
//2, 146499, Note_on_c, 1, 49, 37
//2, 147075, Note_off_c, 1, 49, 0
//2, 147075, Note_on_c, 1, 46, 37
//2, 147267, Note_off_c, 1, 46, 0
//2, 147267, Note_on_c, 1, 43, 37
//2, 147843, Note_off_c, 1, 43, 0
//2, 147843, Note_on_c, 1, 39, 37
//2, 148227, Note_off_c, 1, 39, 0
//2, 148227, Note_on_c, 1, 41, 37
//2, 151299, Note_off_c, 1, 41, 0
//2, 151299, Note_on_c, 1, 62, 37
//2, 151299, Note_on_c, 1, 58, 37
//2, 152835, Note_off_c, 1, 58, 0
//2, 152835, Note_off_c, 1, 62, 0
//2, 152835, Note_on_c, 1, 62, 37
//2, 152835, Note_on_c, 1, 58, 37
//2, 154371, Note_off_c, 1, 58, 0
//2, 154371, Note_off_c, 1, 62, 0
//2, 154371, Note_on_c, 1, 54, 37
//2, 154371, Note_on_c, 1, 49, 37
//2, 154467, Note_off_c, 1, 49, 0
//2, 154467, Note_off_c, 1, 54, 0
//2, 157251, Note_on_c, 1, 85, 37
//2, 158019, Note_off_c, 1, 85, 0
//2, 158019, Note_on_c, 1, 82, 37
//2, 158211, Note_off_c, 1, 82, 0
//2, 158211, Note_on_c, 1, 82, 37
//2, 158403, Note_off_c, 1, 82, 0
//2, 158403, Note_on_c, 1, 81, 37
//2, 158979, Note_off_c, 1, 81, 0
//2, 158979, Note_on_c, 1, 79, 37
//2, 159363, Note_off_c, 1, 79, 0
//2, 159363, Note_on_c, 1, 44, 37
//2, 159363, Note_on_c, 1, 79, 37
//2, 159411, Note_on_c, 1, 49, 37
//2, 159459, Note_off_c, 1, 79, 0
//2, 159459, Note_off_c, 1, 44, 0
//2, 159459, Note_on_c, 1, 61, 37
//2, 159459, Note_on_c, 1, 77, 37
//2, 159507, Note_off_c, 1, 49, 0
//2, 159747, Note_off_c, 1, 77, 0
//2, 159747, Note_on_c, 1, 87, 37
//2, 160131, Note_off_c, 1, 61, 0
//2, 160131, Note_on_c, 1, 61, 37
//2, 160515, Note_off_c, 1, 87, 0
//2, 160515, Note_off_c, 1, 61, 0
//2, 160515, Note_on_c, 1, 58, 37
//2, 160515, Note_on_c, 1, 51, 37
//2, 160611, Note_off_c, 1, 51, 0
//2, 160611, Note_off_c, 1, 58, 0
//2, 160611, Note_on_c, 1, 58, 37
//2, 160611, Note_on_c, 1, 51, 37
//2, 160707, Note_off_c, 1, 51, 0
//2, 160707, Note_off_c, 1, 58, 0
//2, 160707, Note_on_c, 1, 58, 37
//2, 160707, Note_on_c, 1, 51, 37
//2, 160803, Note_off_c, 1, 51, 0
//2, 160803, Note_off_c, 1, 58, 0
//2, 160803, Note_on_c, 1, 58, 37
//2, 160803, Note_on_c, 1, 51, 37
//2, 160899, Note_off_c, 1, 51, 0
//2, 160899, Note_off_c, 1, 58, 0
//2, 160899, Note_on_c, 1, 57, 37
//2, 160899, Note_on_c, 1, 50, 37
//2, 160995, Note_off_c, 1, 50, 0
//2, 160995, Note_off_c, 1, 57, 0
//2, 160995, Note_on_c, 1, 57, 37
//2, 160995, Note_on_c, 1, 50, 37
//2, 161091, Note_off_c, 1, 50, 0
//2, 161091, Note_off_c, 1, 57, 0
//2, 161091, Note_on_c, 1, 57, 37
//2, 161091, Note_on_c, 1, 50, 37
//2, 161187, Note_off_c, 1, 50, 0
//2, 161187, Note_off_c, 1, 57, 0
//2, 161187, Note_on_c, 1, 56, 37
//2, 161187, Note_on_c, 1, 49, 37
//2, 161283, Note_off_c, 1, 49, 0
//2, 161283, Note_off_c, 1, 56, 0
//2, 161283, Note_on_c, 1, 56, 37
//2, 161283, Note_on_c, 1, 49, 37
//2, 161379, Note_off_c, 1, 49, 0
//2, 161379, Note_off_c, 1, 56, 0
//2, 161379, Note_on_c, 1, 56, 37
//2, 161379, Note_on_c, 1, 49, 37
//2, 161475, Note_off_c, 1, 49, 0
//2, 161475, Note_off_c, 1, 56, 0
//2, 161475, Note_on_c, 1, 55, 37
//2, 161475, Note_on_c, 1, 48, 37
//2, 161571, Note_off_c, 1, 48, 0
//2, 161571, Note_off_c, 1, 55, 0
//2, 161571, Note_on_c, 1, 55, 37
//2, 161571, Note_on_c, 1, 48, 37
//2, 161667, Note_off_c, 1, 48, 0
//2, 161667, Note_off_c, 1, 55, 0
//2, 161667, Note_on_c, 1, 54, 37
//2, 161667, Note_on_c, 1, 54, 37
//2, 161667, Note_on_c, 1, 47, 37
//2, 161763, Note_off_c, 1, 47, 0
//2, 161763, Note_off_c, 1, 54, 0
//2, 161763, Note_off_c, 1, 54, 0
//2, 161763, Note_on_c, 1, 54, 37
//2, 161763, Note_on_c, 1, 54, 37
//2, 161763, Note_on_c, 1, 47, 37
//2, 161859, Note_off_c, 1, 47, 0
//2, 161859, Note_off_c, 1, 54, 0
//2, 161859, Note_off_c, 1, 54, 0
//2, 161859, Note_on_c, 1, 53, 37
//2, 161859, Note_on_c, 1, 46, 37
//2, 161955, Note_off_c, 1, 46, 0
//2, 161955, Note_off_c, 1, 53, 0
//2, 161955, Note_on_c, 1, 53, 37
//2, 161955, Note_on_c, 1, 46, 37
//2, 162051, Note_off_c, 1, 46, 0
//2, 162051, Note_off_c, 1, 53, 0
//2, 162051, Note_on_c, 1, 58, 37
//2, 162051, Note_on_c, 1, 51, 37
//2, 162147, Note_off_c, 1, 51, 0
//2, 162147, Note_off_c, 1, 58, 0
//2, 162147, Note_on_c, 1, 58, 37
//2, 162147, Note_on_c, 1, 51, 37
//2, 162243, Note_off_c, 1, 51, 0
//2, 162243, Note_off_c, 1, 58, 0
//2, 162243, Note_on_c, 1, 58, 37
//2, 162243, Note_on_c, 1, 51, 37
//2, 162339, Note_off_c, 1, 51, 0
//2, 162339, Note_off_c, 1, 58, 0
//2, 162339, Note_on_c, 1, 58, 37
//2, 162339, Note_on_c, 1, 51, 37
//2, 162435, Note_off_c, 1, 51, 0
//2, 162435, Note_off_c, 1, 58, 0
//2, 162435, Note_on_c, 1, 57, 37
//2, 162435, Note_on_c, 1, 50, 37
//2, 162531, Note_off_c, 1, 50, 0
//2, 162531, Note_off_c, 1, 57, 0
//2, 162531, Note_on_c, 1, 57, 37
//2, 162531, Note_on_c, 1, 50, 37
//2, 162627, Note_off_c, 1, 50, 0
//2, 162627, Note_off_c, 1, 57, 0
//2, 162627, Note_on_c, 1, 57, 37
//2, 162627, Note_on_c, 1, 50, 37
//2, 162723, Note_off_c, 1, 50, 0
//2, 162723, Note_off_c, 1, 57, 0
//2, 162723, Note_on_c, 1, 56, 37
//2, 162723, Note_on_c, 1, 49, 37
//2, 162819, Note_off_c, 1, 49, 0
//2, 162819, Note_off_c, 1, 56, 0
//2, 162819, Note_on_c, 1, 56, 37
//2, 162819, Note_on_c, 1, 49, 37
//2, 162915, Note_off_c, 1, 49, 0
//2, 162915, Note_off_c, 1, 56, 0
//2, 162915, Note_on_c, 1, 56, 37
//2, 162915, Note_on_c, 1, 49, 37
//2, 163011, Note_off_c, 1, 49, 0
//2, 163011, Note_off_c, 1, 56, 0
//2, 163011, Note_on_c, 1, 48, 37
//2, 163011, Note_on_c, 1, 55, 37
//2, 163107, Note_off_c, 1, 55, 0
//2, 163107, Note_off_c, 1, 48, 0
//2, 163107, Note_on_c, 1, 48, 37
//2, 163107, Note_on_c, 1, 55, 37
//2, 163203, Note_off_c, 1, 55, 0
//2, 163203, Note_off_c, 1, 48, 0
//2, 163203, Note_on_c, 1, 47, 37
//2, 163203, Note_on_c, 1, 54, 37
//2, 163203, Note_on_c, 1, 54, 37
//2, 163299, Note_off_c, 1, 54, 0
//2, 163299, Note_off_c, 1, 54, 0
//2, 163299, Note_off_c, 1, 47, 0
//2, 163299, Note_on_c, 1, 47, 37
//2, 163299, Note_on_c, 1, 54, 37
//2, 163299, Note_on_c, 1, 54, 37
//2, 163395, Note_off_c, 1, 54, 0
//2, 163395, Note_off_c, 1, 54, 0
//2, 163395, Note_off_c, 1, 47, 0
//2, 163395, Note_on_c, 1, 53, 37
//2, 163395, Note_on_c, 1, 46, 37
//2, 163491, Note_off_c, 1, 46, 0
//2, 163491, Note_off_c, 1, 53, 0
//2, 163491, Note_on_c, 1, 53, 37
//2, 163491, Note_on_c, 1, 46, 37
//2, 163587, Note_off_c, 1, 46, 0
//2, 163587, Note_off_c, 1, 53, 0
//2, 163587, Note_on_c, 1, 58, 37
//2, 163587, Note_on_c, 1, 51, 37
//2, 163683, Note_off_c, 1, 51, 0
//2, 163683, Note_off_c, 1, 58, 0
//2, 163683, Note_on_c, 1, 58, 37
//2, 163683, Note_on_c, 1, 51, 37
//2, 163779, Note_off_c, 1, 51, 0
//2, 163779, Note_off_c, 1, 58, 0
//2, 163779, Note_on_c, 1, 58, 37
//2, 163779, Note_on_c, 1, 51, 37
//2, 163875, Note_off_c, 1, 51, 0
//2, 163875, Note_off_c, 1, 58, 0
//2, 163875, Note_on_c, 1, 58, 37
//2, 163875, Note_on_c, 1, 51, 37
//2, 163971, Note_off_c, 1, 51, 0
//2, 163971, Note_off_c, 1, 58, 0
//2, 163971, Note_on_c, 1, 57, 37
//2, 163971, Note_on_c, 1, 50, 37
//2, 164067, Note_off_c, 1, 50, 0
//2, 164067, Note_off_c, 1, 57, 0
//2, 164067, Note_on_c, 1, 57, 37
//2, 164067, Note_on_c, 1, 50, 37
//2, 164163, Note_off_c, 1, 50, 0
//2, 164163, Note_off_c, 1, 57, 0
//2, 164163, Note_on_c, 1, 57, 37
//2, 164163, Note_on_c, 1, 50, 37
//2, 164259, Note_off_c, 1, 50, 0
//2, 164259, Note_off_c, 1, 57, 0
//2, 164259, Note_on_c, 1, 56, 37
//2, 164259, Note_on_c, 1, 49, 37
//2, 164355, Note_off_c, 1, 49, 0
//2, 164355, Note_off_c, 1, 56, 0
//2, 164355, Note_on_c, 1, 56, 37
//2, 164355, Note_on_c, 1, 49, 37
//2, 164451, Note_off_c, 1, 49, 0
//2, 164451, Note_off_c, 1, 56, 0
//2, 164451, Note_on_c, 1, 56, 37
//2, 164451, Note_on_c, 1, 49, 37
//2, 164547, Note_off_c, 1, 49, 0
//2, 164547, Note_off_c, 1, 56, 0
//2, 164547, Note_on_c, 1, 55, 37
//2, 164547, Note_on_c, 1, 48, 37
//2, 164643, Note_off_c, 1, 48, 0
//2, 164643, Note_off_c, 1, 55, 0
//2, 164643, Note_on_c, 1, 55, 37
//2, 164643, Note_on_c, 1, 48, 37
//2, 164739, Note_off_c, 1, 48, 0
//2, 164739, Note_off_c, 1, 55, 0
//2, 164739, Note_on_c, 1, 54, 37
//2, 164739, Note_on_c, 1, 54, 37
//2, 164739, Note_on_c, 1, 47, 37
//2, 164835, Note_off_c, 1, 47, 0
//2, 164835, Note_off_c, 1, 54, 0
//2, 164835, Note_off_c, 1, 54, 0
//2, 164835, Note_on_c, 1, 54, 37
//2, 164835, Note_on_c, 1, 54, 37
//2, 164835, Note_on_c, 1, 47, 37
//2, 164931, Note_off_c, 1, 47, 0
//2, 164931, Note_off_c, 1, 54, 0
//2, 164931, Note_off_c, 1, 54, 0
//2, 164931, Note_on_c, 1, 53, 37
//2, 164931, Note_on_c, 1, 46, 37
//2, 165027, Note_off_c, 1, 46, 0
//2, 165027, Note_off_c, 1, 53, 0
//2, 165027, Note_on_c, 1, 53, 37
//2, 165027, Note_on_c, 1, 46, 37
//2, 165123, Note_off_c, 1, 46, 0
//2, 165123, Note_off_c, 1, 53, 0
//2, 165123, Note_on_c, 1, 58, 37
//2, 165123, Note_on_c, 1, 51, 37
//2, 165219, Note_off_c, 1, 51, 0
//2, 165219, Note_off_c, 1, 58, 0
//2, 165219, Note_on_c, 1, 58, 37
//2, 165219, Note_on_c, 1, 51, 37
//2, 165315, Note_off_c, 1, 51, 0
//2, 165315, Note_off_c, 1, 58, 0
//2, 165315, Note_on_c, 1, 58, 37
//2, 165315, Note_on_c, 1, 51, 37
//2, 165411, Note_off_c, 1, 51, 0
//2, 165411, Note_off_c, 1, 58, 0
//2, 165411, Note_on_c, 1, 58, 37
//2, 165411, Note_on_c, 1, 51, 37
//2, 165507, Note_off_c, 1, 51, 0
//2, 165507, Note_off_c, 1, 58, 0
//2, 165507, Note_on_c, 1, 57, 37
//2, 165507, Note_on_c, 1, 50, 37
//2, 165603, Note_off_c, 1, 50, 0
//2, 165603, Note_off_c, 1, 57, 0
//2, 165603, Note_on_c, 1, 57, 37
//2, 165603, Note_on_c, 1, 50, 37
//2, 165699, Note_off_c, 1, 50, 0
//2, 165699, Note_off_c, 1, 57, 0
//2, 165699, Note_on_c, 1, 57, 37
//2, 165699, Note_on_c, 1, 50, 37
//2, 165795, Note_off_c, 1, 50, 0
//2, 165795, Note_off_c, 1, 57, 0
//2, 165795, Note_on_c, 1, 56, 37
//2, 165795, Note_on_c, 1, 49, 37
//2, 165891, Note_off_c, 1, 49, 0
//2, 165891, Note_off_c, 1, 56, 0
//2, 165891, Note_on_c, 1, 56, 37
//2, 165891, Note_on_c, 1, 49, 37
//2, 165987, Note_off_c, 1, 49, 0
//2, 165987, Note_off_c, 1, 56, 0
//2, 165987, Note_on_c, 1, 56, 37
//2, 165987, Note_on_c, 1, 49, 37
//2, 166083, Note_off_c, 1, 49, 0
//2, 166083, Note_off_c, 1, 56, 0
//2, 166083, Note_on_c, 1, 55, 37
//2, 166083, Note_on_c, 1, 48, 37
//2, 166179, Note_off_c, 1, 48, 0
//2, 166179, Note_off_c, 1, 55, 0
//2, 166179, Note_on_c, 1, 55, 37
//2, 166179, Note_on_c, 1, 48, 37
//2, 166275, Note_off_c, 1, 48, 0
//2, 166275, Note_off_c, 1, 55, 0
//2, 166275, Note_on_c, 1, 54, 37
//2, 166275, Note_on_c, 1, 54, 37
//2, 166275, Note_on_c, 1, 47, 37
//2, 166371, Note_off_c, 1, 47, 0
//2, 166371, Note_off_c, 1, 54, 0
//2, 166371, Note_off_c, 1, 54, 0
//2, 166371, Note_on_c, 1, 54, 37
//2, 166371, Note_on_c, 1, 54, 37
//2, 166371, Note_on_c, 1, 47, 37
//2, 166467, Note_off_c, 1, 47, 0
//2, 166467, Note_off_c, 1, 54, 0
//2, 166467, Note_off_c, 1, 54, 0
//2, 166467, Note_on_c, 1, 53, 37
//2, 166467, Note_on_c, 1, 46, 37
//2, 166563, Note_off_c, 1, 46, 0
//2, 166563, Note_off_c, 1, 53, 0
//2, 166563, Note_on_c, 1, 53, 37
//2, 166563, Note_on_c, 1, 46, 37
//2, 166659, Note_off_c, 1, 46, 0
//2, 166659, Note_off_c, 1, 53, 0
//2, 166659, Note_on_c, 1, 58, 37
//2, 166659, Note_on_c, 1, 51, 37
//2, 166755, Note_off_c, 1, 51, 0
//2, 166755, Note_off_c, 1, 58, 0
//2, 166755, Note_on_c, 1, 58, 37
//2, 166755, Note_on_c, 1, 51, 37
//2, 166851, Note_off_c, 1, 51, 0
//2, 166851, Note_off_c, 1, 58, 0
//2, 166851, Note_on_c, 1, 58, 37
//2, 166851, Note_on_c, 1, 51, 37
//2, 166947, Note_off_c, 1, 51, 0
//2, 166947, Note_off_c, 1, 58, 0
//2, 166947, Note_on_c, 1, 58, 37
//2, 166947, Note_on_c, 1, 51, 37
//2, 167043, Note_off_c, 1, 51, 0
//2, 167043, Note_off_c, 1, 58, 0
//2, 167043, Note_on_c, 1, 57, 37
//2, 167043, Note_on_c, 1, 50, 37
//2, 167139, Note_off_c, 1, 50, 0
//2, 167139, Note_off_c, 1, 57, 0
//2, 167139, Note_on_c, 1, 57, 37
//2, 167139, Note_on_c, 1, 50, 37
//2, 167235, Note_off_c, 1, 50, 0
//2, 167235, Note_off_c, 1, 57, 0
//2, 167235, Note_on_c, 1, 57, 37
//2, 167235, Note_on_c, 1, 50, 37
//2, 167331, Note_off_c, 1, 50, 0
//2, 167331, Note_off_c, 1, 57, 0
//2, 167331, Note_on_c, 1, 56, 37
//2, 167331, Note_on_c, 1, 49, 37
//2, 167427, Note_off_c, 1, 49, 0
//2, 167427, Note_off_c, 1, 56, 0
//2, 167427, Note_on_c, 1, 56, 37
//2, 167427, Note_on_c, 1, 49, 37
//2, 167523, Note_off_c, 1, 49, 0
//2, 167523, Note_off_c, 1, 56, 0
//2, 167523, Note_on_c, 1, 56, 37
//2, 167523, Note_on_c, 1, 49, 37
//2, 167619, Note_off_c, 1, 49, 0
//2, 167619, Note_off_c, 1, 56, 0
//2, 167619, Note_on_c, 1, 55, 37
//2, 167619, Note_on_c, 1, 48, 37
//2, 167715, Note_off_c, 1, 48, 0
//2, 167715, Note_off_c, 1, 55, 0
//2, 167715, Note_on_c, 1, 55, 37
//2, 167715, Note_on_c, 1, 48, 37
//2, 167811, Note_off_c, 1, 48, 0
//2, 167811, Note_off_c, 1, 55, 0
//2, 167811, Note_on_c, 1, 47, 37
//2, 167811, Note_on_c, 1, 53, 37
//2, 167907, Note_off_c, 1, 53, 0
//2, 167907, Note_off_c, 1, 47, 0
//2, 167907, Note_on_c, 1, 54, 37
//2, 167907, Note_on_c, 1, 47, 37
//2, 168003, Note_off_c, 1, 47, 0
//2, 168003, Note_off_c, 1, 54, 0
//2, 168003, Note_on_c, 1, 53, 37
//2, 168003, Note_on_c, 1, 46, 37
//2, 168099, Note_off_c, 1, 46, 0
//2, 168099, Note_off_c, 1, 53, 0
//2, 168099, Note_on_c, 1, 53, 37
//2, 168099, Note_on_c, 1, 46, 37
//2, 168195, Note_off_c, 1, 46, 0
//2, 168195, Note_off_c, 1, 53, 0
//2, 168195, Note_on_c, 1, 58, 37
//2, 168195, Note_on_c, 1, 51, 37
//2, 168291, Note_off_c, 1, 51, 0
//2, 168291, Note_off_c, 1, 58, 0
//2, 168291, Note_on_c, 1, 58, 37
//2, 168291, Note_on_c, 1, 51, 37
//2, 168387, Note_off_c, 1, 51, 0
//2, 168387, Note_off_c, 1, 58, 0
//2, 168387, Note_on_c, 1, 58, 37
//2, 168387, Note_on_c, 1, 51, 37
//2, 168483, Note_off_c, 1, 51, 0
//2, 168483, Note_off_c, 1, 58, 0
//2, 168483, Note_on_c, 1, 58, 37
//2, 168483, Note_on_c, 1, 51, 37
//2, 168579, Note_off_c, 1, 51, 0
//2, 168579, Note_off_c, 1, 58, 0
//2, 168579, Note_on_c, 1, 57, 37
//2, 168579, Note_on_c, 1, 50, 37
//2, 168675, Note_off_c, 1, 50, 0
//2, 168675, Note_off_c, 1, 57, 0
//2, 168675, Note_on_c, 1, 57, 37
//2, 168675, Note_on_c, 1, 50, 37
//2, 168771, Note_off_c, 1, 50, 0
//2, 168771, Note_off_c, 1, 57, 0
//2, 168771, Note_on_c, 1, 57, 37
//2, 168771, Note_on_c, 1, 50, 37
//2, 168867, Note_off_c, 1, 50, 0
//2, 168867, Note_off_c, 1, 57, 0
//2, 168867, Note_on_c, 1, 56, 37
//2, 168867, Note_on_c, 1, 49, 37
//2, 168963, Note_off_c, 1, 49, 0
//2, 168963, Note_off_c, 1, 56, 0
//2, 168963, Note_on_c, 1, 56, 37
//2, 168963, Note_on_c, 1, 49, 37
//2, 169059, Note_off_c, 1, 49, 0
//2, 169059, Note_off_c, 1, 56, 0
//2, 169059, Note_on_c, 1, 56, 37
//2, 169059, Note_on_c, 1, 49, 37
//2, 169155, Note_off_c, 1, 49, 0
//2, 169155, Note_off_c, 1, 56, 0
//2, 169155, Note_on_c, 1, 55, 37
//2, 169155, Note_on_c, 1, 48, 37
//2, 169251, Note_off_c, 1, 48, 0
//2, 169251, Note_off_c, 1, 55, 0
//2, 169251, Note_on_c, 1, 55, 37
//2, 169251, Note_on_c, 1, 48, 37
//2, 169347, Note_off_c, 1, 48, 0
//2, 169347, Note_off_c, 1, 55, 0
//2, 169347, Note_on_c, 1, 47, 37
//2, 169347, Note_on_c, 1, 54, 37
//2, 169347, Note_on_c, 1, 54, 37
//2, 169443, Note_off_c, 1, 54, 0
//2, 169443, Note_off_c, 1, 54, 0
//2, 169443, Note_off_c, 1, 47, 0
//2, 169443, Note_on_c, 1, 54, 37
//2, 169443, Note_on_c, 1, 54, 37
//2, 169443, Note_on_c, 1, 47, 37
//2, 169539, Note_off_c, 1, 47, 0
//2, 169539, Note_off_c, 1, 54, 0
//2, 169539, Note_off_c, 1, 54, 0
//2, 169539, Note_on_c, 1, 53, 37
//2, 169539, Note_on_c, 1, 46, 37
//2, 169635, Note_off_c, 1, 46, 0
//2, 169635, Note_off_c, 1, 53, 0
//2, 169635, Note_on_c, 1, 53, 37
//2, 169635, Note_on_c, 1, 46, 37
//2, 169731, Note_off_c, 1, 46, 0
//2, 169731, Note_off_c, 1, 53, 0
//2, 169731, Note_on_c, 1, 58, 37
//2, 169731, Note_on_c, 1, 51, 37
//2, 169827, Note_off_c, 1, 51, 0
//2, 169827, Note_off_c, 1, 58, 0
//2, 169827, Note_on_c, 1, 58, 37
//2, 169827, Note_on_c, 1, 51, 37
//2, 169923, Note_off_c, 1, 51, 0
//2, 169923, Note_off_c, 1, 58, 0
//2, 169923, Note_on_c, 1, 58, 37
//2, 169923, Note_on_c, 1, 51, 37
//2, 170019, Note_off_c, 1, 51, 0
//2, 170019, Note_off_c, 1, 58, 0
//2, 170019, Note_on_c, 1, 58, 37
//2, 170019, Note_on_c, 1, 51, 37
//2, 170115, Note_off_c, 1, 51, 0
//2, 170115, Note_off_c, 1, 58, 0
//2, 170115, Note_on_c, 1, 57, 37
//2, 170115, Note_on_c, 1, 50, 37
//2, 170211, Note_off_c, 1, 50, 0
//2, 170211, Note_off_c, 1, 57, 0
//2, 170211, Note_on_c, 1, 57, 37
//2, 170211, Note_on_c, 1, 50, 37
//2, 170307, Note_off_c, 1, 50, 0
//2, 170307, Note_off_c, 1, 57, 0
//2, 170307, Note_on_c, 1, 57, 37
//2, 170307, Note_on_c, 1, 50, 37
//2, 170403, Note_off_c, 1, 50, 0
//2, 170403, Note_off_c, 1, 57, 0
//2, 170403, Note_on_c, 1, 56, 37
//2, 170403, Note_on_c, 1, 49, 37
//2, 170499, Note_off_c, 1, 49, 0
//2, 170499, Note_off_c, 1, 56, 0
//2, 170499, Note_on_c, 1, 56, 37
//2, 170499, Note_on_c, 1, 49, 37
//2, 170595, Note_off_c, 1, 49, 0
//2, 170595, Note_off_c, 1, 56, 0
//2, 170595, Note_on_c, 1, 56, 37
//2, 170595, Note_on_c, 1, 49, 37
//2, 170691, Note_off_c, 1, 49, 0
//2, 170691, Note_off_c, 1, 56, 0
//2, 170691, Note_on_c, 1, 55, 37
//2, 170691, Note_on_c, 1, 48, 37
//2, 170787, Note_off_c, 1, 48, 0
//2, 170787, Note_off_c, 1, 55, 0
//2, 170787, Note_on_c, 1, 55, 37
//2, 170787, Note_on_c, 1, 48, 37
//2, 170883, Note_off_c, 1, 48, 0
//2, 170883, Note_off_c, 1, 55, 0
//2, 170883, Note_on_c, 1, 54, 37
//2, 170883, Note_on_c, 1, 54, 37
//2, 170883, Note_on_c, 1, 47, 37
//2, 170979, Note_off_c, 1, 47, 0
//2, 170979, Note_off_c, 1, 54, 0
//2, 170979, Note_off_c, 1, 54, 0
//2, 170979, Note_on_c, 1, 54, 37
//2, 170979, Note_on_c, 1, 54, 37
//2, 170979, Note_on_c, 1, 47, 37
//2, 171075, Note_off_c, 1, 47, 0
//2, 171075, Note_off_c, 1, 54, 0
//2, 171075, Note_off_c, 1, 54, 0
//2, 171075, Note_on_c, 1, 53, 37
//2, 171075, Note_on_c, 1, 46, 37
//2, 171171, Note_off_c, 1, 46, 0
//2, 171171, Note_off_c, 1, 53, 0
//2, 171171, Note_on_c, 1, 53, 37
//2, 171171, Note_on_c, 1, 46, 37
//2, 171267, Note_off_c, 1, 46, 0
//2, 171267, Note_off_c, 1, 53, 0
//2, 171267, Note_on_c, 1, 58, 37
//2, 171267, Note_on_c, 1, 51, 37
//2, 171363, Note_off_c, 1, 51, 0
//2, 171363, Note_off_c, 1, 58, 0
//2, 171363, Note_on_c, 1, 58, 37
//2, 171363, Note_on_c, 1, 51, 37
//2, 171459, Note_off_c, 1, 51, 0
//2, 171459, Note_off_c, 1, 58, 0
//2, 171459, Note_on_c, 1, 58, 37
//2, 171459, Note_on_c, 1, 51, 37
//2, 171555, Note_off_c, 1, 51, 0
//2, 171555, Note_off_c, 1, 58, 0
//2, 171555, Note_on_c, 1, 58, 37
//2, 171555, Note_on_c, 1, 51, 37
//2, 171651, Note_off_c, 1, 51, 0
//2, 171651, Note_off_c, 1, 58, 0
//2, 171651, Note_on_c, 1, 57, 37
//2, 171651, Note_on_c, 1, 50, 37
//2, 171747, Note_off_c, 1, 50, 0
//2, 171747, Note_off_c, 1, 57, 0
//2, 171747, Note_on_c, 1, 57, 37
//2, 171747, Note_on_c, 1, 50, 37
//2, 171843, Note_off_c, 1, 50, 0
//2, 171843, Note_off_c, 1, 57, 0
//2, 171843, Note_on_c, 1, 57, 37
//2, 171843, Note_on_c, 1, 50, 37
//2, 171939, Note_off_c, 1, 50, 0
//2, 171939, Note_off_c, 1, 57, 0
//2, 171939, Note_on_c, 1, 56, 37
//2, 171939, Note_on_c, 1, 49, 37
//2, 172035, Note_off_c, 1, 49, 0
//2, 172035, Note_off_c, 1, 56, 0
//2, 172035, Note_on_c, 1, 56, 37
//2, 172035, Note_on_c, 1, 49, 37
//2, 172131, Note_off_c, 1, 49, 0
//2, 172131, Note_off_c, 1, 56, 0
//2, 172131, Note_on_c, 1, 56, 37
//2, 172131, Note_on_c, 1, 49, 37
//2, 172227, Note_off_c, 1, 49, 0
//2, 172227, Note_off_c, 1, 56, 0
//2, 172227, Note_on_c, 1, 55, 37
//2, 172227, Note_on_c, 1, 48, 37
//2, 172323, Note_off_c, 1, 48, 0
//2, 172323, Note_off_c, 1, 55, 0
//2, 172323, Note_on_c, 1, 55, 37
//2, 172323, Note_on_c, 1, 48, 37
//2, 172419, Note_off_c, 1, 48, 0
//2, 172419, Note_off_c, 1, 55, 0
//2, 172419, Note_on_c, 1, 54, 37
//2, 172419, Note_on_c, 1, 54, 37
//2, 172419, Note_on_c, 1, 47, 37
//2, 172515, Note_off_c, 1, 47, 0
//2, 172515, Note_off_c, 1, 54, 0
//2, 172515, Note_off_c, 1, 54, 0
//2, 172515, Note_on_c, 1, 54, 37
//2, 172515, Note_on_c, 1, 54, 37
//2, 172515, Note_on_c, 1, 47, 37
//2, 172611, Note_off_c, 1, 47, 0
//2, 172611, Note_off_c, 1, 54, 0
//2, 172611, Note_off_c, 1, 54, 0
//2, 172611, Note_on_c, 1, 53, 37
//2, 172611, Note_on_c, 1, 46, 37
//2, 172707, Note_off_c, 1, 46, 0
//2, 172707, Note_off_c, 1, 53, 0
//2, 172707, Note_on_c, 1, 53, 37
//2, 172707, Note_on_c, 1, 46, 37
//2, 172803, Note_off_c, 1, 46, 0
//2, 172803, Note_off_c, 1, 53, 0
//2, 172803, Note_on_c, 1, 48, 37
//2, 172803, Note_on_c, 1, 41, 37
//2, 173187, Note_off_c, 1, 41, 0
//2, 173187, Note_off_c, 1, 48, 0
//2, 173187, Note_on_c, 1, 47, 37
//2, 173187, Note_on_c, 1, 40, 37
//2, 173571, Note_off_c, 1, 40, 0
//2, 173571, Note_off_c, 1, 47, 0
//2, 173571, Note_on_c, 1, 48, 37
//2, 173571, Note_on_c, 1, 41, 37
//2, 173955, Note_off_c, 1, 41, 0
//2, 173955, Note_off_c, 1, 48, 0
//2, 173955, Note_on_c, 1, 49, 37
//2, 173955, Note_on_c, 1, 42, 37
//2, 174339, Note_off_c, 1, 42, 0
//2, 174339, Note_off_c, 1, 49, 0
//2, 174339, Note_on_c, 1, 51, 37
//2, 174339, Note_on_c, 1, 44, 37
//2, 174723, Note_off_c, 1, 44, 0
//2, 174723, Note_off_c, 1, 51, 0
//2, 174723, Note_on_c, 1, 50, 37
//2, 174723, Note_on_c, 1, 43, 37
//2, 175107, Note_off_c, 1, 43, 0
//2, 175107, Note_off_c, 1, 50, 0
//2, 175107, Note_on_c, 1, 51, 37
//2, 175107, Note_on_c, 1, 44, 37
//2, 175491, Note_off_c, 1, 44, 0
//2, 175491, Note_off_c, 1, 51, 0
//2, 175491, Note_on_c, 1, 52, 37
//2, 175491, Note_on_c, 1, 45, 37
//2, 175683, Note_off_c, 1, 45, 0
//2, 175683, Note_off_c, 1, 52, 0
//2, 175683, Note_on_c, 1, 39, 37
//2, 175875, Note_off_c, 1, 39, 0
//2, 175875, Note_on_c, 1, 54, 37
//2, 175875, Note_on_c, 1, 47, 37
//2, 176259, Note_off_c, 1, 47, 0
//2, 176259, Note_off_c, 1, 54, 0
//2, 176259, Note_on_c, 1, 46, 37
//2, 176643, Note_off_c, 1, 46, 0
//2, 176643, Note_on_c, 1, 44, 37
//2, 177027, Note_off_c, 1, 44, 0
//2, 177027, Note_on_c, 1, 42, 37
//2, 177411, Note_off_c, 1, 42, 0
//2, 177411, Note_on_c, 1, 56, 37
//2, 177411, Note_on_c, 1, 49, 37
//2, 177795, Note_off_c, 1, 49, 0
//2, 177795, Note_off_c, 1, 56, 0
//2, 177795, Note_on_c, 1, 48, 37
//2, 178179, Note_off_c, 1, 48, 0
//2, 178179, Note_on_c, 1, 46, 37
//2, 178563, Note_off_c, 1, 46, 0
//2, 178563, Note_on_c, 1, 44, 37
//2, 178947, Note_off_c, 1, 44, 0
//2, 178947, Note_on_c, 1, 58, 37
//2, 178947, Note_on_c, 1, 51, 37
//2, 179139, Note_off_c, 1, 51, 0
//2, 179139, Note_off_c, 1, 58, 0
//2, 179139, Note_on_c, 1, 58, 37
//2, 179139, Note_on_c, 1, 51, 37
//2, 179331, Note_off_c, 1, 51, 0
//2, 179331, Note_off_c, 1, 58, 0
//2, 179331, Note_on_c, 1, 49, 37
//2, 179523, Note_off_c, 1, 49, 0
//2, 179523, Note_on_c, 1, 45, 37
//2, 179619, Note_off_c, 1, 45, 0
//2, 179619, Note_on_c, 1, 45, 37
//2, 179715, Note_off_c, 1, 45, 0
//2, 179811, Note_on_c, 1, 44, 37
//2, 179907, Note_off_c, 1, 44, 0
//2, 179907, Note_on_c, 1, 44, 37
//2, 180099, Note_off_c, 1, 44, 0
//2, 180099, Note_on_c, 1, 42, 37
//2, 180291, Note_off_c, 1, 42, 0
//2, 180291, Note_on_c, 1, 39, 37
//2, 180387, Note_off_c, 1, 39, 0
//2, 180483, Note_on_c, 1, 51, 37
//2, 180675, Note_off_c, 1, 51, 0
//2, 180675, Note_on_c, 1, 51, 37
//2, 180867, Note_off_c, 1, 51, 0
//2, 180867, Note_on_c, 1, 49, 37
//2, 181059, Note_off_c, 1, 49, 0
//2, 181059, Note_on_c, 1, 45, 37
//2, 181155, Note_off_c, 1, 45, 0
//2, 181155, Note_on_c, 1, 45, 37
//2, 181347, Note_off_c, 1, 45, 0
//2, 181347, Note_on_c, 1, 44, 37
//2, 181443, Note_off_c, 1, 44, 0
//2, 181443, Note_on_c, 1, 44, 37
//2, 181635, Note_off_c, 1, 44, 0
//2, 181635, Note_on_c, 1, 42, 37
//2, 181827, Note_off_c, 1, 42, 0
//2, 181827, Note_on_c, 1, 39, 37
//2, 181923, Note_off_c, 1, 39, 0
//2, 182019, Note_on_c, 1, 54, 37
//2, 182019, Note_on_c, 1, 47, 37
//2, 182403, Note_off_c, 1, 47, 0
//2, 182403, Note_off_c, 1, 54, 0
//2, 182403, Note_on_c, 1, 46, 37
//2, 182787, Note_off_c, 1, 46, 0
//2, 182787, Note_on_c, 1, 44, 37
//2, 183171, Note_off_c, 1, 44, 0
//2, 183171, Note_on_c, 1, 42, 37
//2, 183555, Note_off_c, 1, 42, 0
//2, 183555, Note_on_c, 1, 56, 37
//2, 183555, Note_on_c, 1, 49, 37
//2, 183939, Note_off_c, 1, 49, 0
//2, 183939, Note_off_c, 1, 56, 0
//2, 183939, Note_on_c, 1, 48, 37
//2, 184323, Note_off_c, 1, 48, 0
//2, 184323, Note_on_c, 1, 46, 37
//2, 184707, Note_off_c, 1, 46, 0
//2, 184707, Note_on_c, 1, 44, 37
//2, 185091, Note_off_c, 1, 44, 0
//2, 185091, Note_on_c, 1, 58, 37
//2, 185091, Note_on_c, 1, 51, 37
//2, 185283, Note_off_c, 1, 51, 0
//2, 185283, Note_off_c, 1, 58, 0
//2, 185283, Note_on_c, 1, 58, 37
//2, 185283, Note_on_c, 1, 51, 37
//2, 185475, Note_off_c, 1, 51, 0
//2, 185475, Note_off_c, 1, 58, 0
//2, 185475, Note_on_c, 1, 49, 37
//2, 185667, Note_off_c, 1, 49, 0
//2, 185667, Note_on_c, 1, 45, 37
//2, 185763, Note_off_c, 1, 45, 0
//2, 185763, Note_on_c, 1, 45, 37
//2, 185859, Note_off_c, 1, 45, 0
//2, 185955, Note_on_c, 1, 44, 37
//2, 186051, Note_off_c, 1, 44, 0
//2, 186051, Note_on_c, 1, 44, 37
//2, 186243, Note_off_c, 1, 44, 0
//2, 186243, Note_on_c, 1, 42, 37
//2, 186435, Note_off_c, 1, 42, 0
//2, 186435, Note_on_c, 1, 39, 37
//2, 186531, Note_off_c, 1, 39, 0
//2, 186627, Note_on_c, 1, 51, 37
//2, 186819, Note_off_c, 1, 51, 0
//2, 186819, Note_on_c, 1, 51, 37
//2, 187011, Note_off_c, 1, 51, 0
//2, 187011, Note_on_c, 1, 49, 37
//2, 187203, Note_off_c, 1, 49, 0
//2, 187203, Note_on_c, 1, 45, 37
//2, 187299, Note_off_c, 1, 45, 0
//2, 187299, Note_on_c, 1, 45, 37
//2, 187491, Note_off_c, 1, 45, 0
//2, 187491, Note_on_c, 1, 44, 37
//2, 187587, Note_off_c, 1, 44, 0
//2, 187587, Note_on_c, 1, 44, 37
//2, 187779, Note_off_c, 1, 44, 0
//2, 187779, Note_on_c, 1, 42, 37
//2, 187971, Note_off_c, 1, 42, 0
//2, 187971, Note_on_c, 1, 39, 37
//2, 188067, Note_off_c, 1, 39, 0
//2, 188163, Note_on_c, 1, 54, 37
//2, 188163, Note_on_c, 1, 47, 37
//2, 188547, Note_off_c, 1, 47, 0
//2, 188547, Note_off_c, 1, 54, 0
//2, 188547, Note_on_c, 1, 46, 37
//2, 188931, Note_off_c, 1, 46, 0
//2, 188931, Note_on_c, 1, 44, 37
//2, 189315, Note_off_c, 1, 44, 0
//2, 189315, Note_on_c, 1, 42, 37
//2, 189699, Note_off_c, 1, 42, 0
//2, 189699, Note_on_c, 1, 56, 37
//2, 189699, Note_on_c, 1, 49, 37
//2, 190083, Note_off_c, 1, 49, 0
//2, 190083, Note_off_c, 1, 56, 0
//2, 190083, Note_on_c, 1, 48, 37
//2, 190467, Note_off_c, 1, 48, 0
//2, 190467, Note_on_c, 1, 46, 37
//2, 190851, Note_off_c, 1, 46, 0
//2, 190851, Note_on_c, 1, 44, 37
//2, 191235, Note_off_c, 1, 44, 0
//2, 191235, Note_on_c, 1, 58, 37
//2, 191235, Note_on_c, 1, 51, 37
//2, 191427, Note_off_c, 1, 51, 0
//2, 191427, Note_off_c, 1, 58, 0
//2, 191427, Note_on_c, 1, 58, 37
//2, 191427, Note_on_c, 1, 51, 37
//2, 191619, Note_off_c, 1, 51, 0
//2, 191619, Note_off_c, 1, 58, 0
//2, 191619, Note_on_c, 1, 49, 37
//2, 191811, Note_off_c, 1, 49, 0
//2, 191811, Note_on_c, 1, 45, 37
//2, 191907, Note_off_c, 1, 45, 0
//2, 191907, Note_on_c, 1, 45, 37
//2, 192003, Note_off_c, 1, 45, 0
//2, 192099, Note_on_c, 1, 44, 37
//2, 192195, Note_off_c, 1, 44, 0
//2, 192195, Note_on_c, 1, 44, 37
//2, 192387, Note_off_c, 1, 44, 0
//2, 192387, Note_on_c, 1, 42, 37
//2, 192579, Note_off_c, 1, 42, 0
//2, 192579, Note_on_c, 1, 39, 37
//2, 192675, Note_off_c, 1, 39, 0
//2, 192771, Note_on_c, 1, 51, 37
//2, 192963, Note_off_c, 1, 51, 0
//2, 192963, Note_on_c, 1, 51, 37
//2, 193155, Note_off_c, 1, 51, 0
//2, 193155, Note_on_c, 1, 49, 37
//2, 193347, Note_off_c, 1, 49, 0
//2, 193347, Note_on_c, 1, 45, 37
//2, 193443, Note_off_c, 1, 45, 0
//2, 193443, Note_on_c, 1, 45, 37
//2, 193635, Note_off_c, 1, 45, 0
//2, 193635, Note_on_c, 1, 44, 37
//2, 193731, Note_off_c, 1, 44, 0
//2, 193731, Note_on_c, 1, 44, 37
//2, 193923, Note_off_c, 1, 44, 0
//2, 193923, Note_on_c, 1, 42, 37
//2, 194115, Note_off_c, 1, 42, 0
//2, 194115, Note_on_c, 1, 39, 37
//2, 194211, Note_off_c, 1, 39, 0
//2, 194307, Note_on_c, 1, 54, 37
//2, 194307, Note_on_c, 1, 47, 37
//2, 194691, Note_off_c, 1, 47, 0
//2, 194691, Note_off_c, 1, 54, 0
//2, 194691, Note_on_c, 1, 46, 37
//2, 195075, Note_off_c, 1, 46, 0
//2, 195075, Note_on_c, 1, 44, 37
//2, 195459, Note_off_c, 1, 44, 0
//2, 195459, Note_on_c, 1, 42, 37
//2, 195843, Note_off_c, 1, 42, 0
//2, 195843, Note_on_c, 1, 56, 37
//2, 195843, Note_on_c, 1, 49, 37
//2, 196227, Note_off_c, 1, 49, 0
//2, 196227, Note_off_c, 1, 56, 0
//2, 196227, Note_on_c, 1, 48, 37
//2, 196611, Note_off_c, 1, 48, 0
//2, 196611, Note_on_c, 1, 46, 37
//2, 196995, Note_off_c, 1, 46, 0
//2, 196995, Note_on_c, 1, 44, 37
//2, 197379, Note_off_c, 1, 44, 0
//2, 197379, Note_on_c, 1, 58, 37
//2, 197379, Note_on_c, 1, 63, 37
//2, 197667, Note_off_c, 1, 63, 0
//2, 197667, Note_off_c, 1, 58, 0
//2, 197667, Note_on_c, 1, 56, 37
//2, 197667, Note_on_c, 1, 61, 37
//2, 197955, Note_off_c, 1, 61, 0
//2, 197955, Note_off_c, 1, 56, 0
//2, 197955, Note_on_c, 1, 52, 37
//2, 197955, Note_on_c, 1, 57, 37
//2, 198243, Note_off_c, 1, 57, 0
//2, 198243, Note_off_c, 1, 52, 0
//2, 198243, Note_on_c, 1, 51, 37
//2, 198243, Note_on_c, 1, 56, 37
//2, 198531, Note_off_c, 1, 56, 0
//2, 198531, Note_off_c, 1, 51, 0
//2, 198531, Note_on_c, 1, 49, 37
//2, 198531, Note_on_c, 1, 54, 37
//2, 198723, Note_off_c, 1, 54, 0
//2, 198723, Note_off_c, 1, 49, 0
//2, 198915, Note_on_c, 1, 56, 37
//2, 198915, Note_on_c, 1, 60, 37
//2, 199203, Note_off_c, 1, 60, 0
//2, 199203, Note_off_c, 1, 56, 0
//2, 199203, Note_on_c, 1, 54, 37
//2, 199203, Note_on_c, 1, 58, 37
//2, 199491, Note_off_c, 1, 58, 0
//2, 199491, Note_off_c, 1, 54, 0
//2, 199491, Note_on_c, 1, 51, 37
//2, 199491, Note_on_c, 1, 55, 37
//2, 199683, Note_off_c, 1, 55, 0
//2, 199683, Note_off_c, 1, 51, 0
//2, 200067, Note_on_c, 1, 66, 37
//2, 200067, Note_on_c, 1, 61, 37
//2, 203523, Note_off_c, 1, 61, 0
//2, 203523, Note_off_c, 1, 66, 0
//2, 203523, End_track
