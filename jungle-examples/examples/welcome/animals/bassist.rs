use jungle_sdk::prelude::*;

use crate::effect::{DecrementCounterEffect, Monad, Rest};
use crate::instrumentation::{BassArticulation, Thump as LaneThump, Vocals, VocalsArticulation};

use super::Bass;

const BASS_LANE_ID: u32 = <<Bass as Animal>::Id as AnimalIdValue>::U32;
const BASS_BACKUP_VOCALS_LANE_ID: u32 = BASS_LANE_ID + 1;
type Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneThump<NOTE, NOTE_TICK, REST_TICK, BASS_LANE_ID>;

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
    type Effect =
        Monad<Vocals, VocalsArticulation, BASS_BACKUP_VOCALS_LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
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

pub struct MergeJoinUnit;
#[jungle::act]
impl Act for MergeJoinUnit {
    type Effect = DecrementCounterEffect;
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

#[derive(Flow)]
pub struct BassIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, BassSection01>,
    Transparent<IntroSectionMeta, BassSection02>,
    Transparent<IntroSectionMeta, BassSection03>,
    Transparent<IntroSectionMeta, BassSection04>,
    Transparent<IntroSectionMeta, BassSection05>,
    Transparent<IntroSectionMeta, BassSection06>,
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
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart02(
    Step<Thump<46, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<43, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart03(
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<34, 96, 96>>,
    Step<Thump<37, 768, 768>>,
    Step<Thump<32, 768, 768>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart06(
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
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart07(
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
pub struct BassPart08(
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart09(
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart11(
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
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart12(
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
pub struct BassPart13(
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart14(
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart22(
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
#[jungle(focus = BassArticulation)]
pub struct BassPart35(
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
pub struct BassPart36(
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
pub struct BassPart37(
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
pub struct BassPart38(
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
pub struct BassPart39(
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<71, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<35, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPart45(
    Join<Step<Thump<30, 192, 192>>, Step<HarmonySing<66, 384, 384>>>,
    Step<MergeJoinUnit>,
    Step<Thump<27, 192, 192>>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<73, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<72, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 384, 384>>, Step<HarmonySing<70, 384, 384>>>,
    Step<MergeJoinUnit>,
    Join<Step<Thump<37, 192, 192>>, Step<HarmonySing<68, 384, 384>>>,
    Step<MergeJoinUnit>,
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
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::Bass;
    use crate::ecosystem::TheJungle;

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
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

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
}

//5, 0, Title_t, "Bass Guitar"
//5, 0, Program_c, 4, 33
//5, 5376, Note_on_c, 4, 46, 37
//5, 6912, Note_off_c, 4, 46, 0
//5, 6912, Note_on_c, 4, 44, 37
//5, 8256, Note_off_c, 4, 44, 0
//5, 8256, Note_on_c, 4, 34, 37
//5, 8448, Note_off_c, 4, 34, 0
//5, 8448, Note_on_c, 4, 30, 37
//5, 9600, Note_off_c, 4, 30, 0
//5, 9792, Note_on_c, 4, 37, 37
//5, 9888, Note_off_c, 4, 37, 0
//5, 9888, Note_on_c, 4, 38, 37
//5, 9984, Note_off_c, 4, 38, 0
//5, 9984, Note_on_c, 4, 39, 37
//5, 11136, Note_off_c, 4, 39, 0
//5, 11328, Note_on_c, 4, 44, 37
//5, 11424, Note_off_c, 4, 44, 0
//5, 11424, Note_on_c, 4, 45, 37
//5, 11520, Note_off_c, 4, 45, 0
//5, 11520, Note_on_c, 4, 46, 37
//5, 11616, Note_off_c, 4, 46, 0
//5, 11616, Note_on_c, 4, 46, 37
//5, 11712, Note_off_c, 4, 46, 0
//5, 11712, Note_on_c, 4, 46, 37
//5, 11808, Note_off_c, 4, 46, 0
//5, 11808, Note_on_c, 4, 46, 37
//5, 11904, Note_off_c, 4, 46, 0
//5, 11904, Note_on_c, 4, 46, 37
//5, 12000, Note_off_c, 4, 46, 0
//5, 12000, Note_on_c, 4, 46, 37
//5, 12096, Note_off_c, 4, 46, 0
//5, 12096, Note_on_c, 4, 46, 37
//5, 12192, Note_off_c, 4, 46, 0
//5, 12192, Note_on_c, 4, 46, 37
//5, 12288, Note_off_c, 4, 46, 0
//5, 12288, Note_on_c, 4, 46, 37
//5, 12384, Note_off_c, 4, 46, 0
//5, 12384, Note_on_c, 4, 46, 37
//5, 12480, Note_off_c, 4, 46, 0
//5, 12480, Note_on_c, 4, 46, 37
//5, 12576, Note_off_c, 4, 46, 0
//5, 12576, Note_on_c, 4, 46, 37
//5, 12672, Note_off_c, 4, 46, 0
//5, 12672, Note_on_c, 4, 46, 37
//5, 12768, Note_off_c, 4, 46, 0
//5, 12768, Note_on_c, 4, 46, 37
//5, 12864, Note_off_c, 4, 46, 0
//5, 12864, Note_on_c, 4, 46, 37
//5, 12960, Note_off_c, 4, 46, 0
//5, 12960, Note_on_c, 4, 46, 37
//5, 13056, Note_off_c, 4, 46, 0
//5, 13056, Note_on_c, 4, 44, 37
//5, 13152, Note_off_c, 4, 44, 0
//5, 13152, Note_on_c, 4, 44, 37
//5, 13248, Note_off_c, 4, 44, 0
//5, 13248, Note_on_c, 4, 44, 37
//5, 13344, Note_off_c, 4, 44, 0
//5, 13344, Note_on_c, 4, 44, 37
//5, 13440, Note_off_c, 4, 44, 0
//5, 13440, Note_on_c, 4, 44, 37
//5, 13536, Note_off_c, 4, 44, 0
//5, 13536, Note_on_c, 4, 44, 37
//5, 13632, Note_off_c, 4, 44, 0
//5, 13632, Note_on_c, 4, 44, 37
//5, 13728, Note_off_c, 4, 44, 0
//5, 13728, Note_on_c, 4, 44, 37
//5, 13824, Note_off_c, 4, 44, 0
//5, 13824, Note_on_c, 4, 44, 37
//5, 13920, Note_off_c, 4, 44, 0
//5, 13920, Note_on_c, 4, 44, 37
//5, 14016, Note_off_c, 4, 44, 0
//5, 14016, Note_on_c, 4, 44, 37
//5, 14112, Note_off_c, 4, 44, 0
//5, 14112, Note_on_c, 4, 44, 37
//5, 14208, Note_off_c, 4, 44, 0
//5, 14208, Note_on_c, 4, 44, 37
//5, 14304, Note_off_c, 4, 44, 0
//5, 14304, Note_on_c, 4, 44, 37
//5, 14400, Note_off_c, 4, 44, 0
//5, 14400, Note_on_c, 4, 44, 37
//5, 14496, Note_off_c, 4, 44, 0
//5, 14496, Note_on_c, 4, 43, 37
//5, 14592, Note_off_c, 4, 43, 0
//5, 14592, Note_on_c, 4, 39, 37
//5, 14688, Note_off_c, 4, 39, 0
//5, 14688, Note_on_c, 4, 39, 37
//5, 14784, Note_off_c, 4, 39, 0
//5, 14784, Note_on_c, 4, 39, 37
//5, 14880, Note_off_c, 4, 39, 0
//5, 14880, Note_on_c, 4, 39, 37
//5, 14976, Note_off_c, 4, 39, 0
//5, 14976, Note_on_c, 4, 39, 37
//5, 15072, Note_off_c, 4, 39, 0
//5, 15072, Note_on_c, 4, 39, 37
//5, 15168, Note_off_c, 4, 39, 0
//5, 15168, Note_on_c, 4, 39, 37
//5, 15264, Note_off_c, 4, 39, 0
//5, 15264, Note_on_c, 4, 39, 37
//5, 15360, Note_off_c, 4, 39, 0
//5, 15360, Note_on_c, 4, 39, 37
//5, 15456, Note_off_c, 4, 39, 0
//5, 15456, Note_on_c, 4, 39, 37
//5, 15552, Note_off_c, 4, 39, 0
//5, 15552, Note_on_c, 4, 39, 37
//5, 15648, Note_off_c, 4, 39, 0
//5, 15648, Note_on_c, 4, 39, 37
//5, 15744, Note_off_c, 4, 39, 0
//5, 15744, Note_on_c, 4, 39, 37
//5, 15840, Note_off_c, 4, 39, 0
//5, 15840, Note_on_c, 4, 39, 37
//5, 15936, Note_off_c, 4, 39, 0
//5, 15936, Note_on_c, 4, 39, 37
//5, 16032, Note_off_c, 4, 39, 0
//5, 16032, Note_on_c, 4, 34, 37
//5, 16128, Note_off_c, 4, 34, 0
//5, 16128, Note_on_c, 4, 37, 37
//5, 16896, Note_off_c, 4, 37, 0
//5, 16896, Note_on_c, 4, 32, 37
//5, 17664, Note_off_c, 4, 32, 0
//5, 17664, Note_on_c, 4, 34, 37
//5, 17856, Note_off_c, 4, 34, 0
//5, 17856, Note_on_c, 4, 34, 37
//5, 18048, Note_off_c, 4, 34, 0
//5, 18048, Note_on_c, 4, 34, 37
//5, 18240, Note_off_c, 4, 34, 0
//5, 18240, Note_on_c, 4, 34, 37
//5, 18432, Note_off_c, 4, 34, 0
//5, 18432, Note_on_c, 4, 34, 37
//5, 18624, Note_off_c, 4, 34, 0
//5, 18624, Note_on_c, 4, 34, 37
//5, 18816, Note_off_c, 4, 34, 0
//5, 18816, Note_on_c, 4, 34, 37
//5, 19008, Note_off_c, 4, 34, 0
//5, 19008, Note_on_c, 4, 34, 37
//5, 19200, Note_off_c, 4, 34, 0
//5, 19200, Note_on_c, 4, 34, 37
//5, 19392, Note_off_c, 4, 34, 0
//5, 19392, Note_on_c, 4, 41, 37
//5, 19584, Note_off_c, 4, 41, 0
//5, 19584, Note_on_c, 4, 44, 37
//5, 19776, Note_off_c, 4, 44, 0
//5, 19776, Note_on_c, 4, 45, 37
//5, 19968, Note_off_c, 4, 45, 0
//5, 19968, Note_on_c, 4, 46, 37
//5, 20160, Note_off_c, 4, 46, 0
//5, 20160, Note_on_c, 4, 27, 37
//5, 20352, Note_off_c, 4, 27, 0
//5, 20352, Note_on_c, 4, 39, 37
//5, 20736, Note_off_c, 4, 39, 0
//5, 20736, Note_on_c, 4, 32, 37
//5, 20928, Note_off_c, 4, 32, 0
//5, 20928, Note_on_c, 4, 32, 37
//5, 21120, Note_off_c, 4, 32, 0
//5, 21120, Note_on_c, 4, 30, 37
//5, 21312, Note_off_c, 4, 30, 0
//5, 21312, Note_on_c, 4, 27, 37
//5, 21408, Note_off_c, 4, 27, 0
//5, 21408, Note_on_c, 4, 32, 37
//5, 21600, Note_off_c, 4, 32, 0
//5, 21600, Note_on_c, 4, 27, 37
//5, 21696, Note_off_c, 4, 27, 0
//5, 21696, Note_on_c, 4, 30, 37
//5, 21888, Note_off_c, 4, 30, 0
//5, 21888, Note_on_c, 4, 29, 37
//5, 22080, Note_off_c, 4, 29, 0
//5, 22080, Note_on_c, 4, 27, 37
//5, 22272, Note_off_c, 4, 27, 0
//5, 22272, Note_on_c, 4, 32, 37
//5, 22464, Note_off_c, 4, 32, 0
//5, 22464, Note_on_c, 4, 32, 37
//5, 22656, Note_off_c, 4, 32, 0
//5, 22656, Note_on_c, 4, 30, 37
//5, 22848, Note_off_c, 4, 30, 0
//5, 22848, Note_on_c, 4, 27, 37
//5, 22944, Note_off_c, 4, 27, 0
//5, 22944, Note_on_c, 4, 32, 37
//5, 23136, Note_off_c, 4, 32, 0
//5, 23136, Note_on_c, 4, 27, 37
//5, 23232, Note_off_c, 4, 27, 0
//5, 23232, Note_on_c, 4, 30, 37
//5, 23424, Note_off_c, 4, 30, 0
//5, 23424, Note_on_c, 4, 29, 37
//5, 23616, Note_off_c, 4, 29, 0
//5, 23616, Note_on_c, 4, 27, 37
//5, 23808, Note_off_c, 4, 27, 0
//5, 23808, Note_on_c, 4, 32, 37
//5, 24000, Note_off_c, 4, 32, 0
//5, 24000, Note_on_c, 4, 32, 37
//5, 24192, Note_off_c, 4, 32, 0
//5, 24192, Note_on_c, 4, 30, 37
//5, 24384, Note_off_c, 4, 30, 0
//5, 24384, Note_on_c, 4, 27, 37
//5, 24480, Note_off_c, 4, 27, 0
//5, 24480, Note_on_c, 4, 32, 37
//5, 24672, Note_off_c, 4, 32, 0
//5, 24672, Note_on_c, 4, 27, 37
//5, 24768, Note_off_c, 4, 27, 0
//5, 24768, Note_on_c, 4, 30, 37
//5, 24960, Note_off_c, 4, 30, 0
//5, 24960, Note_on_c, 4, 29, 37
//5, 25152, Note_off_c, 4, 29, 0
//5, 25152, Note_on_c, 4, 27, 37
//5, 25344, Note_off_c, 4, 27, 0
//5, 25344, Note_on_c, 4, 32, 37
//5, 25536, Note_off_c, 4, 32, 0
//5, 25536, Note_on_c, 4, 32, 37
//5, 25728, Note_off_c, 4, 32, 0
//5, 25728, Note_on_c, 4, 30, 37
//5, 25920, Note_off_c, 4, 30, 0
//5, 25920, Note_on_c, 4, 27, 37
//5, 26016, Note_off_c, 4, 27, 0
//5, 26016, Note_on_c, 4, 32, 37
//5, 26208, Note_off_c, 4, 32, 0
//5, 26208, Note_on_c, 4, 27, 37
//5, 26304, Note_off_c, 4, 27, 0
//5, 26304, Note_on_c, 4, 42, 37
//5, 26400, Note_off_c, 4, 42, 0
//5, 26496, Note_on_c, 4, 42, 37
//5, 26592, Note_off_c, 4, 42, 0
//5, 26688, Note_on_c, 4, 42, 37
//5, 26784, Note_off_c, 4, 42, 0
//5, 26880, Note_on_c, 4, 32, 37
//5, 27072, Note_off_c, 4, 32, 0
//5, 27072, Note_on_c, 4, 32, 37
//5, 27264, Note_off_c, 4, 32, 0
//5, 27264, Note_on_c, 4, 30, 37
//5, 27456, Note_off_c, 4, 30, 0
//5, 27456, Note_on_c, 4, 27, 37
//5, 27552, Note_off_c, 4, 27, 0
//5, 27552, Note_on_c, 4, 32, 37
//5, 27744, Note_off_c, 4, 32, 0
//5, 27744, Note_on_c, 4, 27, 37
//5, 27840, Note_off_c, 4, 27, 0
//5, 27840, Note_on_c, 4, 30, 37
//5, 28032, Note_off_c, 4, 30, 0
//5, 28032, Note_on_c, 4, 29, 37
//5, 28224, Note_off_c, 4, 29, 0
//5, 28224, Note_on_c, 4, 27, 37
//5, 28320, Note_off_c, 4, 27, 0
//5, 28320, Note_on_c, 4, 27, 37
//5, 28416, Note_off_c, 4, 27, 0
//5, 28416, Note_on_c, 4, 32, 37
//5, 28608, Note_off_c, 4, 32, 0
//5, 28608, Note_on_c, 4, 32, 37
//5, 28800, Note_off_c, 4, 32, 0
//5, 28800, Note_on_c, 4, 30, 37
//5, 28992, Note_off_c, 4, 30, 0
//5, 28992, Note_on_c, 4, 27, 37
//5, 29088, Note_off_c, 4, 27, 0
//5, 29088, Note_on_c, 4, 32, 37
//5, 29280, Note_off_c, 4, 32, 0
//5, 29280, Note_on_c, 4, 27, 37
//5, 29376, Note_off_c, 4, 27, 0
//5, 29376, Note_on_c, 4, 30, 37
//5, 29568, Note_off_c, 4, 30, 0
//5, 29568, Note_on_c, 4, 29, 37
//5, 29760, Note_off_c, 4, 29, 0
//5, 29760, Note_on_c, 4, 27, 37
//5, 29856, Note_off_c, 4, 27, 0
//5, 29856, Note_on_c, 4, 27, 37
//5, 29952, Note_off_c, 4, 27, 0
//5, 29952, Note_on_c, 4, 32, 37
//5, 30144, Note_off_c, 4, 32, 0
//5, 30144, Note_on_c, 4, 32, 37
//5, 30336, Note_off_c, 4, 32, 0
//5, 30336, Note_on_c, 4, 30, 37
//5, 30528, Note_off_c, 4, 30, 0
//5, 30528, Note_on_c, 4, 27, 37
//5, 30624, Note_off_c, 4, 27, 0
//5, 30624, Note_on_c, 4, 32, 37
//5, 30816, Note_off_c, 4, 32, 0
//5, 30816, Note_on_c, 4, 27, 37
//5, 30912, Note_off_c, 4, 27, 0
//5, 30912, Note_on_c, 4, 30, 37
//5, 31104, Note_off_c, 4, 30, 0
//5, 31104, Note_on_c, 4, 29, 37
//5, 31296, Note_off_c, 4, 29, 0
//5, 31296, Note_on_c, 4, 27, 37
//5, 31392, Note_off_c, 4, 27, 0
//5, 31392, Note_on_c, 4, 27, 37
//5, 31488, Note_off_c, 4, 27, 0
//5, 31488, Note_on_c, 4, 32, 37
//5, 31680, Note_off_c, 4, 32, 0
//5, 31680, Note_on_c, 4, 32, 37
//5, 31872, Note_off_c, 4, 32, 0
//5, 31872, Note_on_c, 4, 30, 37
//5, 32064, Note_off_c, 4, 30, 0
//5, 32064, Note_on_c, 4, 27, 37
//5, 32160, Note_off_c, 4, 27, 0
//5, 32160, Note_on_c, 4, 32, 37
//5, 32352, Note_off_c, 4, 32, 0
//5, 32352, Note_on_c, 4, 27, 37
//5, 32448, Note_off_c, 4, 27, 0
//5, 32448, Note_on_c, 4, 30, 37
//5, 32640, Note_off_c, 4, 30, 0
//5, 32640, Note_on_c, 4, 32, 37
//5, 32832, Note_off_c, 4, 32, 0
//5, 32832, Note_on_c, 4, 37, 37
//5, 32928, Note_off_c, 4, 37, 0
//5, 32928, Note_on_c, 4, 38, 37
//5, 33024, Note_off_c, 4, 38, 0
//5, 33024, Note_on_c, 4, 39, 37
//5, 33408, Note_off_c, 4, 39, 0
//5, 33408, Note_on_c, 4, 37, 37
//5, 33600, Note_off_c, 4, 37, 0
//5, 33600, Note_on_c, 4, 32, 37
//5, 33696, Note_off_c, 4, 32, 0
//5, 33696, Note_on_c, 4, 39, 37
//5, 33888, Note_off_c, 4, 39, 0
//5, 33888, Note_on_c, 4, 32, 37
//5, 33984, Note_off_c, 4, 32, 0
//5, 33984, Note_on_c, 4, 37, 37
//5, 34176, Note_off_c, 4, 37, 0
//5, 34176, Note_on_c, 4, 36, 37
//5, 34368, Note_off_c, 4, 36, 0
//5, 34368, Note_on_c, 4, 34, 37
//5, 34560, Note_off_c, 4, 34, 0
//5, 34560, Note_on_c, 4, 39, 37
//5, 34752, Note_off_c, 4, 39, 0
//5, 34752, Note_on_c, 4, 39, 37
//5, 34944, Note_off_c, 4, 39, 0
//5, 34944, Note_on_c, 4, 37, 37
//5, 35136, Note_off_c, 4, 37, 0
//5, 35136, Note_on_c, 4, 32, 37
//5, 35232, Note_off_c, 4, 32, 0
//5, 35232, Note_on_c, 4, 39, 37
//5, 35424, Note_off_c, 4, 39, 0
//5, 35424, Note_on_c, 4, 32, 37
//5, 35520, Note_off_c, 4, 32, 0
//5, 35520, Note_on_c, 4, 37, 37
//5, 35712, Note_off_c, 4, 37, 0
//5, 35712, Note_on_c, 4, 36, 37
//5, 35904, Note_off_c, 4, 36, 0
//5, 35904, Note_on_c, 4, 34, 37
//5, 36096, Note_off_c, 4, 34, 0
//5, 36096, Note_on_c, 4, 39, 37
//5, 36288, Note_off_c, 4, 39, 0
//5, 36288, Note_on_c, 4, 39, 37
//5, 36480, Note_off_c, 4, 39, 0
//5, 36480, Note_on_c, 4, 37, 37
//5, 36672, Note_off_c, 4, 37, 0
//5, 36672, Note_on_c, 4, 32, 37
//5, 36768, Note_off_c, 4, 32, 0
//5, 36768, Note_on_c, 4, 39, 37
//5, 36960, Note_off_c, 4, 39, 0
//5, 36960, Note_on_c, 4, 32, 37
//5, 37056, Note_off_c, 4, 32, 0
//5, 37056, Note_on_c, 4, 37, 37
//5, 37248, Note_off_c, 4, 37, 0
//5, 37248, Note_on_c, 4, 36, 37
//5, 37440, Note_off_c, 4, 36, 0
//5, 37440, Note_on_c, 4, 34, 37
//5, 37632, Note_off_c, 4, 34, 0
//5, 37632, Note_on_c, 4, 39, 37
//5, 37824, Note_off_c, 4, 39, 0
//5, 37824, Note_on_c, 4, 39, 37
//5, 38016, Note_off_c, 4, 39, 0
//5, 38016, Note_on_c, 4, 37, 37
//5, 38208, Note_off_c, 4, 37, 0
//5, 38208, Note_on_c, 4, 32, 37
//5, 38304, Note_off_c, 4, 32, 0
//5, 38304, Note_on_c, 4, 39, 37
//5, 38496, Note_off_c, 4, 39, 0
//5, 38496, Note_on_c, 4, 32, 37
//5, 38592, Note_off_c, 4, 32, 0
//5, 38592, Note_on_c, 4, 37, 37
//5, 38784, Note_off_c, 4, 37, 0
//5, 38784, Note_on_c, 4, 39, 37
//5, 38976, Note_off_c, 4, 39, 0
//5, 38976, Note_on_c, 4, 32, 37
//5, 39168, Note_off_c, 4, 32, 0
//5, 39168, Note_on_c, 4, 35, 37
//5, 39552, Note_off_c, 4, 35, 0
//5, 39552, Note_on_c, 4, 35, 37
//5, 39936, Note_off_c, 4, 35, 0
//5, 39936, Note_on_c, 4, 35, 37
//5, 40128, Note_off_c, 4, 35, 0
//5, 40128, Note_on_c, 4, 27, 37
//5, 40320, Note_off_c, 4, 27, 0
//5, 40320, Note_on_c, 4, 30, 37
//5, 40512, Note_off_c, 4, 30, 0
//5, 40512, Note_on_c, 4, 27, 37
//5, 40704, Note_off_c, 4, 27, 0
//5, 40704, Note_on_c, 4, 37, 37
//5, 41088, Note_off_c, 4, 37, 0
//5, 41088, Note_on_c, 4, 37, 37
//5, 41472, Note_off_c, 4, 37, 0
//5, 41472, Note_on_c, 4, 37, 37
//5, 41856, Note_off_c, 4, 37, 0
//5, 41856, Note_on_c, 4, 37, 37
//5, 42048, Note_off_c, 4, 37, 0
//5, 42048, Note_on_c, 4, 32, 37
//5, 42144, Note_off_c, 4, 32, 0
//5, 42144, Note_on_c, 4, 38, 37
//5, 42240, Note_off_c, 4, 38, 0
//5, 42240, Note_on_c, 4, 39, 37
//5, 42432, Note_off_c, 4, 39, 0
//5, 42432, Note_on_c, 4, 39, 37
//5, 42624, Note_off_c, 4, 39, 0
//5, 42624, Note_on_c, 4, 37, 37
//5, 42816, Note_off_c, 4, 37, 0
//5, 42816, Note_on_c, 4, 33, 37
//5, 42912, Note_off_c, 4, 33, 0
//5, 42912, Note_on_c, 4, 33, 37
//5, 43104, Note_off_c, 4, 33, 0
//5, 43104, Note_on_c, 4, 32, 37
//5, 43200, Note_off_c, 4, 32, 0
//5, 43200, Note_on_c, 4, 32, 37
//5, 43392, Note_off_c, 4, 32, 0
//5, 43392, Note_on_c, 4, 30, 37
//5, 43584, Note_off_c, 4, 30, 0
//5, 43584, Note_on_c, 4, 27, 37
//5, 43776, Note_off_c, 4, 27, 0
//5, 43776, Note_on_c, 4, 39, 37
//5, 43968, Note_off_c, 4, 39, 0
//5, 43968, Note_on_c, 4, 39, 37
//5, 44160, Note_off_c, 4, 39, 0
//5, 44160, Note_on_c, 4, 37, 37
//5, 44352, Note_off_c, 4, 37, 0
//5, 44352, Note_on_c, 4, 33, 37
//5, 44448, Note_off_c, 4, 33, 0
//5, 44448, Note_on_c, 4, 33, 37
//5, 44640, Note_off_c, 4, 33, 0
//5, 44640, Note_on_c, 4, 32, 37
//5, 44736, Note_off_c, 4, 32, 0
//5, 44736, Note_on_c, 4, 32, 37
//5, 44928, Note_off_c, 4, 32, 0
//5, 44928, Note_on_c, 4, 30, 37
//5, 45120, Note_off_c, 4, 30, 0
//5, 45120, Note_on_c, 4, 27, 37
//5, 45312, Note_off_c, 4, 27, 0
//5, 45312, Note_on_c, 4, 39, 37
//5, 45504, Note_off_c, 4, 39, 0
//5, 45504, Note_on_c, 4, 39, 37
//5, 45696, Note_off_c, 4, 39, 0
//5, 45696, Note_on_c, 4, 37, 37
//5, 45888, Note_off_c, 4, 37, 0
//5, 45888, Note_on_c, 4, 33, 37
//5, 45984, Note_off_c, 4, 33, 0
//5, 45984, Note_on_c, 4, 33, 37
//5, 46176, Note_off_c, 4, 33, 0
//5, 46176, Note_on_c, 4, 32, 37
//5, 46272, Note_off_c, 4, 32, 0
//5, 46272, Note_on_c, 4, 32, 37
//5, 46464, Note_off_c, 4, 32, 0
//5, 46464, Note_on_c, 4, 30, 37
//5, 46656, Note_off_c, 4, 30, 0
//5, 46656, Note_on_c, 4, 27, 37
//5, 46848, Note_off_c, 4, 27, 0
//5, 46848, Note_on_c, 4, 39, 37
//5, 47136, Note_off_c, 4, 39, 0
//5, 47136, Note_on_c, 4, 37, 37
//5, 47424, Note_off_c, 4, 37, 0
//5, 47424, Note_on_c, 4, 33, 37
//5, 47808, Note_off_c, 4, 33, 0
//5, 47808, Note_on_c, 4, 32, 37
//5, 48000, Note_off_c, 4, 32, 0
//5, 48000, Note_on_c, 4, 30, 37
//5, 48192, Note_off_c, 4, 30, 0
//5, 48192, Note_on_c, 4, 31, 37
//5, 48384, Note_off_c, 4, 31, 0
//5, 48384, Note_on_c, 4, 32, 37
//5, 48576, Note_off_c, 4, 32, 0
//5, 48576, Note_on_c, 4, 32, 37
//5, 48768, Note_off_c, 4, 32, 0
//5, 48768, Note_on_c, 4, 30, 37
//5, 48960, Note_off_c, 4, 30, 0
//5, 48960, Note_on_c, 4, 27, 37
//5, 49056, Note_off_c, 4, 27, 0
//5, 49056, Note_on_c, 4, 32, 37
//5, 49248, Note_off_c, 4, 32, 0
//5, 49248, Note_on_c, 4, 27, 37
//5, 49344, Note_off_c, 4, 27, 0
//5, 49344, Note_on_c, 4, 30, 37
//5, 49536, Note_off_c, 4, 30, 0
//5, 49536, Note_on_c, 4, 29, 37
//5, 49728, Note_off_c, 4, 29, 0
//5, 49728, Note_on_c, 4, 27, 37
//5, 49824, Note_off_c, 4, 27, 0
//5, 49824, Note_on_c, 4, 27, 37
//5, 49920, Note_off_c, 4, 27, 0
//5, 49920, Note_on_c, 4, 32, 37
//5, 50112, Note_off_c, 4, 32, 0
//5, 50112, Note_on_c, 4, 32, 37
//5, 50304, Note_off_c, 4, 32, 0
//5, 50304, Note_on_c, 4, 30, 37
//5, 50496, Note_off_c, 4, 30, 0
//5, 50496, Note_on_c, 4, 27, 37
//5, 50592, Note_off_c, 4, 27, 0
//5, 50592, Note_on_c, 4, 32, 37
//5, 50784, Note_off_c, 4, 32, 0
//5, 50784, Note_on_c, 4, 27, 37
//5, 50880, Note_off_c, 4, 27, 0
//5, 50880, Note_on_c, 4, 30, 37
//5, 51072, Note_off_c, 4, 30, 0
//5, 51072, Note_on_c, 4, 29, 37
//5, 51264, Note_off_c, 4, 29, 0
//5, 51264, Note_on_c, 4, 27, 37
//5, 51360, Note_off_c, 4, 27, 0
//5, 51360, Note_on_c, 4, 27, 37
//5, 51456, Note_off_c, 4, 27, 0
//5, 51456, Note_on_c, 4, 32, 37
//5, 51648, Note_off_c, 4, 32, 0
//5, 51648, Note_on_c, 4, 32, 37
//5, 51840, Note_off_c, 4, 32, 0
//5, 51840, Note_on_c, 4, 30, 37
//5, 52032, Note_off_c, 4, 30, 0
//5, 52032, Note_on_c, 4, 27, 37
//5, 52128, Note_off_c, 4, 27, 0
//5, 52128, Note_on_c, 4, 32, 37
//5, 52320, Note_off_c, 4, 32, 0
//5, 52320, Note_on_c, 4, 27, 37
//5, 52416, Note_off_c, 4, 27, 0
//5, 52416, Note_on_c, 4, 30, 37
//5, 52608, Note_off_c, 4, 30, 0
//5, 52608, Note_on_c, 4, 29, 37
//5, 52800, Note_off_c, 4, 29, 0
//5, 52800, Note_on_c, 4, 27, 37
//5, 52896, Note_off_c, 4, 27, 0
//5, 52896, Note_on_c, 4, 27, 37
//5, 52992, Note_off_c, 4, 27, 0
//5, 52992, Note_on_c, 4, 32, 37
//5, 53184, Note_off_c, 4, 32, 0
//5, 53184, Note_on_c, 4, 32, 37
//5, 53376, Note_off_c, 4, 32, 0
//5, 53376, Note_on_c, 4, 30, 37
//5, 53568, Note_off_c, 4, 30, 0
//5, 53568, Note_on_c, 4, 27, 37
//5, 53664, Note_off_c, 4, 27, 0
//5, 53664, Note_on_c, 4, 32, 37
//5, 53856, Note_off_c, 4, 32, 0
//5, 53856, Note_on_c, 4, 27, 37
//5, 53952, Note_off_c, 4, 27, 0
//5, 53952, Note_on_c, 4, 30, 37
//5, 54144, Note_off_c, 4, 30, 0
//5, 54144, Note_on_c, 4, 32, 37
//5, 54336, Note_off_c, 4, 32, 0
//5, 54336, Note_on_c, 4, 37, 37
//5, 54432, Note_off_c, 4, 37, 0
//5, 54432, Note_on_c, 4, 38, 37
//5, 54528, Note_off_c, 4, 38, 0
//5, 54528, Note_on_c, 4, 39, 37
//5, 54912, Note_off_c, 4, 39, 0
//5, 54912, Note_on_c, 4, 37, 37
//5, 55104, Note_off_c, 4, 37, 0
//5, 55104, Note_on_c, 4, 32, 37
//5, 55200, Note_off_c, 4, 32, 0
//5, 55200, Note_on_c, 4, 39, 37
//5, 55392, Note_off_c, 4, 39, 0
//5, 55392, Note_on_c, 4, 32, 37
//5, 55488, Note_off_c, 4, 32, 0
//5, 55488, Note_on_c, 4, 37, 37
//5, 55680, Note_off_c, 4, 37, 0
//5, 55680, Note_on_c, 4, 36, 37
//5, 55872, Note_off_c, 4, 36, 0
//5, 55872, Note_on_c, 4, 34, 37
//5, 56064, Note_off_c, 4, 34, 0
//5, 56064, Note_on_c, 4, 39, 37
//5, 56256, Note_off_c, 4, 39, 0
//5, 56256, Note_on_c, 4, 39, 37
//5, 56448, Note_off_c, 4, 39, 0
//5, 56448, Note_on_c, 4, 37, 37
//5, 56640, Note_off_c, 4, 37, 0
//5, 56640, Note_on_c, 4, 32, 37
//5, 56736, Note_off_c, 4, 32, 0
//5, 56736, Note_on_c, 4, 39, 37
//5, 56928, Note_off_c, 4, 39, 0
//5, 56928, Note_on_c, 4, 32, 37
//5, 57024, Note_off_c, 4, 32, 0
//5, 57024, Note_on_c, 4, 37, 37
//5, 57216, Note_off_c, 4, 37, 0
//5, 57216, Note_on_c, 4, 36, 37
//5, 57408, Note_off_c, 4, 36, 0
//5, 57408, Note_on_c, 4, 34, 37
//5, 57600, Note_off_c, 4, 34, 0
//5, 57600, Note_on_c, 4, 39, 37
//5, 57792, Note_off_c, 4, 39, 0
//5, 57792, Note_on_c, 4, 39, 37
//5, 57984, Note_off_c, 4, 39, 0
//5, 57984, Note_on_c, 4, 37, 37
//5, 58176, Note_off_c, 4, 37, 0
//5, 58176, Note_on_c, 4, 32, 37
//5, 58272, Note_off_c, 4, 32, 0
//5, 58272, Note_on_c, 4, 39, 37
//5, 58464, Note_off_c, 4, 39, 0
//5, 58464, Note_on_c, 4, 32, 37
//5, 58560, Note_off_c, 4, 32, 0
//5, 58560, Note_on_c, 4, 37, 37
//5, 58752, Note_off_c, 4, 37, 0
//5, 58752, Note_on_c, 4, 36, 37
//5, 58944, Note_off_c, 4, 36, 0
//5, 58944, Note_on_c, 4, 34, 37
//5, 59136, Note_off_c, 4, 34, 0
//5, 59136, Note_on_c, 4, 39, 37
//5, 59328, Note_off_c, 4, 39, 0
//5, 59328, Note_on_c, 4, 39, 37
//5, 59520, Note_off_c, 4, 39, 0
//5, 59520, Note_on_c, 4, 37, 37
//5, 59712, Note_off_c, 4, 37, 0
//5, 59712, Note_on_c, 4, 32, 37
//5, 59808, Note_off_c, 4, 32, 0
//5, 59808, Note_on_c, 4, 39, 37
//5, 60000, Note_off_c, 4, 39, 0
//5, 60000, Note_on_c, 4, 32, 37
//5, 60096, Note_off_c, 4, 32, 0
//5, 60096, Note_on_c, 4, 37, 37
//5, 60288, Note_off_c, 4, 37, 0
//5, 60288, Note_on_c, 4, 39, 37
//5, 60480, Note_off_c, 4, 39, 0
//5, 60480, Note_on_c, 4, 32, 37
//5, 60672, Note_off_c, 4, 32, 0
//5, 60672, Note_on_c, 4, 35, 37
//5, 61056, Note_off_c, 4, 35, 0
//5, 61056, Note_on_c, 4, 35, 37
//5, 61440, Note_off_c, 4, 35, 0
//5, 61440, Note_on_c, 4, 35, 37
//5, 61632, Note_off_c, 4, 35, 0
//5, 61632, Note_on_c, 4, 27, 37
//5, 61824, Note_off_c, 4, 27, 0
//5, 61824, Note_on_c, 4, 30, 37
//5, 62016, Note_off_c, 4, 30, 0
//5, 62016, Note_on_c, 4, 27, 37
//5, 62208, Note_off_c, 4, 27, 0
//5, 62208, Note_on_c, 4, 37, 37
//5, 62592, Note_off_c, 4, 37, 0
//5, 62592, Note_on_c, 4, 37, 37
//5, 62976, Note_off_c, 4, 37, 0
//5, 62976, Note_on_c, 4, 37, 37
//5, 63360, Note_off_c, 4, 37, 0
//5, 63360, Note_on_c, 4, 37, 37
//5, 63552, Note_off_c, 4, 37, 0
//5, 63552, Note_on_c, 4, 32, 37
//5, 63648, Note_off_c, 4, 32, 0
//5, 63648, Note_on_c, 4, 38, 37
//5, 63744, Note_off_c, 4, 38, 0
//5, 63744, Note_on_c, 4, 39, 37
//5, 63936, Note_off_c, 4, 39, 0
//5, 63936, Note_on_c, 4, 39, 37
//5, 64128, Note_off_c, 4, 39, 0
//5, 64128, Note_on_c, 4, 37, 37
//5, 64320, Note_off_c, 4, 37, 0
//5, 64320, Note_on_c, 4, 33, 37
//5, 64416, Note_off_c, 4, 33, 0
//5, 64416, Note_on_c, 4, 33, 37
//5, 64608, Note_off_c, 4, 33, 0
//5, 64608, Note_on_c, 4, 32, 37
//5, 64704, Note_off_c, 4, 32, 0
//5, 64704, Note_on_c, 4, 32, 37
//5, 64896, Note_off_c, 4, 32, 0
//5, 64896, Note_on_c, 4, 30, 37
//5, 65088, Note_off_c, 4, 30, 0
//5, 65088, Note_on_c, 4, 27, 37
//5, 65280, Note_off_c, 4, 27, 0
//5, 65280, Note_on_c, 4, 39, 37
//5, 65472, Note_off_c, 4, 39, 0
//5, 65472, Note_on_c, 4, 39, 37
//5, 65664, Note_off_c, 4, 39, 0
//5, 65664, Note_on_c, 4, 37, 37
//5, 65856, Note_off_c, 4, 37, 0
//5, 65856, Note_on_c, 4, 33, 37
//5, 65952, Note_off_c, 4, 33, 0
//5, 65952, Note_on_c, 4, 33, 37
//5, 66144, Note_off_c, 4, 33, 0
//5, 66144, Note_on_c, 4, 32, 37
//5, 66240, Note_off_c, 4, 32, 0
//5, 66240, Note_on_c, 4, 32, 37
//5, 66432, Note_off_c, 4, 32, 0
//5, 66432, Note_on_c, 4, 30, 37
//5, 66624, Note_off_c, 4, 30, 0
//5, 66624, Note_on_c, 4, 27, 37
//5, 66816, Note_off_c, 4, 27, 0
//5, 66816, Note_on_c, 4, 39, 37
//5, 67008, Note_off_c, 4, 39, 0
//5, 67008, Note_on_c, 4, 39, 37
//5, 67200, Note_off_c, 4, 39, 0
//5, 67200, Note_on_c, 4, 37, 37
//5, 67392, Note_off_c, 4, 37, 0
//5, 67392, Note_on_c, 4, 33, 37
//5, 67488, Note_off_c, 4, 33, 0
//5, 67488, Note_on_c, 4, 33, 37
//5, 67680, Note_off_c, 4, 33, 0
//5, 67680, Note_on_c, 4, 32, 37
//5, 67776, Note_off_c, 4, 32, 0
//5, 67776, Note_on_c, 4, 32, 37
//5, 67968, Note_off_c, 4, 32, 0
//5, 67968, Note_on_c, 4, 30, 37
//5, 68160, Note_off_c, 4, 30, 0
//5, 68160, Note_on_c, 4, 27, 37
//5, 68352, Note_off_c, 4, 27, 0
//5, 68352, Note_on_c, 4, 34, 37
//5, 68640, Note_off_c, 4, 34, 0
//5, 68640, Note_on_c, 4, 34, 37
//5, 68928, Note_off_c, 4, 34, 0
//5, 68928, Note_on_c, 4, 34, 37
//5, 69312, Note_off_c, 4, 34, 0
//5, 69312, Note_on_c, 4, 34, 37
//5, 69504, Note_off_c, 4, 34, 0
//5, 69504, Note_on_c, 4, 44, 37
//5, 69888, Note_off_c, 4, 44, 0
//5, 69888, Note_on_c, 4, 27, 37
//5, 70080, Note_off_c, 4, 27, 0
//5, 70080, Note_on_c, 4, 36, 37
//5, 70272, Note_off_c, 4, 36, 0
//5, 70272, Note_on_c, 4, 37, 37
//5, 70464, Note_off_c, 4, 37, 0
//5, 70464, Note_on_c, 4, 27, 37
//5, 70560, Note_off_c, 4, 27, 0
//5, 70560, Note_on_c, 4, 27, 37
//5, 70656, Note_off_c, 4, 27, 0
//5, 70656, Note_on_c, 4, 27, 37
//5, 70848, Note_off_c, 4, 27, 0
//5, 70848, Note_on_c, 4, 39, 37
//5, 71040, Note_off_c, 4, 39, 0
//5, 71040, Note_on_c, 4, 39, 37
//5, 71232, Note_off_c, 4, 39, 0
//5, 71232, Note_on_c, 4, 27, 37
//5, 71328, Note_off_c, 4, 27, 0
//5, 71328, Note_on_c, 4, 27, 37
//5, 71424, Note_off_c, 4, 27, 0
//5, 71424, Note_on_c, 4, 27, 37
//5, 71616, Note_off_c, 4, 27, 0
//5, 71616, Note_on_c, 4, 36, 37
//5, 71808, Note_off_c, 4, 36, 0
//5, 71808, Note_on_c, 4, 37, 37
//5, 72000, Note_off_c, 4, 37, 0
//5, 72000, Note_on_c, 4, 27, 37
//5, 72096, Note_off_c, 4, 27, 0
//5, 72096, Note_on_c, 4, 27, 37
//5, 72192, Note_off_c, 4, 27, 0
//5, 72192, Note_on_c, 4, 30, 37
//5, 72288, Note_off_c, 4, 30, 0
//5, 72384, Note_on_c, 4, 30, 37
//5, 72480, Note_off_c, 4, 30, 0
//5, 72576, Note_on_c, 4, 30, 37
//5, 72672, Note_off_c, 4, 30, 0
//5, 72768, Note_on_c, 4, 27, 37
//5, 72864, Note_off_c, 4, 27, 0
//5, 72864, Note_on_c, 4, 27, 37
//5, 72960, Note_off_c, 4, 27, 0
//5, 72960, Note_on_c, 4, 27, 37
//5, 73152, Note_off_c, 4, 27, 0
//5, 73152, Note_on_c, 4, 36, 37
//5, 73344, Note_off_c, 4, 36, 0
//5, 73344, Note_on_c, 4, 37, 37
//5, 73536, Note_off_c, 4, 37, 0
//5, 73536, Note_on_c, 4, 27, 37
//5, 73632, Note_off_c, 4, 27, 0
//5, 73632, Note_on_c, 4, 27, 37
//5, 73728, Note_off_c, 4, 27, 0
//5, 73728, Note_on_c, 4, 27, 37
//5, 73920, Note_off_c, 4, 27, 0
//5, 73920, Note_on_c, 4, 39, 37
//5, 74112, Note_off_c, 4, 39, 0
//5, 74112, Note_on_c, 4, 39, 37
//5, 74304, Note_off_c, 4, 39, 0
//5, 74304, Note_on_c, 4, 27, 37
//5, 74400, Note_off_c, 4, 27, 0
//5, 74400, Note_on_c, 4, 27, 37
//5, 74496, Note_off_c, 4, 27, 0
//5, 74496, Note_on_c, 4, 27, 37
//5, 74688, Note_off_c, 4, 27, 0
//5, 74688, Note_on_c, 4, 36, 37
//5, 74880, Note_off_c, 4, 36, 0
//5, 74880, Note_on_c, 4, 37, 37
//5, 75072, Note_off_c, 4, 37, 0
//5, 75072, Note_on_c, 4, 27, 37
//5, 75168, Note_off_c, 4, 27, 0
//5, 75168, Note_on_c, 4, 27, 37
//5, 75264, Note_off_c, 4, 27, 0
//5, 75264, Note_on_c, 4, 30, 37
//5, 75360, Note_off_c, 4, 30, 0
//5, 75456, Note_on_c, 4, 30, 37
//5, 75552, Note_off_c, 4, 30, 0
//5, 75648, Note_on_c, 4, 30, 37
//5, 75744, Note_off_c, 4, 30, 0
//5, 75840, Note_on_c, 4, 27, 37
//5, 75936, Note_off_c, 4, 27, 0
//5, 75936, Note_on_c, 4, 27, 37
//5, 76032, Note_off_c, 4, 27, 0
//5, 76032, Note_on_c, 4, 27, 37
//5, 76224, Note_off_c, 4, 27, 0
//5, 76224, Note_on_c, 4, 36, 37
//5, 76416, Note_off_c, 4, 36, 0
//5, 76416, Note_on_c, 4, 37, 37
//5, 76608, Note_off_c, 4, 37, 0
//5, 76608, Note_on_c, 4, 27, 37
//5, 76704, Note_off_c, 4, 27, 0
//5, 76704, Note_on_c, 4, 27, 37
//5, 76800, Note_off_c, 4, 27, 0
//5, 76800, Note_on_c, 4, 27, 37
//5, 76992, Note_off_c, 4, 27, 0
//5, 76992, Note_on_c, 4, 39, 37
//5, 77184, Note_off_c, 4, 39, 0
//5, 77184, Note_on_c, 4, 39, 37
//5, 77376, Note_off_c, 4, 39, 0
//5, 77376, Note_on_c, 4, 27, 37
//5, 77472, Note_off_c, 4, 27, 0
//5, 77472, Note_on_c, 4, 27, 37
//5, 77568, Note_off_c, 4, 27, 0
//5, 77568, Note_on_c, 4, 27, 37
//5, 77760, Note_off_c, 4, 27, 0
//5, 77760, Note_on_c, 4, 36, 37
//5, 77952, Note_off_c, 4, 36, 0
//5, 77952, Note_on_c, 4, 37, 37
//5, 78144, Note_off_c, 4, 37, 0
//5, 78144, Note_on_c, 4, 27, 37
//5, 78336, Note_off_c, 4, 27, 0
//5, 78336, Note_on_c, 4, 30, 37
//5, 78528, Note_off_c, 4, 30, 0
//5, 78528, Note_on_c, 4, 42, 37
//5, 78720, Note_off_c, 4, 42, 0
//5, 78720, Note_on_c, 4, 30, 37
//5, 78912, Note_off_c, 4, 30, 0
//5, 78912, Note_on_c, 4, 42, 37
//5, 79104, Note_off_c, 4, 42, 0
//5, 79104, Note_on_c, 4, 27, 37
//5, 79296, Note_off_c, 4, 27, 0
//5, 79296, Note_on_c, 4, 36, 37
//5, 79488, Note_off_c, 4, 36, 0
//5, 79488, Note_on_c, 4, 37, 37
//5, 79680, Note_off_c, 4, 37, 0
//5, 79680, Note_on_c, 4, 27, 37
//5, 79776, Note_off_c, 4, 27, 0
//5, 79776, Note_on_c, 4, 27, 37
//5, 79872, Note_off_c, 4, 27, 0
//5, 79872, Note_on_c, 4, 27, 37
//5, 80064, Note_off_c, 4, 27, 0
//5, 80064, Note_on_c, 4, 39, 37
//5, 80256, Note_off_c, 4, 39, 0
//5, 80256, Note_on_c, 4, 39, 37
//5, 80448, Note_off_c, 4, 39, 0
//5, 80448, Note_on_c, 4, 27, 37
//5, 80544, Note_off_c, 4, 27, 0
//5, 80544, Note_on_c, 4, 27, 37
//5, 80640, Note_off_c, 4, 27, 0
//5, 80640, Note_on_c, 4, 27, 37
//5, 80832, Note_off_c, 4, 27, 0
//5, 80832, Note_on_c, 4, 36, 37
//5, 81024, Note_off_c, 4, 36, 0
//5, 81024, Note_on_c, 4, 37, 37
//5, 81216, Note_off_c, 4, 37, 0
//5, 81216, Note_on_c, 4, 27, 37
//5, 81312, Note_off_c, 4, 27, 0
//5, 81312, Note_on_c, 4, 27, 37
//5, 81408, Note_off_c, 4, 27, 0
//5, 81408, Note_on_c, 4, 30, 37
//5, 81504, Note_off_c, 4, 30, 0
//5, 81600, Note_on_c, 4, 30, 37
//5, 81696, Note_off_c, 4, 30, 0
//5, 81792, Note_on_c, 4, 30, 37
//5, 81888, Note_off_c, 4, 30, 0
//5, 81984, Note_on_c, 4, 27, 37
//5, 82080, Note_off_c, 4, 27, 0
//5, 82080, Note_on_c, 4, 27, 37
//5, 82176, Note_off_c, 4, 27, 0
//5, 82176, Note_on_c, 4, 32, 37
//5, 82368, Note_off_c, 4, 32, 0
//5, 82368, Note_on_c, 4, 32, 37
//5, 82560, Note_off_c, 4, 32, 0
//5, 82560, Note_on_c, 4, 30, 37
//5, 82752, Note_off_c, 4, 30, 0
//5, 82752, Note_on_c, 4, 27, 37
//5, 82848, Note_off_c, 4, 27, 0
//5, 82848, Note_on_c, 4, 32, 37
//5, 83040, Note_off_c, 4, 32, 0
//5, 83040, Note_on_c, 4, 27, 37
//5, 83136, Note_off_c, 4, 27, 0
//5, 83136, Note_on_c, 4, 30, 37
//5, 83328, Note_off_c, 4, 30, 0
//5, 83328, Note_on_c, 4, 29, 37
//5, 83520, Note_off_c, 4, 29, 0
//5, 83520, Note_on_c, 4, 27, 37
//5, 83616, Note_off_c, 4, 27, 0
//5, 83616, Note_on_c, 4, 27, 37
//5, 83712, Note_off_c, 4, 27, 0
//5, 83712, Note_on_c, 4, 32, 37
//5, 83904, Note_off_c, 4, 32, 0
//5, 83904, Note_on_c, 4, 32, 37
//5, 84096, Note_off_c, 4, 32, 0
//5, 84096, Note_on_c, 4, 30, 37
//5, 84288, Note_off_c, 4, 30, 0
//5, 84288, Note_on_c, 4, 27, 37
//5, 84384, Note_off_c, 4, 27, 0
//5, 84384, Note_on_c, 4, 32, 37
//5, 84576, Note_off_c, 4, 32, 0
//5, 84576, Note_on_c, 4, 27, 37
//5, 84672, Note_off_c, 4, 27, 0
//5, 84672, Note_on_c, 4, 30, 37
//5, 84864, Note_off_c, 4, 30, 0
//5, 84864, Note_on_c, 4, 29, 37
//5, 85056, Note_off_c, 4, 29, 0
//5, 85056, Note_on_c, 4, 27, 37
//5, 85152, Note_off_c, 4, 27, 0
//5, 85152, Note_on_c, 4, 27, 37
//5, 85248, Note_off_c, 4, 27, 0
//5, 85248, Note_on_c, 4, 32, 37
//5, 85440, Note_off_c, 4, 32, 0
//5, 85440, Note_on_c, 4, 32, 37
//5, 85632, Note_off_c, 4, 32, 0
//5, 85632, Note_on_c, 4, 30, 37
//5, 85824, Note_off_c, 4, 30, 0
//5, 85824, Note_on_c, 4, 27, 37
//5, 85920, Note_off_c, 4, 27, 0
//5, 85920, Note_on_c, 4, 32, 37
//5, 86112, Note_off_c, 4, 32, 0
//5, 86112, Note_on_c, 4, 27, 37
//5, 86208, Note_off_c, 4, 27, 0
//5, 86208, Note_on_c, 4, 30, 37
//5, 86400, Note_off_c, 4, 30, 0
//5, 86400, Note_on_c, 4, 29, 37
//5, 86592, Note_off_c, 4, 29, 0
//5, 86592, Note_on_c, 4, 27, 37
//5, 86688, Note_off_c, 4, 27, 0
//5, 86688, Note_on_c, 4, 27, 37
//5, 86784, Note_off_c, 4, 27, 0
//5, 86784, Note_on_c, 4, 32, 37
//5, 86976, Note_off_c, 4, 32, 0
//5, 86976, Note_on_c, 4, 32, 37
//5, 87168, Note_off_c, 4, 32, 0
//5, 87168, Note_on_c, 4, 30, 37
//5, 87360, Note_off_c, 4, 30, 0
//5, 87360, Note_on_c, 4, 27, 37
//5, 87456, Note_off_c, 4, 27, 0
//5, 87456, Note_on_c, 4, 32, 37
//5, 87648, Note_off_c, 4, 32, 0
//5, 87648, Note_on_c, 4, 27, 37
//5, 87744, Note_off_c, 4, 27, 0
//5, 87744, Note_on_c, 4, 30, 37
//5, 87936, Note_off_c, 4, 30, 0
//5, 87936, Note_on_c, 4, 32, 37
//5, 88128, Note_off_c, 4, 32, 0
//5, 88128, Note_on_c, 4, 37, 37
//5, 88224, Note_off_c, 4, 37, 0
//5, 88224, Note_on_c, 4, 38, 37
//5, 88320, Note_off_c, 4, 38, 0
//5, 88320, Note_on_c, 4, 39, 37
//5, 88704, Note_off_c, 4, 39, 0
//5, 88704, Note_on_c, 4, 37, 37
//5, 88896, Note_off_c, 4, 37, 0
//5, 88896, Note_on_c, 4, 32, 37
//5, 88992, Note_off_c, 4, 32, 0
//5, 88992, Note_on_c, 4, 39, 37
//5, 89184, Note_off_c, 4, 39, 0
//5, 89184, Note_on_c, 4, 32, 37
//5, 89280, Note_off_c, 4, 32, 0
//5, 89280, Note_on_c, 4, 37, 37
//5, 89472, Note_off_c, 4, 37, 0
//5, 89472, Note_on_c, 4, 36, 37
//5, 89664, Note_off_c, 4, 36, 0
//5, 89664, Note_on_c, 4, 34, 37
//5, 89856, Note_off_c, 4, 34, 0
//5, 89856, Note_on_c, 4, 39, 37
//5, 90048, Note_off_c, 4, 39, 0
//5, 90048, Note_on_c, 4, 39, 37
//5, 90240, Note_off_c, 4, 39, 0
//5, 90240, Note_on_c, 4, 37, 37
//5, 90432, Note_off_c, 4, 37, 0
//5, 90432, Note_on_c, 4, 32, 37
//5, 90528, Note_off_c, 4, 32, 0
//5, 90528, Note_on_c, 4, 39, 37
//5, 90720, Note_off_c, 4, 39, 0
//5, 90720, Note_on_c, 4, 32, 37
//5, 90816, Note_off_c, 4, 32, 0
//5, 90816, Note_on_c, 4, 37, 37
//5, 91008, Note_off_c, 4, 37, 0
//5, 91008, Note_on_c, 4, 36, 37
//5, 91200, Note_off_c, 4, 36, 0
//5, 91200, Note_on_c, 4, 34, 37
//5, 91392, Note_off_c, 4, 34, 0
//5, 91392, Note_on_c, 4, 39, 37
//5, 91584, Note_off_c, 4, 39, 0
//5, 91584, Note_on_c, 4, 39, 37
//5, 91776, Note_off_c, 4, 39, 0
//5, 91776, Note_on_c, 4, 37, 37
//5, 91968, Note_off_c, 4, 37, 0
//5, 91968, Note_on_c, 4, 32, 37
//5, 92064, Note_off_c, 4, 32, 0
//5, 92064, Note_on_c, 4, 39, 37
//5, 92256, Note_off_c, 4, 39, 0
//5, 92256, Note_on_c, 4, 32, 37
//5, 92352, Note_off_c, 4, 32, 0
//5, 92352, Note_on_c, 4, 37, 37
//5, 92544, Note_off_c, 4, 37, 0
//5, 92544, Note_on_c, 4, 36, 37
//5, 92736, Note_off_c, 4, 36, 0
//5, 92736, Note_on_c, 4, 34, 37
//5, 92928, Note_off_c, 4, 34, 0
//5, 92928, Note_on_c, 4, 39, 37
//5, 93120, Note_off_c, 4, 39, 0
//5, 93120, Note_on_c, 4, 39, 37
//5, 93312, Note_off_c, 4, 39, 0
//5, 93312, Note_on_c, 4, 37, 37
//5, 93504, Note_off_c, 4, 37, 0
//5, 93504, Note_on_c, 4, 32, 37
//5, 93600, Note_off_c, 4, 32, 0
//5, 93600, Note_on_c, 4, 39, 37
//5, 93792, Note_off_c, 4, 39, 0
//5, 93792, Note_on_c, 4, 32, 37
//5, 93888, Note_off_c, 4, 32, 0
//5, 93888, Note_on_c, 4, 37, 37
//5, 94080, Note_off_c, 4, 37, 0
//5, 94080, Note_on_c, 4, 39, 37
//5, 94272, Note_off_c, 4, 39, 0
//5, 94272, Note_on_c, 4, 32, 37
//5, 94464, Note_off_c, 4, 32, 0
//5, 94464, Note_on_c, 4, 35, 37
//5, 94848, Note_off_c, 4, 35, 0
//5, 94848, Note_on_c, 4, 35, 37
//5, 95232, Note_off_c, 4, 35, 0
//5, 95232, Note_on_c, 4, 35, 37
//5, 95424, Note_off_c, 4, 35, 0
//5, 95424, Note_on_c, 4, 27, 37
//5, 95616, Note_off_c, 4, 27, 0
//5, 95616, Note_on_c, 4, 30, 37
//5, 95808, Note_off_c, 4, 30, 0
//5, 95808, Note_on_c, 4, 27, 37
//5, 96000, Note_off_c, 4, 27, 0
//5, 96000, Note_on_c, 4, 37, 37
//5, 96384, Note_off_c, 4, 37, 0
//5, 96384, Note_on_c, 4, 37, 37
//5, 96768, Note_off_c, 4, 37, 0
//5, 96768, Note_on_c, 4, 37, 37
//5, 97152, Note_off_c, 4, 37, 0
//5, 97152, Note_on_c, 4, 37, 37
//5, 97344, Note_off_c, 4, 37, 0
//5, 97344, Note_on_c, 4, 32, 37
//5, 97440, Note_off_c, 4, 32, 0
//5, 97440, Note_on_c, 4, 38, 37
//5, 97536, Note_off_c, 4, 38, 0
//5, 97536, Note_on_c, 4, 39, 37
//5, 97728, Note_off_c, 4, 39, 0
//5, 97728, Note_on_c, 4, 39, 37
//5, 97920, Note_off_c, 4, 39, 0
//5, 97920, Note_on_c, 4, 37, 37
//5, 98112, Note_off_c, 4, 37, 0
//5, 98112, Note_on_c, 4, 33, 37
//5, 98208, Note_off_c, 4, 33, 0
//5, 98208, Note_on_c, 4, 33, 37
//5, 98400, Note_off_c, 4, 33, 0
//5, 98400, Note_on_c, 4, 32, 37
//5, 98496, Note_off_c, 4, 32, 0
//5, 98496, Note_on_c, 4, 32, 37
//5, 98688, Note_off_c, 4, 32, 0
//5, 98688, Note_on_c, 4, 30, 37
//5, 98880, Note_off_c, 4, 30, 0
//5, 98880, Note_on_c, 4, 27, 37
//5, 99072, Note_off_c, 4, 27, 0
//5, 99072, Note_on_c, 4, 39, 37
//5, 99264, Note_off_c, 4, 39, 0
//5, 99264, Note_on_c, 4, 39, 37
//5, 99456, Note_off_c, 4, 39, 0
//5, 99456, Note_on_c, 4, 37, 37
//5, 99648, Note_off_c, 4, 37, 0
//5, 99648, Note_on_c, 4, 33, 37
//5, 99744, Note_off_c, 4, 33, 0
//5, 99744, Note_on_c, 4, 33, 37
//5, 99936, Note_off_c, 4, 33, 0
//5, 99936, Note_on_c, 4, 32, 37
//5, 100032, Note_off_c, 4, 32, 0
//5, 100032, Note_on_c, 4, 32, 37
//5, 100224, Note_off_c, 4, 32, 0
//5, 100224, Note_on_c, 4, 30, 37
//5, 100416, Note_off_c, 4, 30, 0
//5, 100416, Note_on_c, 4, 27, 37
//5, 100608, Note_off_c, 4, 27, 0
//5, 100608, Note_on_c, 4, 39, 37
//5, 100800, Note_off_c, 4, 39, 0
//5, 100800, Note_on_c, 4, 39, 37
//5, 100992, Note_off_c, 4, 39, 0
//5, 100992, Note_on_c, 4, 37, 37
//5, 101184, Note_off_c, 4, 37, 0
//5, 101184, Note_on_c, 4, 33, 37
//5, 101280, Note_off_c, 4, 33, 0
//5, 101280, Note_on_c, 4, 33, 37
//5, 101472, Note_off_c, 4, 33, 0
//5, 101472, Note_on_c, 4, 32, 37
//5, 101568, Note_off_c, 4, 32, 0
//5, 101568, Note_on_c, 4, 32, 37
//5, 101760, Note_off_c, 4, 32, 0
//5, 101760, Note_on_c, 4, 30, 37
//5, 101952, Note_off_c, 4, 30, 0
//5, 101952, Note_on_c, 4, 27, 37
//5, 102144, Note_off_c, 4, 27, 0
//5, 102144, Note_on_c, 4, 39, 37
//5, 102432, Note_off_c, 4, 39, 0
//5, 102432, Note_on_c, 4, 37, 37
//5, 102720, Note_off_c, 4, 37, 0
//5, 102720, Note_on_c, 4, 33, 37
//5, 103104, Note_off_c, 4, 33, 0
//5, 103104, Note_on_c, 4, 32, 37
//5, 103296, Note_off_c, 4, 32, 0
//5, 103296, Note_on_c, 4, 30, 37
//5, 103488, Note_off_c, 4, 30, 0
//5, 103488, Note_on_c, 4, 27, 37
//5, 103680, Note_off_c, 4, 27, 0
//5, 103680, Note_on_c, 4, 37, 37
//5, 105216, Note_off_c, 4, 37, 0
//5, 105216, Note_on_c, 4, 30, 37
//5, 106752, Note_off_c, 4, 30, 0
//5, 106752, Note_on_c, 4, 37, 37
//5, 108288, Note_off_c, 4, 37, 0
//5, 108288, Note_on_c, 4, 30, 37
//5, 109440, Note_off_c, 4, 30, 0
//5, 109632, Note_on_c, 4, 35, 37
//5, 109728, Note_off_c, 4, 35, 0
//5, 109728, Note_on_c, 4, 36, 37
//5, 109824, Note_off_c, 4, 36, 0
//5, 109824, Note_on_c, 4, 37, 37
//5, 110976, Note_off_c, 4, 37, 0
//5, 111168, Note_on_c, 4, 40, 37
//5, 111264, Note_off_c, 4, 40, 0
//5, 111264, Note_on_c, 4, 41, 37
//5, 111360, Note_off_c, 4, 41, 0
//5, 111360, Note_on_c, 4, 42, 37
//5, 112320, Note_off_c, 4, 42, 0
//5, 112320, Note_on_c, 4, 32, 37
//5, 112512, Note_off_c, 4, 32, 0
//5, 112512, Note_on_c, 4, 35, 37
//5, 112704, Note_off_c, 4, 35, 0
//5, 112704, Note_on_c, 4, 36, 37
//5, 112896, Note_off_c, 4, 36, 0
//5, 112896, Note_on_c, 4, 37, 37
//5, 113280, Note_off_c, 4, 37, 0
//5, 113280, Note_on_c, 4, 37, 37
//5, 113568, Note_off_c, 4, 37, 0
//5, 113568, Note_on_c, 4, 32, 37
//5, 113664, Note_off_c, 4, 32, 0
//5, 113664, Note_on_c, 4, 37, 37
//5, 113856, Note_off_c, 4, 37, 0
//5, 113856, Note_on_c, 4, 35, 37
//5, 114048, Note_off_c, 4, 35, 0
//5, 114048, Note_on_c, 4, 32, 37
//5, 114240, Note_off_c, 4, 32, 0
//5, 114240, Note_on_c, 4, 27, 37
//5, 114432, Note_off_c, 4, 27, 0
//5, 114432, Note_on_c, 4, 30, 37
//5, 114624, Note_off_c, 4, 30, 0
//5, 114624, Note_on_c, 4, 30, 37
//5, 114816, Note_off_c, 4, 30, 0
//5, 114816, Note_on_c, 4, 30, 37
//5, 115008, Note_off_c, 4, 30, 0
//5, 115008, Note_on_c, 4, 30, 37
//5, 115200, Note_off_c, 4, 30, 0
//5, 115200, Note_on_c, 4, 30, 37
//5, 115392, Note_off_c, 4, 30, 0
//5, 115584, Note_on_c, 4, 28, 37
//5, 115968, Note_off_c, 4, 28, 0
//5, 115968, Note_on_c, 4, 30, 37
//5, 116160, Note_off_c, 4, 30, 0
//5, 116160, Note_on_c, 4, 30, 37
//5, 116352, Note_off_c, 4, 30, 0
//5, 116352, Note_on_c, 4, 30, 37
//5, 116544, Note_off_c, 4, 30, 0
//5, 116544, Note_on_c, 4, 30, 37
//5, 116736, Note_off_c, 4, 30, 0
//5, 116736, Note_on_c, 4, 30, 37
//5, 116928, Note_off_c, 4, 30, 0
//5, 117120, Note_on_c, 4, 28, 37
//5, 117504, Note_off_c, 4, 28, 0
//5, 117504, Note_on_c, 4, 30, 37
//5, 117696, Note_off_c, 4, 30, 0
//5, 117696, Note_on_c, 4, 30, 37
//5, 117888, Note_off_c, 4, 30, 0
//5, 117888, Note_on_c, 4, 30, 37
//5, 118080, Note_off_c, 4, 30, 0
//5, 118080, Note_on_c, 4, 30, 37
//5, 118272, Note_off_c, 4, 30, 0
//5, 118272, Note_on_c, 4, 30, 37
//5, 118464, Note_off_c, 4, 30, 0
//5, 118656, Note_on_c, 4, 28, 37
//5, 119040, Note_off_c, 4, 28, 0
//5, 119040, Note_on_c, 4, 32, 37
//5, 119232, Note_off_c, 4, 32, 0
//5, 119232, Note_on_c, 4, 32, 37
//5, 119424, Note_off_c, 4, 32, 0
//5, 119424, Note_on_c, 4, 32, 37
//5, 119616, Note_off_c, 4, 32, 0
//5, 119616, Note_on_c, 4, 32, 37
//5, 119808, Note_off_c, 4, 32, 0
//5, 119808, Note_on_c, 4, 42, 37
//5, 119904, Note_off_c, 4, 42, 0
//5, 120000, Note_on_c, 4, 42, 37
//5, 120096, Note_off_c, 4, 42, 0
//5, 120192, Note_on_c, 4, 42, 37
//5, 120288, Note_off_c, 4, 42, 0
//5, 120384, Note_on_c, 4, 42, 37
//5, 120480, Note_off_c, 4, 42, 0
//5, 120576, Note_on_c, 4, 27, 37
//5, 120768, Note_off_c, 4, 27, 0
//5, 120768, Note_on_c, 4, 36, 37
//5, 120960, Note_off_c, 4, 36, 0
//5, 120960, Note_on_c, 4, 37, 37
//5, 121152, Note_off_c, 4, 37, 0
//5, 121152, Note_on_c, 4, 27, 37
//5, 121248, Note_off_c, 4, 27, 0
//5, 121248, Note_on_c, 4, 27, 37
//5, 121344, Note_off_c, 4, 27, 0
//5, 121344, Note_on_c, 4, 27, 37
//5, 121536, Note_off_c, 4, 27, 0
//5, 121536, Note_on_c, 4, 39, 37
//5, 121728, Note_off_c, 4, 39, 0
//5, 121728, Note_on_c, 4, 39, 37
//5, 121920, Note_off_c, 4, 39, 0
//5, 121920, Note_on_c, 4, 27, 37
//5, 122016, Note_off_c, 4, 27, 0
//5, 122016, Note_on_c, 4, 27, 37
//5, 122112, Note_off_c, 4, 27, 0
//5, 122112, Note_on_c, 4, 27, 37
//5, 122304, Note_off_c, 4, 27, 0
//5, 122304, Note_on_c, 4, 36, 37
//5, 122496, Note_off_c, 4, 36, 0
//5, 122496, Note_on_c, 4, 37, 37
//5, 122688, Note_off_c, 4, 37, 0
//5, 122688, Note_on_c, 4, 27, 37
//5, 122784, Note_off_c, 4, 27, 0
//5, 122784, Note_on_c, 4, 27, 37
//5, 122880, Note_off_c, 4, 27, 0
//5, 122880, Note_on_c, 4, 30, 37
//5, 122976, Note_off_c, 4, 30, 0
//5, 123072, Note_on_c, 4, 30, 37
//5, 123168, Note_off_c, 4, 30, 0
//5, 123264, Note_on_c, 4, 30, 37
//5, 123360, Note_off_c, 4, 30, 0
//5, 123456, Note_on_c, 4, 27, 37
//5, 123552, Note_off_c, 4, 27, 0
//5, 123552, Note_on_c, 4, 27, 37
//5, 123648, Note_off_c, 4, 27, 0
//5, 123648, Note_on_c, 4, 27, 37
//5, 123840, Note_off_c, 4, 27, 0
//5, 123840, Note_on_c, 4, 36, 37
//5, 124032, Note_off_c, 4, 36, 0
//5, 124032, Note_on_c, 4, 37, 37
//5, 124224, Note_off_c, 4, 37, 0
//5, 124224, Note_on_c, 4, 27, 37
//5, 124320, Note_off_c, 4, 27, 0
//5, 124320, Note_on_c, 4, 27, 37
//5, 124416, Note_off_c, 4, 27, 0
//5, 124416, Note_on_c, 4, 27, 37
//5, 124608, Note_off_c, 4, 27, 0
//5, 124608, Note_on_c, 4, 39, 37
//5, 124800, Note_off_c, 4, 39, 0
//5, 124800, Note_on_c, 4, 39, 37
//5, 124992, Note_off_c, 4, 39, 0
//5, 124992, Note_on_c, 4, 27, 37
//5, 125088, Note_off_c, 4, 27, 0
//5, 125088, Note_on_c, 4, 27, 37
//5, 125184, Note_off_c, 4, 27, 0
//5, 125184, Note_on_c, 4, 27, 37
//5, 125376, Note_off_c, 4, 27, 0
//5, 125376, Note_on_c, 4, 36, 37
//5, 125568, Note_off_c, 4, 36, 0
//5, 125568, Note_on_c, 4, 37, 37
//5, 125760, Note_off_c, 4, 37, 0
//5, 125760, Note_on_c, 4, 27, 37
//5, 125856, Note_off_c, 4, 27, 0
//5, 125856, Note_on_c, 4, 27, 37
//5, 125952, Note_off_c, 4, 27, 0
//5, 125952, Note_on_c, 4, 30, 37
//5, 126048, Note_off_c, 4, 30, 0
//5, 126144, Note_on_c, 4, 30, 37
//5, 126240, Note_off_c, 4, 30, 0
//5, 126336, Note_on_c, 4, 30, 37
//5, 126720, Note_off_c, 4, 30, 0
//5, 126720, Note_on_c, 4, 36, 37
//5, 126912, Note_off_c, 4, 36, 0
//5, 126912, Note_on_c, 4, 34, 37
//5, 127104, Note_off_c, 4, 34, 0
//5, 127104, Note_on_c, 4, 36, 37
//5, 127296, Note_off_c, 4, 36, 0
//5, 127296, Note_on_c, 4, 34, 37
//5, 127392, Note_off_c, 4, 34, 0
//5, 127392, Note_on_c, 4, 34, 37
//5, 127488, Note_off_c, 4, 34, 0
//5, 127488, Note_on_c, 4, 36, 37
//5, 127680, Note_off_c, 4, 36, 0
//5, 127680, Note_on_c, 4, 34, 37
//5, 127872, Note_off_c, 4, 34, 0
//5, 127872, Note_on_c, 4, 36, 37
//5, 128064, Note_off_c, 4, 36, 0
//5, 128064, Note_on_c, 4, 34, 37
//5, 128448, Note_off_c, 4, 34, 0
//5, 128448, Note_on_c, 4, 32, 37
//5, 128640, Note_off_c, 4, 32, 0
//5, 128640, Note_on_c, 4, 34, 37
//5, 128832, Note_off_c, 4, 34, 0
//5, 128832, Note_on_c, 4, 32, 37
//5, 128928, Note_off_c, 4, 32, 0
//5, 128928, Note_on_c, 4, 32, 37
//5, 129024, Note_off_c, 4, 32, 0
//5, 129024, Note_on_c, 4, 34, 37
//5, 129216, Note_off_c, 4, 34, 0
//5, 129216, Note_on_c, 4, 32, 37
//5, 129408, Note_off_c, 4, 32, 0
//5, 129408, Note_on_c, 4, 34, 37
//5, 129600, Note_off_c, 4, 34, 0
//5, 129600, Note_on_c, 4, 27, 37
//5, 129696, Note_off_c, 4, 27, 0
//5, 129696, Note_on_c, 4, 27, 37
//5, 129792, Note_off_c, 4, 27, 0
//5, 129792, Note_on_c, 4, 36, 37
//5, 129984, Note_off_c, 4, 36, 0
//5, 129984, Note_on_c, 4, 34, 37
//5, 130176, Note_off_c, 4, 34, 0
//5, 130176, Note_on_c, 4, 36, 37
//5, 130368, Note_off_c, 4, 36, 0
//5, 130368, Note_on_c, 4, 34, 37
//5, 130464, Note_off_c, 4, 34, 0
//5, 130464, Note_on_c, 4, 34, 37
//5, 130560, Note_off_c, 4, 34, 0
//5, 130560, Note_on_c, 4, 36, 37
//5, 130752, Note_off_c, 4, 36, 0
//5, 130752, Note_on_c, 4, 34, 37
//5, 130944, Note_off_c, 4, 34, 0
//5, 130944, Note_on_c, 4, 36, 37
//5, 131136, Note_off_c, 4, 36, 0
//5, 131136, Note_on_c, 4, 34, 37
//5, 131520, Note_off_c, 4, 34, 0
//5, 131520, Note_on_c, 4, 32, 37
//5, 131712, Note_off_c, 4, 32, 0
//5, 131712, Note_on_c, 4, 34, 37
//5, 131904, Note_off_c, 4, 34, 0
//5, 131904, Note_on_c, 4, 32, 37
//5, 132000, Note_off_c, 4, 32, 0
//5, 132000, Note_on_c, 4, 32, 37
//5, 132096, Note_off_c, 4, 32, 0
//5, 132096, Note_on_c, 4, 34, 37
//5, 132288, Note_off_c, 4, 34, 0
//5, 132288, Note_on_c, 4, 32, 37
//5, 132480, Note_off_c, 4, 32, 0
//5, 132480, Note_on_c, 4, 34, 37
//5, 132672, Note_off_c, 4, 34, 0
//5, 132672, Note_on_c, 4, 27, 37
//5, 132768, Note_off_c, 4, 27, 0
//5, 132768, Note_on_c, 4, 27, 37
//5, 132864, Note_off_c, 4, 27, 0
//5, 132864, Note_on_c, 4, 36, 37
//5, 133056, Note_off_c, 4, 36, 0
//5, 133056, Note_on_c, 4, 34, 37
//5, 133248, Note_off_c, 4, 34, 0
//5, 133248, Note_on_c, 4, 36, 37
//5, 133440, Note_off_c, 4, 36, 0
//5, 133440, Note_on_c, 4, 34, 37
//5, 133536, Note_off_c, 4, 34, 0
//5, 133536, Note_on_c, 4, 34, 37
//5, 133632, Note_off_c, 4, 34, 0
//5, 133632, Note_on_c, 4, 36, 37
//5, 133824, Note_off_c, 4, 36, 0
//5, 133824, Note_on_c, 4, 34, 37
//5, 134016, Note_off_c, 4, 34, 0
//5, 134016, Note_on_c, 4, 36, 37
//5, 134208, Note_off_c, 4, 36, 0
//5, 134208, Note_on_c, 4, 34, 37
//5, 134592, Note_off_c, 4, 34, 0
//5, 134592, Note_on_c, 4, 32, 37
//5, 134784, Note_off_c, 4, 32, 0
//5, 134784, Note_on_c, 4, 34, 37
//5, 134976, Note_off_c, 4, 34, 0
//5, 134976, Note_on_c, 4, 27, 37
//5, 135168, Note_off_c, 4, 27, 0
//5, 135168, Note_on_c, 4, 34, 37
//5, 135360, Note_off_c, 4, 34, 0
//5, 135360, Note_on_c, 4, 27, 37
//5, 135552, Note_off_c, 4, 27, 0
//5, 135552, Note_on_c, 4, 30, 37
//5, 135744, Note_off_c, 4, 30, 0
//5, 135744, Note_on_c, 4, 27, 37
//5, 136128, Note_off_c, 4, 27, 0
//5, 136128, Note_on_c, 4, 27, 37
//5, 136320, Note_off_c, 4, 27, 0
//5, 136320, Note_on_c, 4, 39, 37
//5, 136512, Note_off_c, 4, 39, 0
//5, 136512, Note_on_c, 4, 27, 37
//5, 136704, Note_off_c, 4, 27, 0
//5, 136704, Note_on_c, 4, 27, 37
//5, 136896, Note_off_c, 4, 27, 0
//5, 136896, Note_on_c, 4, 27, 37
//5, 137088, Note_off_c, 4, 27, 0
//5, 137088, Note_on_c, 4, 39, 37
//5, 137280, Note_off_c, 4, 39, 0
//5, 137280, Note_on_c, 4, 27, 37
//5, 137472, Note_off_c, 4, 27, 0
//5, 137472, Note_on_c, 4, 27, 37
//5, 137664, Note_off_c, 4, 27, 0
//5, 137664, Note_on_c, 4, 27, 37
//5, 137856, Note_off_c, 4, 27, 0
//5, 137856, Note_on_c, 4, 39, 37
//5, 138048, Note_off_c, 4, 39, 0
//5, 138048, Note_on_c, 4, 27, 37
//5, 138240, Note_off_c, 4, 27, 0
//5, 138240, Note_on_c, 4, 27, 37
//5, 138432, Note_off_c, 4, 27, 0
//5, 138432, Note_on_c, 4, 32, 37
//5, 138624, Note_off_c, 4, 32, 0
//5, 138624, Note_on_c, 4, 33, 37
//5, 138816, Note_off_c, 4, 33, 0
//5, 138816, Note_on_c, 4, 34, 37
//5, 139200, Note_off_c, 4, 34, 0
//5, 139200, Note_on_c, 4, 34, 37
//5, 139392, Note_off_c, 4, 34, 0
//5, 139392, Note_on_c, 4, 34, 37
//5, 139584, Note_off_c, 4, 34, 0
//5, 139584, Note_on_c, 4, 27, 37
//5, 139680, Note_off_c, 4, 27, 0
//5, 139680, Note_on_c, 4, 27, 37
//5, 139776, Note_off_c, 4, 27, 0
//5, 139776, Note_on_c, 4, 34, 37
//5, 139968, Note_off_c, 4, 34, 0
//5, 139968, Note_on_c, 4, 32, 37
//5, 140160, Note_off_c, 4, 32, 0
//5, 140160, Note_on_c, 4, 33, 37
//5, 140352, Note_off_c, 4, 33, 0
//5, 140352, Note_on_c, 4, 34, 37
//5, 140544, Note_off_c, 4, 34, 0
//5, 140544, Note_on_c, 4, 27, 37
//5, 140736, Note_off_c, 4, 27, 0
//5, 140736, Note_on_c, 4, 34, 37
//5, 140928, Note_off_c, 4, 34, 0
//5, 140928, Note_on_c, 4, 27, 37
//5, 141120, Note_off_c, 4, 27, 0
//5, 141120, Note_on_c, 4, 27, 37
//5, 141312, Note_off_c, 4, 27, 0
//5, 141312, Note_on_c, 4, 46, 37
//5, 141504, Note_off_c, 4, 46, 0
//5, 141504, Note_on_c, 4, 44, 37
//5, 141696, Note_off_c, 4, 44, 0
//5, 141696, Note_on_c, 4, 41, 37
//5, 141888, Note_off_c, 4, 41, 0
//5, 141888, Note_on_c, 4, 39, 37
//5, 142272, Note_off_c, 4, 39, 0
//5, 142272, Note_on_c, 4, 39, 37
//5, 142464, Note_off_c, 4, 39, 0
//5, 142464, Note_on_c, 4, 37, 37
//5, 142656, Note_off_c, 4, 37, 0
//5, 142656, Note_on_c, 4, 34, 37
//5, 142848, Note_off_c, 4, 34, 0
//5, 142848, Note_on_c, 4, 37, 37
//5, 143040, Note_off_c, 4, 37, 0
//5, 143040, Note_on_c, 4, 39, 37
//5, 143232, Note_off_c, 4, 39, 0
//5, 143232, Note_on_c, 4, 32, 37
//5, 143424, Note_off_c, 4, 32, 0
//5, 143424, Note_on_c, 4, 39, 37
//5, 143808, Note_off_c, 4, 39, 0
//5, 143808, Note_on_c, 4, 39, 37
//5, 144000, Note_off_c, 4, 39, 0
//5, 144000, Note_on_c, 4, 37, 37
//5, 144192, Note_off_c, 4, 37, 0
//5, 144192, Note_on_c, 4, 34, 37
//5, 144384, Note_off_c, 4, 34, 0
//5, 144384, Note_on_c, 4, 37, 37
//5, 144576, Note_off_c, 4, 37, 0
//5, 144576, Note_on_c, 4, 32, 37
//5, 144768, Note_off_c, 4, 32, 0
//5, 144768, Note_on_c, 4, 33, 37
//5, 144960, Note_off_c, 4, 33, 0
//5, 144960, Note_on_c, 4, 34, 37
//5, 145344, Note_off_c, 4, 34, 0
//5, 145344, Note_on_c, 4, 34, 37
//5, 145536, Note_off_c, 4, 34, 0
//5, 145536, Note_on_c, 4, 34, 37
//5, 145728, Note_off_c, 4, 34, 0
//5, 145728, Note_on_c, 4, 27, 37
//5, 145824, Note_off_c, 4, 27, 0
//5, 145824, Note_on_c, 4, 27, 37
//5, 145920, Note_off_c, 4, 27, 0
//5, 145920, Note_on_c, 4, 34, 37
//5, 146112, Note_off_c, 4, 34, 0
//5, 146112, Note_on_c, 4, 34, 37
//5, 146304, Note_off_c, 4, 34, 0
//5, 146304, Note_on_c, 4, 27, 37
//5, 146496, Note_off_c, 4, 27, 0
//5, 146496, Note_on_c, 4, 34, 37
//5, 146880, Note_off_c, 4, 34, 0
//5, 146880, Note_on_c, 4, 34, 37
//5, 147072, Note_off_c, 4, 34, 0
//5, 147072, Note_on_c, 4, 34, 37
//5, 147264, Note_off_c, 4, 34, 0
//5, 147264, Note_on_c, 4, 27, 37
//5, 147328, Note_off_c, 4, 27, 0
//5, 147328, Note_on_c, 4, 27, 37
//5, 147391, Note_on_c, 4, 27, 37
//5, 147391, Note_off_c, 4, 27, 0
//5, 147455, Note_off_c, 4, 27, 0
//5, 147455, Note_on_c, 4, 34, 37
//5, 147647, Note_off_c, 4, 34, 0
//5, 147839, Note_on_c, 4, 39, 37
//5, 148223, Note_off_c, 4, 39, 0
//5, 148223, Note_on_c, 4, 29, 37
//5, 148415, Note_off_c, 4, 29, 0
//5, 148415, Note_on_c, 4, 29, 37
//5, 148607, Note_off_c, 4, 29, 0
//5, 148607, Note_on_c, 4, 29, 37
//5, 148799, Note_off_c, 4, 29, 0
//5, 148799, Note_on_c, 4, 29, 37
//5, 148991, Note_off_c, 4, 29, 0
//5, 148991, Note_on_c, 4, 29, 37
//5, 149183, Note_off_c, 4, 29, 0
//5, 149183, Note_on_c, 4, 29, 37
//5, 149375, Note_off_c, 4, 29, 0
//5, 149375, Note_on_c, 4, 27, 37
//5, 149567, Note_off_c, 4, 27, 0
//5, 149567, Note_on_c, 4, 39, 37
//5, 149663, Note_off_c, 4, 39, 0
//5, 149663, Note_on_c, 4, 40, 37
//5, 149759, Note_off_c, 4, 40, 0
//5, 149759, Note_on_c, 4, 41, 37
//5, 149951, Note_off_c, 4, 41, 0
//5, 149951, Note_on_c, 4, 41, 37
//5, 150143, Note_off_c, 4, 41, 0
//5, 150143, Note_on_c, 4, 41, 37
//5, 150335, Note_off_c, 4, 41, 0
//5, 150335, Note_on_c, 4, 41, 37
//5, 150527, Note_off_c, 4, 41, 0
//5, 150527, Note_on_c, 4, 41, 37
//5, 150719, Note_off_c, 4, 41, 0
//5, 150719, Note_on_c, 4, 39, 37
//5, 150911, Note_off_c, 4, 39, 0
//5, 150911, Note_on_c, 4, 36, 37
//5, 151103, Note_off_c, 4, 36, 0
//5, 151103, Note_on_c, 4, 32, 37
//5, 151295, Note_off_c, 4, 32, 0
//5, 151295, Note_on_c, 4, 34, 37
//5, 152447, Note_off_c, 4, 34, 0
//5, 152447, Note_on_c, 4, 34, 37
//5, 152831, Note_off_c, 4, 34, 0
//5, 152831, Note_on_c, 4, 46, 37
//5, 154367, Note_off_c, 4, 46, 0
//5, 154367, Note_on_c, 4, 27, 37
//5, 154463, Note_off_c, 4, 27, 0
//5, 154463, Note_on_c, 4, 27, 37
//5, 154559, Note_off_c, 4, 27, 0
//5, 154559, Note_on_c, 4, 27, 37
//5, 154655, Note_off_c, 4, 27, 0
//5, 154655, Note_on_c, 4, 27, 37
//5, 154751, Note_off_c, 4, 27, 0
//5, 154751, Note_on_c, 4, 29, 37
//5, 154847, Note_off_c, 4, 29, 0
//5, 154847, Note_on_c, 4, 29, 37
//5, 154943, Note_off_c, 4, 29, 0
//5, 154943, Note_on_c, 4, 29, 37
//5, 155039, Note_off_c, 4, 29, 0
//5, 155039, Note_on_c, 4, 30, 37
//5, 155135, Note_off_c, 4, 30, 0
//5, 155135, Note_on_c, 4, 30, 37
//5, 155231, Note_off_c, 4, 30, 0
//5, 155231, Note_on_c, 4, 30, 37
//5, 155327, Note_off_c, 4, 30, 0
//5, 155327, Note_on_c, 4, 27, 37
//5, 155423, Note_off_c, 4, 27, 0
//5, 155423, Note_on_c, 4, 27, 37
//5, 155519, Note_off_c, 4, 27, 0
//5, 155519, Note_on_c, 4, 29, 37
//5, 155615, Note_off_c, 4, 29, 0
//5, 155615, Note_on_c, 4, 29, 37
//5, 155711, Note_off_c, 4, 29, 0
//5, 155711, Note_on_c, 4, 30, 37
//5, 155807, Note_off_c, 4, 30, 0
//5, 155807, Note_on_c, 4, 30, 37
//5, 155903, Note_off_c, 4, 30, 0
//5, 155903, Note_on_c, 4, 29, 37
//5, 155999, Note_off_c, 4, 29, 0
//5, 155999, Note_on_c, 4, 29, 37
//5, 156095, Note_off_c, 4, 29, 0
//5, 156095, Note_on_c, 4, 29, 37
//5, 156191, Note_off_c, 4, 29, 0
//5, 156191, Note_on_c, 4, 29, 37
//5, 156287, Note_off_c, 4, 29, 0
//5, 156287, Note_on_c, 4, 30, 37
//5, 156383, Note_off_c, 4, 30, 0
//5, 156383, Note_on_c, 4, 30, 37
//5, 156479, Note_off_c, 4, 30, 0
//5, 156479, Note_on_c, 4, 30, 37
//5, 156575, Note_off_c, 4, 30, 0
//5, 156575, Note_on_c, 4, 31, 37
//5, 156671, Note_off_c, 4, 31, 0
//5, 156671, Note_on_c, 4, 31, 37
//5, 156767, Note_off_c, 4, 31, 0
//5, 156767, Note_on_c, 4, 31, 37
//5, 156863, Note_off_c, 4, 31, 0
//5, 156863, Note_on_c, 4, 30, 37
//5, 156959, Note_off_c, 4, 30, 0
//5, 156959, Note_on_c, 4, 30, 37
//5, 157055, Note_off_c, 4, 30, 0
//5, 157055, Note_on_c, 4, 31, 37
//5, 157151, Note_off_c, 4, 31, 0
//5, 157151, Note_on_c, 4, 31, 37
//5, 157247, Note_off_c, 4, 31, 0
//5, 157247, Note_on_c, 4, 32, 37
//5, 157343, Note_off_c, 4, 32, 0
//5, 157343, Note_on_c, 4, 32, 37
//5, 157439, Note_off_c, 4, 32, 0
//5, 157439, Note_on_c, 4, 39, 37
//5, 157535, Note_off_c, 4, 39, 0
//5, 157535, Note_on_c, 4, 39, 37
//5, 157631, Note_off_c, 4, 39, 0
//5, 157631, Note_on_c, 4, 39, 37
//5, 157727, Note_off_c, 4, 39, 0
//5, 157727, Note_on_c, 4, 39, 37
//5, 157823, Note_off_c, 4, 39, 0
//5, 157823, Note_on_c, 4, 38, 37
//5, 157919, Note_off_c, 4, 38, 0
//5, 157919, Note_on_c, 4, 38, 37
//5, 158015, Note_off_c, 4, 38, 0
//5, 158015, Note_on_c, 4, 38, 37
//5, 158111, Note_off_c, 4, 38, 0
//5, 158111, Note_on_c, 4, 37, 37
//5, 158207, Note_off_c, 4, 37, 0
//5, 158207, Note_on_c, 4, 37, 37
//5, 158303, Note_off_c, 4, 37, 0
//5, 158303, Note_on_c, 4, 37, 37
//5, 158399, Note_off_c, 4, 37, 0
//5, 158399, Note_on_c, 4, 36, 37
//5, 158495, Note_off_c, 4, 36, 0
//5, 158495, Note_on_c, 4, 36, 37
//5, 158591, Note_off_c, 4, 36, 0
//5, 158591, Note_on_c, 4, 35, 37
//5, 158687, Note_off_c, 4, 35, 0
//5, 158687, Note_on_c, 4, 35, 37
//5, 158783, Note_off_c, 4, 35, 0
//5, 158783, Note_on_c, 4, 34, 37
//5, 158879, Note_off_c, 4, 34, 0
//5, 158879, Note_on_c, 4, 34, 37
//5, 158975, Note_off_c, 4, 34, 0
//5, 158975, Note_on_c, 4, 39, 37
//5, 159071, Note_off_c, 4, 39, 0
//5, 159071, Note_on_c, 4, 39, 37
//5, 159167, Note_off_c, 4, 39, 0
//5, 159167, Note_on_c, 4, 39, 37
//5, 159263, Note_off_c, 4, 39, 0
//5, 159263, Note_on_c, 4, 39, 37
//5, 159359, Note_off_c, 4, 39, 0
//5, 159359, Note_on_c, 4, 38, 37
//5, 159455, Note_off_c, 4, 38, 0
//5, 159455, Note_on_c, 4, 38, 37
//5, 159551, Note_off_c, 4, 38, 0
//5, 159551, Note_on_c, 4, 38, 37
//5, 159647, Note_off_c, 4, 38, 0
//5, 159647, Note_on_c, 4, 37, 37
//5, 159743, Note_off_c, 4, 37, 0
//5, 159743, Note_on_c, 4, 37, 37
//5, 159839, Note_off_c, 4, 37, 0
//5, 159839, Note_on_c, 4, 37, 37
//5, 159935, Note_off_c, 4, 37, 0
//5, 159935, Note_on_c, 4, 36, 37
//5, 160031, Note_off_c, 4, 36, 0
//5, 160031, Note_on_c, 4, 36, 37
//5, 160127, Note_off_c, 4, 36, 0
//5, 160127, Note_on_c, 4, 35, 37
//5, 160223, Note_off_c, 4, 35, 0
//5, 160223, Note_on_c, 4, 35, 37
//5, 160319, Note_off_c, 4, 35, 0
//5, 160319, Note_on_c, 4, 34, 37
//5, 160415, Note_off_c, 4, 34, 0
//5, 160415, Note_on_c, 4, 34, 37
//5, 160511, Note_off_c, 4, 34, 0
//5, 160511, Note_on_c, 4, 39, 37
//5, 160607, Note_off_c, 4, 39, 0
//5, 160607, Note_on_c, 4, 39, 37
//5, 160703, Note_off_c, 4, 39, 0
//5, 160703, Note_on_c, 4, 39, 37
//5, 160799, Note_off_c, 4, 39, 0
//5, 160799, Note_on_c, 4, 39, 37
//5, 160895, Note_off_c, 4, 39, 0
//5, 160895, Note_on_c, 4, 38, 37
//5, 160991, Note_off_c, 4, 38, 0
//5, 160991, Note_on_c, 4, 38, 37
//5, 161087, Note_off_c, 4, 38, 0
//5, 161087, Note_on_c, 4, 38, 37
//5, 161183, Note_off_c, 4, 38, 0
//5, 161183, Note_on_c, 4, 37, 37
//5, 161279, Note_off_c, 4, 37, 0
//5, 161279, Note_on_c, 4, 37, 37
//5, 161375, Note_off_c, 4, 37, 0
//5, 161375, Note_on_c, 4, 37, 37
//5, 161471, Note_off_c, 4, 37, 0
//5, 161471, Note_on_c, 4, 36, 37
//5, 161567, Note_off_c, 4, 36, 0
//5, 161567, Note_on_c, 4, 36, 37
//5, 161663, Note_off_c, 4, 36, 0
//5, 161663, Note_on_c, 4, 35, 37
//5, 161759, Note_off_c, 4, 35, 0
//5, 161759, Note_on_c, 4, 35, 37
//5, 161855, Note_off_c, 4, 35, 0
//5, 161855, Note_on_c, 4, 34, 37
//5, 161951, Note_off_c, 4, 34, 0
//5, 161951, Note_on_c, 4, 34, 37
//5, 162047, Note_off_c, 4, 34, 0
//5, 162047, Note_on_c, 4, 39, 37
//5, 162143, Note_off_c, 4, 39, 0
//5, 162143, Note_on_c, 4, 39, 37
//5, 162239, Note_off_c, 4, 39, 0
//5, 162239, Note_on_c, 4, 39, 37
//5, 162335, Note_off_c, 4, 39, 0
//5, 162335, Note_on_c, 4, 39, 37
//5, 162431, Note_off_c, 4, 39, 0
//5, 162431, Note_on_c, 4, 38, 37
//5, 162527, Note_off_c, 4, 38, 0
//5, 162527, Note_on_c, 4, 38, 37
//5, 162623, Note_off_c, 4, 38, 0
//5, 162623, Note_on_c, 4, 38, 37
//5, 162719, Note_off_c, 4, 38, 0
//5, 162719, Note_on_c, 4, 37, 37
//5, 162815, Note_off_c, 4, 37, 0
//5, 162815, Note_on_c, 4, 37, 37
//5, 162911, Note_off_c, 4, 37, 0
//5, 162911, Note_on_c, 4, 37, 37
//5, 163007, Note_off_c, 4, 37, 0
//5, 163007, Note_on_c, 4, 36, 37
//5, 163103, Note_off_c, 4, 36, 0
//5, 163103, Note_on_c, 4, 36, 37
//5, 163199, Note_off_c, 4, 36, 0
//5, 163199, Note_on_c, 4, 35, 37
//5, 163295, Note_off_c, 4, 35, 0
//5, 163295, Note_on_c, 4, 35, 37
//5, 163391, Note_off_c, 4, 35, 0
//5, 163391, Note_on_c, 4, 34, 37
//5, 163487, Note_off_c, 4, 34, 0
//5, 163487, Note_on_c, 4, 34, 37
//5, 163583, Note_off_c, 4, 34, 0
//5, 163583, Note_on_c, 4, 39, 37
//5, 163679, Note_off_c, 4, 39, 0
//5, 163679, Note_on_c, 4, 39, 37
//5, 163775, Note_off_c, 4, 39, 0
//5, 163775, Note_on_c, 4, 39, 37
//5, 163871, Note_off_c, 4, 39, 0
//5, 163871, Note_on_c, 4, 39, 37
//5, 163967, Note_off_c, 4, 39, 0
//5, 163967, Note_on_c, 4, 38, 37
//5, 164063, Note_off_c, 4, 38, 0
//5, 164063, Note_on_c, 4, 38, 37
//5, 164159, Note_off_c, 4, 38, 0
//5, 164159, Note_on_c, 4, 38, 37
//5, 164255, Note_off_c, 4, 38, 0
//5, 164255, Note_on_c, 4, 37, 37
//5, 164351, Note_off_c, 4, 37, 0
//5, 164351, Note_on_c, 4, 37, 37
//5, 164447, Note_off_c, 4, 37, 0
//5, 164447, Note_on_c, 4, 37, 37
//5, 164543, Note_off_c, 4, 37, 0
//5, 164543, Note_on_c, 4, 36, 37
//5, 164639, Note_off_c, 4, 36, 0
//5, 164639, Note_on_c, 4, 36, 37
//5, 164735, Note_off_c, 4, 36, 0
//5, 164735, Note_on_c, 4, 35, 37
//5, 164831, Note_off_c, 4, 35, 0
//5, 164831, Note_on_c, 4, 35, 37
//5, 164927, Note_off_c, 4, 35, 0
//5, 164927, Note_on_c, 4, 34, 37
//5, 165023, Note_off_c, 4, 34, 0
//5, 165023, Note_on_c, 4, 34, 37
//5, 165119, Note_off_c, 4, 34, 0
//5, 165119, Note_on_c, 4, 39, 37
//5, 165215, Note_off_c, 4, 39, 0
//5, 165215, Note_on_c, 4, 39, 37
//5, 165311, Note_off_c, 4, 39, 0
//5, 165311, Note_on_c, 4, 39, 37
//5, 165407, Note_off_c, 4, 39, 0
//5, 165407, Note_on_c, 4, 39, 37
//5, 165503, Note_off_c, 4, 39, 0
//5, 165503, Note_on_c, 4, 38, 37
//5, 165599, Note_off_c, 4, 38, 0
//5, 165599, Note_on_c, 4, 38, 37
//5, 165695, Note_off_c, 4, 38, 0
//5, 165695, Note_on_c, 4, 38, 37
//5, 165791, Note_off_c, 4, 38, 0
//5, 165791, Note_on_c, 4, 37, 37
//5, 165887, Note_off_c, 4, 37, 0
//5, 165887, Note_on_c, 4, 37, 37
//5, 165983, Note_off_c, 4, 37, 0
//5, 165983, Note_on_c, 4, 37, 37
//5, 166079, Note_off_c, 4, 37, 0
//5, 166079, Note_on_c, 4, 36, 37
//5, 166175, Note_off_c, 4, 36, 0
//5, 166175, Note_on_c, 4, 36, 37
//5, 166271, Note_off_c, 4, 36, 0
//5, 166271, Note_on_c, 4, 35, 37
//5, 166367, Note_off_c, 4, 35, 0
//5, 166367, Note_on_c, 4, 35, 37
//5, 166463, Note_off_c, 4, 35, 0
//5, 166463, Note_on_c, 4, 34, 37
//5, 166559, Note_off_c, 4, 34, 0
//5, 166559, Note_on_c, 4, 34, 37
//5, 166655, Note_off_c, 4, 34, 0
//5, 166655, Note_on_c, 4, 39, 37
//5, 166751, Note_off_c, 4, 39, 0
//5, 166751, Note_on_c, 4, 39, 37
//5, 166847, Note_off_c, 4, 39, 0
//5, 166847, Note_on_c, 4, 39, 37
//5, 166943, Note_off_c, 4, 39, 0
//5, 166943, Note_on_c, 4, 39, 37
//5, 167039, Note_off_c, 4, 39, 0
//5, 167039, Note_on_c, 4, 38, 37
//5, 167135, Note_off_c, 4, 38, 0
//5, 167135, Note_on_c, 4, 38, 37
//5, 167231, Note_off_c, 4, 38, 0
//5, 167231, Note_on_c, 4, 38, 37
//5, 167327, Note_off_c, 4, 38, 0
//5, 167327, Note_on_c, 4, 37, 37
//5, 167423, Note_off_c, 4, 37, 0
//5, 167423, Note_on_c, 4, 37, 37
//5, 167519, Note_off_c, 4, 37, 0
//5, 167519, Note_on_c, 4, 37, 37
//5, 167615, Note_off_c, 4, 37, 0
//5, 167615, Note_on_c, 4, 36, 37
//5, 167711, Note_off_c, 4, 36, 0
//5, 167711, Note_on_c, 4, 36, 37
//5, 167807, Note_off_c, 4, 36, 0
//5, 167807, Note_on_c, 4, 35, 37
//5, 167903, Note_off_c, 4, 35, 0
//5, 167903, Note_on_c, 4, 35, 37
//5, 167999, Note_off_c, 4, 35, 0
//5, 167999, Note_on_c, 4, 34, 37
//5, 168095, Note_off_c, 4, 34, 0
//5, 168095, Note_on_c, 4, 34, 37
//5, 168191, Note_off_c, 4, 34, 0
//5, 168191, Note_on_c, 4, 39, 37
//5, 168287, Note_off_c, 4, 39, 0
//5, 168287, Note_on_c, 4, 39, 37
//5, 168383, Note_off_c, 4, 39, 0
//5, 168383, Note_on_c, 4, 39, 37
//5, 168479, Note_off_c, 4, 39, 0
//5, 168479, Note_on_c, 4, 39, 37
//5, 168575, Note_off_c, 4, 39, 0
//5, 168575, Note_on_c, 4, 38, 37
//5, 168671, Note_off_c, 4, 38, 0
//5, 168671, Note_on_c, 4, 38, 37
//5, 168767, Note_off_c, 4, 38, 0
//5, 168767, Note_on_c, 4, 38, 37
//5, 168863, Note_off_c, 4, 38, 0
//5, 168863, Note_on_c, 4, 37, 37
//5, 168959, Note_off_c, 4, 37, 0
//5, 168959, Note_on_c, 4, 37, 37
//5, 169055, Note_off_c, 4, 37, 0
//5, 169055, Note_on_c, 4, 37, 37
//5, 169151, Note_off_c, 4, 37, 0
//5, 169151, Note_on_c, 4, 36, 37
//5, 169247, Note_off_c, 4, 36, 0
//5, 169247, Note_on_c, 4, 36, 37
//5, 169343, Note_off_c, 4, 36, 0
//5, 169343, Note_on_c, 4, 35, 37
//5, 169439, Note_off_c, 4, 35, 0
//5, 169439, Note_on_c, 4, 35, 37
//5, 169535, Note_off_c, 4, 35, 0
//5, 169535, Note_on_c, 4, 34, 37
//5, 169631, Note_off_c, 4, 34, 0
//5, 169631, Note_on_c, 4, 34, 37
//5, 169727, Note_off_c, 4, 34, 0
//5, 169727, Note_on_c, 4, 39, 37
//5, 169823, Note_off_c, 4, 39, 0
//5, 169823, Note_on_c, 4, 39, 37
//5, 169919, Note_off_c, 4, 39, 0
//5, 169919, Note_on_c, 4, 39, 37
//5, 170015, Note_off_c, 4, 39, 0
//5, 170015, Note_on_c, 4, 39, 37
//5, 170111, Note_off_c, 4, 39, 0
//5, 170111, Note_on_c, 4, 38, 37
//5, 170207, Note_off_c, 4, 38, 0
//5, 170207, Note_on_c, 4, 38, 37
//5, 170303, Note_off_c, 4, 38, 0
//5, 170303, Note_on_c, 4, 38, 37
//5, 170399, Note_off_c, 4, 38, 0
//5, 170399, Note_on_c, 4, 37, 37
//5, 170495, Note_off_c, 4, 37, 0
//5, 170495, Note_on_c, 4, 37, 37
//5, 170591, Note_off_c, 4, 37, 0
//5, 170591, Note_on_c, 4, 37, 37
//5, 170687, Note_off_c, 4, 37, 0
//5, 170687, Note_on_c, 4, 36, 37
//5, 170783, Note_off_c, 4, 36, 0
//5, 170783, Note_on_c, 4, 36, 37
//5, 170879, Note_off_c, 4, 36, 0
//5, 170879, Note_on_c, 4, 35, 37
//5, 170975, Note_off_c, 4, 35, 0
//5, 170975, Note_on_c, 4, 35, 37
//5, 171071, Note_off_c, 4, 35, 0
//5, 171071, Note_on_c, 4, 34, 37
//5, 171167, Note_off_c, 4, 34, 0
//5, 171167, Note_on_c, 4, 34, 37
//5, 171263, Note_off_c, 4, 34, 0
//5, 171263, Note_on_c, 4, 39, 37
//5, 171359, Note_off_c, 4, 39, 0
//5, 171359, Note_on_c, 4, 39, 37
//5, 171455, Note_off_c, 4, 39, 0
//5, 171455, Note_on_c, 4, 39, 37
//5, 171551, Note_off_c, 4, 39, 0
//5, 171551, Note_on_c, 4, 39, 37
//5, 171647, Note_off_c, 4, 39, 0
//5, 171647, Note_on_c, 4, 38, 37
//5, 171743, Note_off_c, 4, 38, 0
//5, 171743, Note_on_c, 4, 38, 37
//5, 171839, Note_off_c, 4, 38, 0
//5, 171839, Note_on_c, 4, 38, 37
//5, 171935, Note_off_c, 4, 38, 0
//5, 171935, Note_on_c, 4, 37, 37
//5, 172031, Note_off_c, 4, 37, 0
//5, 172031, Note_on_c, 4, 37, 37
//5, 172127, Note_off_c, 4, 37, 0
//5, 172127, Note_on_c, 4, 37, 37
//5, 172223, Note_off_c, 4, 37, 0
//5, 172223, Note_on_c, 4, 36, 37
//5, 172319, Note_off_c, 4, 36, 0
//5, 172319, Note_on_c, 4, 36, 37
//5, 172415, Note_off_c, 4, 36, 0
//5, 172415, Note_on_c, 4, 35, 37
//5, 172607, Note_off_c, 4, 35, 0
//5, 172607, Note_on_c, 4, 34, 37
//5, 172799, Note_off_c, 4, 34, 0
//5, 172799, Note_on_c, 4, 29, 37
//5, 173183, Note_off_c, 4, 29, 0
//5, 173183, Note_on_c, 4, 28, 37
//5, 173567, Note_off_c, 4, 28, 0
//5, 173567, Note_on_c, 4, 29, 37
//5, 173951, Note_off_c, 4, 29, 0
//5, 173951, Note_on_c, 4, 30, 37
//5, 174335, Note_off_c, 4, 30, 0
//5, 174335, Note_on_c, 4, 32, 37
//5, 174719, Note_off_c, 4, 32, 0
//5, 174719, Note_on_c, 4, 31, 37
//5, 175103, Note_off_c, 4, 31, 0
//5, 175103, Note_on_c, 4, 32, 37
//5, 175487, Note_off_c, 4, 32, 0
//5, 175487, Note_on_c, 4, 33, 37
//5, 175679, Note_off_c, 4, 33, 0
//5, 175679, Note_on_c, 4, 27, 37
//5, 175871, Note_off_c, 4, 27, 0
//5, 175871, Note_on_c, 4, 35, 37
//5, 176255, Note_off_c, 4, 35, 0
//5, 176255, Note_on_c, 4, 35, 37
//5, 176639, Note_off_c, 4, 35, 0
//5, 176639, Note_on_c, 4, 35, 37
//5, 176831, Note_off_c, 4, 35, 0
//5, 176831, Note_on_c, 4, 27, 37
//5, 177023, Note_off_c, 4, 27, 0
//5, 177023, Note_on_c, 4, 30, 37
//5, 177215, Note_off_c, 4, 30, 0
//5, 177215, Note_on_c, 4, 27, 37
//5, 177407, Note_off_c, 4, 27, 0
//5, 177407, Note_on_c, 4, 37, 37
//5, 177791, Note_off_c, 4, 37, 0
//5, 177791, Note_on_c, 4, 37, 37
//5, 178175, Note_off_c, 4, 37, 0
//5, 178175, Note_on_c, 4, 37, 37
//5, 178559, Note_off_c, 4, 37, 0
//5, 178559, Note_on_c, 4, 37, 37
//5, 178751, Note_off_c, 4, 37, 0
//5, 178751, Note_on_c, 4, 32, 37
//5, 178847, Note_off_c, 4, 32, 0
//5, 178847, Note_on_c, 4, 38, 37
//5, 178943, Note_off_c, 4, 38, 0
//5, 178943, Note_on_c, 4, 39, 37
//5, 179135, Note_off_c, 4, 39, 0
//5, 179135, Note_on_c, 4, 39, 37
//5, 179327, Note_off_c, 4, 39, 0
//5, 179327, Note_on_c, 4, 37, 37
//5, 179519, Note_off_c, 4, 37, 0
//5, 179519, Note_on_c, 4, 33, 37
//5, 179615, Note_off_c, 4, 33, 0
//5, 179615, Note_on_c, 4, 33, 37
//5, 179807, Note_off_c, 4, 33, 0
//5, 179807, Note_on_c, 4, 32, 37
//5, 179903, Note_off_c, 4, 32, 0
//5, 179903, Note_on_c, 4, 32, 37
//5, 180095, Note_off_c, 4, 32, 0
//5, 180095, Note_on_c, 4, 30, 37
//5, 180287, Note_off_c, 4, 30, 0
//5, 180287, Note_on_c, 4, 27, 37
//5, 180479, Note_off_c, 4, 27, 0
//5, 180479, Note_on_c, 4, 39, 37
//5, 180671, Note_off_c, 4, 39, 0
//5, 180671, Note_on_c, 4, 39, 37
//5, 180863, Note_off_c, 4, 39, 0
//5, 180863, Note_on_c, 4, 37, 37
//5, 181055, Note_off_c, 4, 37, 0
//5, 181055, Note_on_c, 4, 33, 37
//5, 181151, Note_off_c, 4, 33, 0
//5, 181151, Note_on_c, 4, 33, 37
//5, 181343, Note_off_c, 4, 33, 0
//5, 181343, Note_on_c, 4, 32, 37
//5, 181439, Note_off_c, 4, 32, 0
//5, 181439, Note_on_c, 4, 32, 37
//5, 181631, Note_off_c, 4, 32, 0
//5, 181631, Note_on_c, 4, 30, 37
//5, 181823, Note_off_c, 4, 30, 0
//5, 181823, Note_on_c, 4, 27, 37
//5, 182015, Note_off_c, 4, 27, 0
//5, 182015, Note_on_c, 4, 35, 37
//5, 182399, Note_off_c, 4, 35, 0
//5, 182399, Note_on_c, 4, 35, 37
//5, 182783, Note_off_c, 4, 35, 0
//5, 182783, Note_on_c, 4, 35, 37
//5, 182975, Note_off_c, 4, 35, 0
//5, 182975, Note_on_c, 4, 27, 37
//5, 183167, Note_off_c, 4, 27, 0
//5, 183167, Note_on_c, 4, 30, 37
//5, 183359, Note_off_c, 4, 30, 0
//5, 183359, Note_on_c, 4, 27, 37
//5, 183551, Note_off_c, 4, 27, 0
//5, 183551, Note_on_c, 4, 37, 37
//5, 183935, Note_off_c, 4, 37, 0
//5, 183935, Note_on_c, 4, 37, 37
//5, 184319, Note_off_c, 4, 37, 0
//5, 184319, Note_on_c, 4, 37, 37
//5, 184703, Note_off_c, 4, 37, 0
//5, 184703, Note_on_c, 4, 37, 37
//5, 184895, Note_off_c, 4, 37, 0
//5, 184895, Note_on_c, 4, 32, 37
//5, 184991, Note_off_c, 4, 32, 0
//5, 184991, Note_on_c, 4, 38, 37
//5, 185087, Note_off_c, 4, 38, 0
//5, 185087, Note_on_c, 4, 39, 37
//5, 185279, Note_off_c, 4, 39, 0
//5, 185279, Note_on_c, 4, 39, 37
//5, 185471, Note_off_c, 4, 39, 0
//5, 185471, Note_on_c, 4, 37, 37
//5, 185663, Note_off_c, 4, 37, 0
//5, 185663, Note_on_c, 4, 33, 37
//5, 185759, Note_off_c, 4, 33, 0
//5, 185759, Note_on_c, 4, 33, 37
//5, 185951, Note_off_c, 4, 33, 0
//5, 185951, Note_on_c, 4, 32, 37
//5, 186047, Note_off_c, 4, 32, 0
//5, 186047, Note_on_c, 4, 32, 37
//5, 186239, Note_off_c, 4, 32, 0
//5, 186239, Note_on_c, 4, 30, 37
//5, 186431, Note_off_c, 4, 30, 0
//5, 186431, Note_on_c, 4, 27, 37
//5, 186623, Note_off_c, 4, 27, 0
//5, 186623, Note_on_c, 4, 39, 37
//5, 186815, Note_off_c, 4, 39, 0
//5, 186815, Note_on_c, 4, 39, 37
//5, 187007, Note_off_c, 4, 39, 0
//5, 187007, Note_on_c, 4, 37, 37
//5, 187199, Note_off_c, 4, 37, 0
//5, 187199, Note_on_c, 4, 33, 37
//5, 187295, Note_off_c, 4, 33, 0
//5, 187295, Note_on_c, 4, 33, 37
//5, 187487, Note_off_c, 4, 33, 0
//5, 187487, Note_on_c, 4, 32, 37
//5, 187583, Note_off_c, 4, 32, 0
//5, 187583, Note_on_c, 4, 32, 37
//5, 187775, Note_off_c, 4, 32, 0
//5, 187775, Note_on_c, 4, 30, 37
//5, 187967, Note_off_c, 4, 30, 0
//5, 187967, Note_on_c, 4, 27, 37
//5, 188159, Note_off_c, 4, 27, 0
//5, 188159, Note_on_c, 4, 35, 37
//5, 188543, Note_off_c, 4, 35, 0
//5, 188543, Note_on_c, 4, 35, 37
//5, 188927, Note_off_c, 4, 35, 0
//5, 188927, Note_on_c, 4, 35, 37
//5, 189119, Note_off_c, 4, 35, 0
//5, 189119, Note_on_c, 4, 27, 37
//5, 189311, Note_off_c, 4, 27, 0
//5, 189311, Note_on_c, 4, 30, 37
//5, 189503, Note_off_c, 4, 30, 0
//5, 189503, Note_on_c, 4, 27, 37
//5, 189695, Note_off_c, 4, 27, 0
//5, 189695, Note_on_c, 4, 37, 37
//5, 190079, Note_off_c, 4, 37, 0
//5, 190079, Note_on_c, 4, 37, 37
//5, 190463, Note_off_c, 4, 37, 0
//5, 190463, Note_on_c, 4, 37, 37
//5, 190847, Note_off_c, 4, 37, 0
//5, 190847, Note_on_c, 4, 37, 37
//5, 191039, Note_off_c, 4, 37, 0
//5, 191039, Note_on_c, 4, 32, 37
//5, 191135, Note_off_c, 4, 32, 0
//5, 191135, Note_on_c, 4, 38, 37
//5, 191231, Note_off_c, 4, 38, 0
//5, 191231, Note_on_c, 4, 39, 37
//5, 191423, Note_off_c, 4, 39, 0
//5, 191423, Note_on_c, 4, 39, 37
//5, 191615, Note_off_c, 4, 39, 0
//5, 191615, Note_on_c, 4, 37, 37
//5, 191807, Note_off_c, 4, 37, 0
//5, 191807, Note_on_c, 4, 33, 37
//5, 191903, Note_off_c, 4, 33, 0
//5, 191903, Note_on_c, 4, 33, 37
//5, 192095, Note_off_c, 4, 33, 0
//5, 192095, Note_on_c, 4, 32, 37
//5, 192191, Note_off_c, 4, 32, 0
//5, 192191, Note_on_c, 4, 32, 37
//5, 192383, Note_off_c, 4, 32, 0
//5, 192383, Note_on_c, 4, 30, 37
//5, 192575, Note_off_c, 4, 30, 0
//5, 192575, Note_on_c, 4, 27, 37
//5, 192767, Note_off_c, 4, 27, 0
//5, 192767, Note_on_c, 4, 39, 37
//5, 192959, Note_off_c, 4, 39, 0
//5, 192959, Note_on_c, 4, 39, 37
//5, 193151, Note_off_c, 4, 39, 0
//5, 193151, Note_on_c, 4, 37, 37
//5, 193343, Note_off_c, 4, 37, 0
//5, 193343, Note_on_c, 4, 33, 37
//5, 193439, Note_off_c, 4, 33, 0
//5, 193439, Note_on_c, 4, 33, 37
//5, 193631, Note_off_c, 4, 33, 0
//5, 193631, Note_on_c, 4, 32, 37
//5, 193727, Note_off_c, 4, 32, 0
//5, 193727, Note_on_c, 4, 32, 37
//5, 193919, Note_off_c, 4, 32, 0
//5, 193919, Note_on_c, 4, 30, 37
//5, 194111, Note_off_c, 4, 30, 0
//5, 194111, Note_on_c, 4, 27, 37
//5, 194303, Note_off_c, 4, 27, 0
//5, 194303, Note_on_c, 4, 35, 37
//5, 194687, Note_off_c, 4, 35, 0
//5, 194687, Note_on_c, 4, 35, 37
//5, 195071, Note_off_c, 4, 35, 0
//5, 195071, Note_on_c, 4, 35, 37
//5, 195263, Note_off_c, 4, 35, 0
//5, 195263, Note_on_c, 4, 27, 37
//5, 195455, Note_off_c, 4, 27, 0
//5, 195455, Note_on_c, 4, 30, 37
//5, 195647, Note_off_c, 4, 30, 0
//5, 195647, Note_on_c, 4, 27, 37
//5, 195839, Note_off_c, 4, 27, 0
//5, 195839, Note_on_c, 4, 37, 37
//5, 196223, Note_off_c, 4, 37, 0
//5, 196223, Note_on_c, 4, 37, 37
//5, 196607, Note_off_c, 4, 37, 0
//5, 196607, Note_on_c, 4, 37, 37
//5, 196991, Note_off_c, 4, 37, 0
//5, 196991, Note_on_c, 4, 37, 37
//5, 197183, Note_off_c, 4, 37, 0
//5, 197183, Note_on_c, 4, 32, 37
//5, 197279, Note_off_c, 4, 32, 0
//5, 197279, Note_on_c, 4, 38, 37
//5, 197375, Note_off_c, 4, 38, 0
//5, 197375, Note_on_c, 4, 39, 37
//5, 197663, Note_off_c, 4, 39, 0
//5, 197663, Note_on_c, 4, 37, 37
//5, 197951, Note_off_c, 4, 37, 0
//5, 197951, Note_on_c, 4, 33, 37
//5, 198239, Note_off_c, 4, 33, 0
//5, 198239, Note_on_c, 4, 32, 37
//5, 198527, Note_off_c, 4, 32, 0
//5, 198527, Note_on_c, 4, 30, 37
//5, 198719, Note_off_c, 4, 30, 0
//5, 198719, Note_on_c, 4, 27, 37
//5, 198911, Note_off_c, 4, 27, 0
//5, 198911, Note_on_c, 4, 32, 37
//5, 199199, Note_off_c, 4, 32, 0
//5, 199199, Note_on_c, 4, 30, 37
//5, 199487, Note_off_c, 4, 30, 0
//5, 199487, Note_on_c, 4, 27, 37
//5, 199679, Note_off_c, 4, 27, 0
//5, 200063, Note_on_c, 4, 27, 37
//5, 203519, Note_off_c, 4, 27, 0
//5, 203519, End_track

//4, 0, Title_t, "Backup Vocals"
//4, 0, Program_c, 3, 73
//4, 39168, Note_on_c, 3, 71, 37
//4, 39552, Note_off_c, 3, 71, 0
//4, 39552, Note_on_c, 3, 70, 37
//4, 39936, Note_off_c, 3, 70, 0
//4, 39936, Note_on_c, 3, 68, 37
//4, 40320, Note_off_c, 3, 68, 0
//4, 40320, Note_on_c, 3, 66, 37
//4, 40704, Note_off_c, 3, 66, 0
//4, 40704, Note_on_c, 3, 73, 37
//4, 41088, Note_off_c, 3, 73, 0
//4, 41088, Note_on_c, 3, 72, 37
//4, 41472, Note_off_c, 3, 72, 0
//4, 41472, Note_on_c, 3, 70, 37
//4, 41856, Note_off_c, 3, 70, 0
//4, 41856, Note_on_c, 3, 68, 37
//4, 42240, Note_off_c, 3, 68, 0
//4, 60672, Note_on_c, 3, 71, 37
//4, 61056, Note_off_c, 3, 71, 0
//4, 61056, Note_on_c, 3, 70, 37
//4, 61440, Note_off_c, 3, 70, 0
//4, 61440, Note_on_c, 3, 68, 37
//4, 61824, Note_off_c, 3, 68, 0
//4, 61824, Note_on_c, 3, 66, 37
//4, 62208, Note_off_c, 3, 66, 0
//4, 62208, Note_on_c, 3, 73, 37
//4, 62592, Note_off_c, 3, 73, 0
//4, 62592, Note_on_c, 3, 72, 37
//4, 62976, Note_off_c, 3, 72, 0
//4, 62976, Note_on_c, 3, 70, 37
//4, 63360, Note_off_c, 3, 70, 0
//4, 63360, Note_on_c, 3, 68, 37
//4, 63744, Note_off_c, 3, 68, 0
//4, 94464, Note_on_c, 3, 71, 37
//4, 94848, Note_off_c, 3, 71, 0
//4, 94848, Note_on_c, 3, 70, 37
//4, 95232, Note_off_c, 3, 70, 0
//4, 95232, Note_on_c, 3, 68, 37
//4, 95616, Note_off_c, 3, 68, 0
//4, 95616, Note_on_c, 3, 66, 37
//4, 96000, Note_off_c, 3, 66, 0
//4, 96000, Note_on_c, 3, 73, 37
//4, 96384, Note_off_c, 3, 73, 0
//4, 96384, Note_on_c, 3, 72, 37
//4, 96768, Note_off_c, 3, 72, 0
//4, 96768, Note_on_c, 3, 70, 37
//4, 97152, Note_off_c, 3, 70, 0
//4, 97152, Note_on_c, 3, 68, 37
//4, 97536, Note_off_c, 3, 68, 0
//4, 175872, Note_on_c, 3, 71, 37
//4, 176256, Note_off_c, 3, 71, 0
//4, 176256, Note_on_c, 3, 70, 37
//4, 176640, Note_off_c, 3, 70, 0
//4, 176640, Note_on_c, 3, 68, 37
//4, 177024, Note_off_c, 3, 68, 0
//4, 177024, Note_on_c, 3, 66, 37
//4, 177408, Note_off_c, 3, 66, 0
//4, 177408, Note_on_c, 3, 73, 37
//4, 177792, Note_off_c, 3, 73, 0
//4, 177792, Note_on_c, 3, 72, 37
//4, 178176, Note_off_c, 3, 72, 0
//4, 178176, Note_on_c, 3, 70, 37
//4, 178560, Note_off_c, 3, 70, 0
//4, 178560, Note_on_c, 3, 68, 37
//4, 178944, Note_off_c, 3, 68, 0
//4, 182016, Note_on_c, 3, 71, 37
//4, 182400, Note_off_c, 3, 71, 0
//4, 182400, Note_on_c, 3, 70, 37
//4, 182784, Note_off_c, 3, 70, 0
//4, 182784, Note_on_c, 3, 68, 37
//4, 183168, Note_off_c, 3, 68, 0
//4, 183168, Note_on_c, 3, 66, 37
//4, 183552, Note_off_c, 3, 66, 0
//4, 183552, Note_on_c, 3, 73, 37
//4, 183936, Note_off_c, 3, 73, 0
//4, 183936, Note_on_c, 3, 72, 37
//4, 184320, Note_off_c, 3, 72, 0
//4, 184320, Note_on_c, 3, 70, 37
//4, 184704, Note_off_c, 3, 70, 0
//4, 184704, Note_on_c, 3, 68, 37
//4, 185088, Note_off_c, 3, 68, 0
//4, 188160, Note_on_c, 3, 71, 37
//4, 188544, Note_off_c, 3, 71, 0
//4, 188544, Note_on_c, 3, 70, 37
//4, 188928, Note_off_c, 3, 70, 0
//4, 188928, Note_on_c, 3, 68, 37
//4, 189312, Note_off_c, 3, 68, 0
//4, 189312, Note_on_c, 3, 66, 37
//4, 189696, Note_off_c, 3, 66, 0
//4, 189696, Note_on_c, 3, 73, 37
//4, 190080, Note_off_c, 3, 73, 0
//4, 190080, Note_on_c, 3, 72, 37
//4, 190464, Note_off_c, 3, 72, 0
//4, 190464, Note_on_c, 3, 70, 37
//4, 190848, Note_off_c, 3, 70, 0
//4, 190848, Note_on_c, 3, 68, 37
//4, 191232, Note_off_c, 3, 68, 0
//4, 194304, Note_on_c, 3, 71, 37
//4, 194688, Note_off_c, 3, 71, 0
//4, 194688, Note_on_c, 3, 70, 37
//4, 195072, Note_off_c, 3, 70, 0
//4, 195072, Note_on_c, 3, 68, 37
//4, 195456, Note_off_c, 3, 68, 0
//4, 195456, Note_on_c, 3, 66, 37
//4, 195840, Note_off_c, 3, 66, 0
//4, 195840, Note_on_c, 3, 73, 37
//4, 196224, Note_off_c, 3, 73, 0
//4, 196224, Note_on_c, 3, 72, 37
//4, 196608, Note_off_c, 3, 72, 0
//4, 196608, Note_on_c, 3, 70, 37
//4, 196992, Note_off_c, 3, 70, 0
//4, 196992, Note_on_c, 3, 68, 37
//4, 197376, Note_off_c, 3, 68, 0
//4, 197376, End_track
//
