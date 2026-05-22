use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::{U1, U2};

use crate::instrumentation::{BassArticulation, Thump};

use super::counter::DecrementCounter;

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

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct OstinatoLoopsRemaining;
impl LoopCondition<BassistState> for OstinatoLoopsRemaining {
    type Arg = ();

    fn should_continue(state: &BassistState) -> bool {
        state.ostinato_loops_remaining > 0
    }
}

pub struct RiffLoopsRemaining;
impl LoopCondition<BassistState> for RiffLoopsRemaining {
    type Arg = ();

    fn should_continue(state: &BassistState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct IntroTailNeeded;
impl<In> Condition<(BassistState, In)> for IntroTailNeeded {
    fn choose(input: &(BassistState, In)) -> bool {
        input.0.riff_loops_remaining == 0
    }
}

type OstinatoLoopCounter = Lens<BassistState, U1>;
type RiffLoopCounter = Lens<BassistState, U2>;

pub type AdvanceOstinatoLoop = DecrementCounter<OstinatoLoopCounter>;
pub type AdvanceRiffLoop = DecrementCounter<RiffLoopCounter>;

#[derive(Flow)]
pub struct BassIntro(
    Transparent<IntroSectionMeta, BassPrelude>,
    Transparent<IntroSectionMeta, While<OstinatoLoopsRemaining, BassOstinatoLoopBody>>,
    Transparent<IntroSectionMeta, BassTransition>,
    Transparent<IntroSectionMeta, While<RiffLoopsRemaining, BassRiffLoopBody>>,
    Transparent<IntroSectionMeta, Conditional<IntroTailNeeded, BassTail, BassRelease>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPrelude(
    Step<Thump<46, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
pub struct BassOstinatoLoopBody(
    Transparent<IntroSectionMeta, BassOstinatoCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceOstinatoLoop>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassOstinatoCycle(
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
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<34, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassTransition(
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 192, 192>>,
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
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
pub struct BassRiffLoopBody(
    Transparent<IntroSectionMeta, BassRiffCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceRiffLoop>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassRiffCycle(
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassTail(
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassRelease(Step<Thump<27, 192, 192>>);
