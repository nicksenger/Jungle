#![recursion_limit = "1024"]

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileState {
    counter: u32,
}

pub struct CompileEffectId<const EFFECT_ID: usize>;
impl<const EFFECT_ID: usize> IdValue for CompileEffectId<EFFECT_ID> {
    type Value = num::U0;
}

pub struct CompileNoop<const EFFECT_ID: usize>;
impl<const EFFECT_ID: usize> EffectSchema for CompileNoop<EFFECT_ID> {
    type Id = CompileEffectId<EFFECT_ID>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J, const EFFECT_ID: usize> Effect<J> for CompileNoop<EFFECT_ID> {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        std::future::ready(Ok(()))
    }
}

pub struct TickSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for TickSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("noop effect should succeed");
    }
}

impl From<CompileState> for () {
    fn from(_value: CompileState) -> Self {}
}

macro_rules! define_journey_24 {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            Step<TickSpec<0>>,
            Step<TickSpec<1>>,
            Step<TickSpec<2>>,
            Step<TickSpec<3>>,
            Step<TickSpec<4>>,
            Step<TickSpec<5>>,
            Step<TickSpec<6>>,
            Step<TickSpec<7>>,
            Step<TickSpec<8>>,
            Step<TickSpec<9>>,
            Step<TickSpec<10>>,
            Step<TickSpec<11>>,
            Step<TickSpec<12>>,
            Step<TickSpec<13>>,
            Step<TickSpec<14>>,
            Step<TickSpec<15>>,
            Step<TickSpec<16>>,
            Step<TickSpec<17>>,
            Step<TickSpec<18>>,
            Step<TickSpec<19>>,
            Step<TickSpec<20>>,
            Step<TickSpec<21>>,
            Step<TickSpec<22>>,
            Step<TickSpec<23>>,
        );
    };
}

#[allow(unused_macros)]
macro_rules! define_journey_double {
    ($name:ident, $inner:ty) => {
        #[derive(Flow)]
        pub struct $name($inner, $inner);
    };
}

define_journey_24!(Journey24);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey_double!(Journey48, Journey24);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey_double!(Journey96, Journey48);
#[cfg(feature = "xlarge")]
define_journey_double!(Journey192, Journey96);

#[cfg(all(
    feature = "small",
    not(feature = "medium"),
    not(feature = "large"),
    not(feature = "xlarge")
))]
type TierJourney = Journey24;

#[cfg(all(feature = "medium", not(feature = "large"), not(feature = "xlarge")))]
type TierJourney = Journey48;

#[cfg(all(feature = "large", not(feature = "xlarge")))]
type TierJourney = Journey96;

#[cfg(feature = "xlarge")]
type TierJourney = Journey192;

pub struct Animal01;
#[jungle::animal(id = 1001, generation = 0)]
impl Animal for Animal01 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

#[derive(Animals)]
pub struct CompileAnimals(Animal01);

pub struct CompileZoo;
impl Ecosystem for CompileZoo {
    const NAME: &'static str = "compile-times";
    type Animals = CompileAnimals;
}

fn force_worker_typecheck() {
    let client = jungle_sdk::MockClient::default();
    let worker = jungle_sdk::core::JungleWorker::new(CompileZoo, client);
    std::hint::black_box(worker);
}

fn force_journey_ast_typecheck() {
    let ast = <TierJourney as JourneyAstSource>::journey_ast();
    std::hint::black_box(ast);
}

fn main() {
    force_worker_typecheck();
    force_journey_ast_typecheck();
}
