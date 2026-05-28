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

#[allow(unused_macros)]
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
macro_rules! define_journey_40 {
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
macro_rules! define_journey_56 {
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
macro_rules! define_journey_72 {
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

define_journey_24!(Journey01);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey_40!(Journey02);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey_56!(Journey03);
#[cfg(feature = "xlarge")]
define_journey_72!(Journey04);

pub struct Animal01;
#[jungle::animal(id = 1001, generation = 0)]
impl Animal for Animal01 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey01;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal02;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1002, generation = 0)]
impl Animal for Animal02 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey02;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal03;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1003, generation = 0)]
impl Animal for Animal03 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey03;
}

#[cfg(feature = "xlarge")]
pub struct Animal04;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Animal04 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey04;
}

#[derive(Animals)]
pub struct SmallAnimals(Animal01);

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[derive(Animals)]
pub struct MediumAnimals(Animal02);

#[cfg(any(feature = "large", feature = "xlarge"))]
#[derive(Animals)]
pub struct LargeAnimals(Animal03);

#[cfg(feature = "xlarge")]
#[derive(Animals)]
pub struct XLargeAnimals(Animal04);

#[cfg(all(
    feature = "small",
    not(feature = "medium"),
    not(feature = "large"),
    not(feature = "xlarge")
))]
type CompileAnimals = SmallAnimals;

#[cfg(all(feature = "medium", not(feature = "large"), not(feature = "xlarge")))]
type CompileAnimals = MediumAnimals;

#[cfg(all(feature = "large", not(feature = "xlarge")))]
type CompileAnimals = LargeAnimals;

#[cfg(feature = "xlarge")]
type CompileAnimals = XLargeAnimals;

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
