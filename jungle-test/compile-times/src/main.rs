#![recursion_limit = "1024"]

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileState {
    counter: u32,
}

pub struct TickSpec;
#[jungle::act]
impl Act for TickSpec {
    type Effect = Noop;
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
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
            Step<TickSpec>,
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

fn main() {
    force_worker_typecheck();
}
