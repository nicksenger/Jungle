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

macro_rules! define_journey {
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

define_journey!(Journey01);
define_journey!(Journey02);
define_journey!(Journey03);
define_journey!(Journey04);
define_journey!(Journey05);
define_journey!(Journey06);
define_journey!(Journey07);
define_journey!(Journey08);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey09);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey10);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey11);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey12);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey13);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey14);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey15);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey!(Journey16);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey17);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey18);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey19);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey20);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey21);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey22);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey23);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey!(Journey24);
#[cfg(feature = "xlarge")]
define_journey!(Journey25);
#[cfg(feature = "xlarge")]
define_journey!(Journey26);
#[cfg(feature = "xlarge")]
define_journey!(Journey27);
#[cfg(feature = "xlarge")]
define_journey!(Journey28);
#[cfg(feature = "xlarge")]
define_journey!(Journey29);
#[cfg(feature = "xlarge")]
define_journey!(Journey30);
#[cfg(feature = "xlarge")]
define_journey!(Journey31);
#[cfg(feature = "xlarge")]
define_journey!(Journey32);

pub struct Animal01;
#[jungle::animal(id = 1001, generation = 0)]
impl Animal for Animal01 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey01;
}

pub struct Animal02;
#[jungle::animal(id = 1002, generation = 0)]
impl Animal for Animal02 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey02;
}

pub struct Animal03;
#[jungle::animal(id = 1003, generation = 0)]
impl Animal for Animal03 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey03;
}

pub struct Animal04;
#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Animal04 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey04;
}

pub struct Animal05;
#[jungle::animal(id = 1005, generation = 0)]
impl Animal for Animal05 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey05;
}

pub struct Animal06;
#[jungle::animal(id = 1006, generation = 0)]
impl Animal for Animal06 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey06;
}

pub struct Animal07;
#[jungle::animal(id = 1007, generation = 0)]
impl Animal for Animal07 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey07;
}

pub struct Animal08;
#[jungle::animal(id = 1008, generation = 0)]
impl Animal for Animal08 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey08;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal09;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1009, generation = 0)]
impl Animal for Animal09 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey09;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal10;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1010, generation = 0)]
impl Animal for Animal10 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey10;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal11;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1011, generation = 0)]
impl Animal for Animal11 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey11;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal12;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1012, generation = 0)]
impl Animal for Animal12 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey12;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal13;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1013, generation = 0)]
impl Animal for Animal13 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey13;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal14;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1014, generation = 0)]
impl Animal for Animal14 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey14;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal15;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1015, generation = 0)]
impl Animal for Animal15 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey15;
}

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
pub struct Animal16;
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1016, generation = 0)]
impl Animal for Animal16 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey16;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal17;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1017, generation = 0)]
impl Animal for Animal17 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey17;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal18;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1018, generation = 0)]
impl Animal for Animal18 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey18;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal19;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1019, generation = 0)]
impl Animal for Animal19 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey19;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal20;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1020, generation = 0)]
impl Animal for Animal20 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey20;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal21;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1021, generation = 0)]
impl Animal for Animal21 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey21;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal22;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1022, generation = 0)]
impl Animal for Animal22 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey22;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal23;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1023, generation = 0)]
impl Animal for Animal23 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey23;
}

#[cfg(any(feature = "large", feature = "xlarge"))]
pub struct Animal24;
#[cfg(any(feature = "large", feature = "xlarge"))]
#[jungle::animal(id = 1024, generation = 0)]
impl Animal for Animal24 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey24;
}

#[cfg(feature = "xlarge")]
pub struct Animal25;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 993, generation = 0)]
impl Animal for Animal25 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey25;
}

#[cfg(feature = "xlarge")]
pub struct Animal26;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 994, generation = 0)]
impl Animal for Animal26 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey26;
}

#[cfg(feature = "xlarge")]
pub struct Animal27;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 995, generation = 0)]
impl Animal for Animal27 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey27;
}

#[cfg(feature = "xlarge")]
pub struct Animal28;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 996, generation = 0)]
impl Animal for Animal28 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey28;
}

#[cfg(feature = "xlarge")]
pub struct Animal29;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 997, generation = 0)]
impl Animal for Animal29 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey29;
}

#[cfg(feature = "xlarge")]
pub struct Animal30;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 998, generation = 0)]
impl Animal for Animal30 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey30;
}

#[cfg(feature = "xlarge")]
pub struct Animal31;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 999, generation = 0)]
impl Animal for Animal31 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey31;
}

#[cfg(feature = "xlarge")]
pub struct Animal32;
#[cfg(feature = "xlarge")]
#[jungle::animal(id = 1000, generation = 0)]
impl Animal for Animal32 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = Journey32;
}

#[derive(Animals)]
pub struct SmallAnimals(
    Animal01,
    Animal02,
    Animal03,
    Animal04,
    Animal05,
    Animal06,
    Animal07,
    Animal08,
);

#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
#[derive(Animals)]
pub struct MediumAnimals(
    SmallAnimals,
    Animal09,
    Animal10,
    Animal11,
    Animal12,
    Animal13,
    Animal14,
    Animal15,
    Animal16,
);

#[cfg(any(feature = "large", feature = "xlarge"))]
#[derive(Animals)]
pub struct LargeAnimals(
    MediumAnimals,
    Animal17,
    Animal18,
    Animal19,
    Animal20,
    Animal21,
    Animal22,
    Animal23,
    Animal24,
);

#[cfg(feature = "xlarge")]
#[derive(Animals)]
pub struct XLargeAnimals(
    LargeAnimals,
    Animal25,
    Animal26,
    Animal27,
    Animal28,
    Animal29,
    Animal30,
    Animal31,
    Animal32,
);

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
