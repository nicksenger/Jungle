use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::U1;

use crate::effect::DecrementCounterEffect;
use crate::instrumentation::{ElectricGuitarArticulation, Pick, Pluck};

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

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct LeadIntroRiffRemaining;
impl LoopCondition<LeadGuitaristState> for LeadIntroRiffRemaining {
    type Arg = ();

    fn should_continue(state: &LeadGuitaristState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct LeadIntroCadenceNeeded;
impl<In> Condition<(LeadGuitaristState, In)> for LeadIntroCadenceNeeded {
    fn choose(input: &(LeadGuitaristState, In)) -> bool {
        input.0.riff_loops_remaining == 0
    }
}

pub struct DecrementCounter<Focus>(core::marker::PhantomData<fn() -> Focus>);
#[jungle::act(aspect = Focus)]
impl<Focus> Act for DecrementCounter<Focus> {
    type Effect = DecrementCounterEffect;
    type Input = ();
    type Output = ();

    fn emit(_view: &u8, _input: Self::Input) -> Self::Input {}

    fn absorb(view: &mut u8, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("counter decrement should succeed");
        *view = view.saturating_sub(1);
    }
}

type RiffLoopCounter = Lens<LeadGuitaristState, U1>;
pub type AdvanceLeadIntroRiff = DecrementCounter<RiffLoopCounter>;

#[derive(Flow)]
pub struct LeadGuitarIntro(
    Transparent<IntroSectionMeta, LeadPrelude>,
    Transparent<IntroSectionMeta, While<LeadIntroRiffRemaining, LeadIntroRiffLoopBody>>,
    Transparent<
        IntroSectionMeta,
        Conditional<LeadIntroCadenceNeeded, LeadIntroCadence, LeadIntroTail>,
    >,
);

#[derive(Flow)]
pub struct LeadPrelude(
    Transparent<IntroSectionMeta, LeadOpeningPads>,
    Transparent<IntroSectionMeta, LeadAscentFigure>,
    Transparent<IntroSectionMeta, LeadPreRiffCadence>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadOpeningPads(
    Step<Pluck<46, 53, 192, 0>>,
    Step<Pick<61, 192, 0>>,
    Step<Pick<63, 192, 0>>,
    Step<Pick<63, 192, 0>>,
    Step<Pick<63, 96, 0>>,
    Step<Pick<61, 192, 0>>,
    Step<Pick<61, 192, 0>>,
    Step<Pick<56, 96, 0>>,
    Step<Pick<58, 192, 0>>,
    Step<Pick<58, 192, 0>>,
    Step<Pick<51, 192, 0>>,
    Step<Pick<53, 192, 0>>,
    Step<Pluck<58, 65, 192, 0>>,
    Step<Pluck<58, 65, 192, 0>>,
    Step<Pluck<44, 51, 192, 0>>,
    Step<Pick<56, 192, 0>>,
    Step<Pluck<39, 46, 192, 0>>,
    Step<Pluck<49, 56, 192, 0>>,
    Step<Pluck<44, 51, 192, 0>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadAscentFigure(
    Step<Pick<46, 192, 0>>,
    Step<Pick<48, 192, 0>>,
    Step<Pick<51, 192, 0>>,
    Step<Pick<53, 192, 0>>,
    Step<Pick<56, 192, 0>>,
    Step<Pick<58, 192, 0>>,
    Step<Pick<61, 192, 0>>,
    Step<Pick<63, 192, 0>>,
    Step<Pick<63, 192, 0>>,
    Step<Pick<68, 192, 0>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPreRiffCadence(
    Step<Pluck<63, 68, 192, 0>>,
    Step<Pick<61, 192, 0>>,
    Step<Pick<58, 192, 0>>,
);

#[derive(Flow)]
pub struct LeadIntroRiffLoopBody(
    Transparent<IntroSectionMeta, LeadIntroRiffCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceLeadIntroRiff>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroRiffCycle(
    Step<Pluck<44, 51, 192, 0>>,
    Step<Pluck<44, 51, 192, 0>>,
    Step<Pluck<42, 49, 192, 0>>,
    Step<Pluck<44, 51, 96, 0>>,
    Step<Pluck<44, 51, 192, 0>>,
    Step<Pluck<44, 51, 96, 0>>,
    Step<Pluck<42, 49, 192, 0>>,
    Step<Pluck<41, 48, 192, 0>>,
    Step<Pluck<39, 46, 192, 0>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroCadence(Step<Pluck<39, 46, 192, 0>>);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroTail(Step<Pick<58, 192, 0>>);
