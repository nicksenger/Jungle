#![recursion_limit = "1024"]

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusState {
    counter: u32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileState {
    #[jungle(focus)]
    focus: FocusState,
    counter: u32,
}

pub struct CompileEffectId<const EFFECT_ID: usize>;
impl<const EFFECT_ID: usize> IdValue for CompileEffectId<EFFECT_ID> {
    type Value = num::U0;
}

pub struct CompileNoEffect<const EFFECT_ID: usize>;
impl<const EFFECT_ID: usize> EffectSchema for CompileNoEffect<EFFECT_ID> {
    type Id = CompileEffectId<EFFECT_ID>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J, const EFFECT_ID: usize> Effect<J> for CompileNoEffect<EFFECT_ID> {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        black_box(EFFECT_ID);
        std::future::ready(Ok(()))
    }
}

pub struct CompileTouch<const EFFECT_ID: usize>;
impl<const EFFECT_ID: usize> EffectSchema for CompileTouch<EFFECT_ID> {
    type Id = CompileEffectId<EFFECT_ID>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J, const EFFECT_ID: usize> Effect<J> for CompileTouch<EFFECT_ID> {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        black_box(EFFECT_ID);
        std::future::ready(Ok(()))
    }
}

pub struct TickSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for TickSpec<EFFECT_ID> {
    type Effect = CompileTouch<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("touch effect should succeed"))?;
        };
        Ok(__absorb_out_1)
    }
}

pub struct FocusTickSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for FocusTickSpec<EFFECT_ID> {
    type Effect = CompileNoEffect<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &FocusState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(_state: &mut FocusState, output: EffectCompletion<Self::Effect>) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("focused no-effect should succeed"))?;
        };
        Ok(__absorb_out_2)
    }
}

pub struct FocusJoinMergeSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for FocusJoinMergeSpec<EFFECT_ID> {
    type Effect = CompileTouch<EFFECT_ID>;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &FocusState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(_state: &mut FocusState, output: EffectCompletion<Self::Effect>) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("focused join merge effect should succeed"))?;
        };
        Ok(__absorb_out_3)
    }
}

pub struct JoinTickSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for JoinTickSpec<EFFECT_ID> {
    type Effect = CompileTouch<EFFECT_ID>;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("join branch effect should succeed"))?;
        };
        Ok(__absorb_out_4)
    }
}

pub struct JoinFlattenSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for JoinFlattenSpec<EFFECT_ID> {
    type Effect = CompileNoEffect<EFFECT_ID>;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("join flatten effect should succeed"))?;
        };
        Ok(__absorb_out_5)
    }
}

pub struct CompileChooseLeft<const SEGMENT_ID: usize>;
impl<const SEGMENT_ID: usize> Predicate<(CompileState, ())> for CompileChooseLeft<SEGMENT_ID> {
    fn eval(input: &(CompileState, ())) -> bool {
        black_box(input);
        black_box(SEGMENT_ID);
        SEGMENT_ID.is_multiple_of(2)
    }
}

pub struct CompileLoopOnce<const SEGMENT_ID: usize>;
impl<const SEGMENT_ID: usize> Predicate<(&CompileState, &())> for CompileLoopOnce<SEGMENT_ID> {
    fn eval((state, _): &(&CompileState, &())) -> bool {
        black_box(SEGMENT_ID);
        state.counter == 0
    }
}

pub struct IncrementCounterSpec<const EFFECT_ID: usize>;
#[jungle::action]
impl<const EFFECT_ID: usize> Action for IncrementCounterSpec<EFFECT_ID> {
    type Effect = CompileTouch<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {
        black_box(EFFECT_ID);
    }

    fn absorb(
        state: &mut CompileState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_6 = {
        black_box(EFFECT_ID);
        output.map_err(|_err| Failure::from("counter increment effect should succeed"))?;
        state.counter = state.counter.saturating_add(1);
        };
        Ok(__absorb_out_6)
    }
}

impl From<CompileState> for () {
    fn from(_value: CompileState) -> Self {}
}

pub struct CompileMetaA;
impl NodeMetadata for CompileMetaA {
    const METADATA: &'static str = "compile-times-a";
}

pub struct CompileMetaB;
impl NodeMetadata for CompileMetaB {
    const METADATA: &'static str = "compile-times-b";
}

pub struct CompileMetaC;
impl NodeMetadata for CompileMetaC {
    const METADATA: &'static str = "compile-times-c";
}

pub struct CompileMetaD;
impl NodeMetadata for CompileMetaD {
    const METADATA: &'static str = "compile-times-d";
}

#[derive(Flow)]
#[jungle(focus = FocusState)]
pub struct FocusedSegmentA(
    jungle_zoo::ClonedJoinUnit<Step<FocusTickSpec<0>>, Step<FocusTickSpec<1>>>,
    Step<FocusJoinMergeSpec<2>>,
    Step<FocusTickSpec<3>>,
    Step<FocusTickSpec<4>>,
    Step<FocusTickSpec<5>>,
    Step<FocusTickSpec<6>>,
    Step<FocusTickSpec<7>>,
    Step<FocusTickSpec<8>>,
    Step<FocusTickSpec<9>>,
    Step<FocusTickSpec<10>>,
    Step<FocusTickSpec<11>>,
);

#[derive(Flow)]
#[jungle(focus = FocusState)]
pub struct FocusedSegmentB(
    Step<FocusTickSpec<30>>,
    Step<FocusTickSpec<31>>,
    Transparent<CompileMetaB, jungle_zoo::ClonedJoinUnit<Step<FocusTickSpec<32>>, Step<FocusTickSpec<33>>>>,
    Step<FocusJoinMergeSpec<34>>,
    Step<FocusTickSpec<35>>,
    Step<FocusTickSpec<36>>,
    Step<FocusTickSpec<37>>,
    Step<FocusTickSpec<38>>,
    Step<FocusTickSpec<39>>,
    Step<FocusTickSpec<40>>,
    Step<FocusTickSpec<41>>,
);

#[derive(Flow)]
pub struct LoopBodyA(
    Transparent<
        CompileMetaA,
        Conditional<CompileChooseLeft<1>, Step<TickSpec<10>>, Step<TickSpec<11>>>,
    >,
    jungle_zoo::ClonedJoinUnit<Step<JoinTickSpec<12>>, Step<JoinTickSpec<13>>>,
    Step<JoinFlattenSpec<14>>,
    Step<IncrementCounterSpec<15>>,
);

#[derive(Flow)]
pub struct LoopBodyB(
    Transparent<
        CompileMetaC,
        Conditional<CompileChooseLeft<2>, Step<TickSpec<53>>, Step<TickSpec<54>>>,
    >,
    Transparent<
        CompileMetaD,
        jungle_zoo::ClonedJoinUnit<Step<JoinTickSpec<50>>, Step<JoinTickSpec<51>>>,
    >,
    Step<JoinFlattenSpec<52>>,
    Step<IncrementCounterSpec<55>>,
);

macro_rules! define_journey_24_a {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            Transparent<CompileMetaA, FocusedSegmentA>,
            While<CompileLoopOnce<1>, LoopBodyA>,
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

macro_rules! define_journey_24_b {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            While<CompileLoopOnce<3>, LoopBodyB>,
            Transparent<CompileMetaB, FocusedSegmentB>,
            Step<TickSpec<56>>,
            Step<TickSpec<57>>,
            Step<TickSpec<58>>,
            Step<TickSpec<59>>,
            Step<TickSpec<60>>,
            Step<TickSpec<61>>,
            Step<TickSpec<62>>,
            Step<TickSpec<63>>,
        );
    };
}

macro_rules! define_journey_24_c {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            Transparent<CompileMetaC, FocusedSegmentA>,
            Step<TickSpec<80>>,
            While<CompileLoopOnce<4>, LoopBodyA>,
            Step<TickSpec<81>>,
            Step<TickSpec<82>>,
            Step<TickSpec<83>>,
            Step<TickSpec<84>>,
            Step<TickSpec<85>>,
            Step<TickSpec<86>>,
            Step<TickSpec<87>>,
        );
    };
}

macro_rules! define_journey_24_d {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            Transparent<CompileMetaD, FocusedSegmentB>,
            Step<TickSpec<90>>,
            While<CompileLoopOnce<5>, LoopBodyB>,
            Step<TickSpec<91>>,
            Step<TickSpec<92>>,
            Step<TickSpec<93>>,
            Step<TickSpec<94>>,
            Step<TickSpec<95>>,
            Step<TickSpec<96>>,
            Step<TickSpec<97>>,
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

macro_rules! define_journey_family {
    ($j24:ident, $j48:ident, $j96:ident, $j192:ident, $j384:ident, $j768:ident, $build24:ident) => {
        $build24!($j24);
        define_journey_double!($j48, $j24);
        define_journey_double!($j96, $j48);
        #[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
        define_journey_double!($j192, $j96);
        #[cfg(any(feature = "large", feature = "xlarge"))]
        define_journey_double!($j384, $j192);
        #[cfg(feature = "xlarge")]
        define_journey_double!($j768, $j384);
    };
}

define_journey_family!(JourneyA24, JourneyA48, JourneyA96, JourneyA192, JourneyA384, JourneyA768, define_journey_24_a);
define_journey_family!(JourneyB24, JourneyB48, JourneyB96, JourneyB192, JourneyB384, JourneyB768, define_journey_24_b);
define_journey_family!(JourneyC24, JourneyC48, JourneyC96, JourneyC192, JourneyC384, JourneyC768, define_journey_24_c);
define_journey_family!(JourneyD24, JourneyD48, JourneyD96, JourneyD192, JourneyD384, JourneyD768, define_journey_24_d);
define_journey_family!(JourneyE24, JourneyE48, JourneyE96, JourneyE192, JourneyE384, JourneyE768, define_journey_24_a);
define_journey_family!(JourneyF24, JourneyF48, JourneyF96, JourneyF192, JourneyF384, JourneyF768, define_journey_24_b);
define_journey_family!(JourneyG24, JourneyG48, JourneyG96, JourneyG192, JourneyG384, JourneyG768, define_journey_24_c);
define_journey_family!(JourneyH24, JourneyH48, JourneyH96, JourneyH192, JourneyH384, JourneyH768, define_journey_24_d);

macro_rules! select_tier_journey {
    ($j96:ty, $j192:ty, $j384:ty, $j768:ty) => {
        #[cfg(all(
            feature = "small",
            not(feature = "medium"),
            not(feature = "large"),
            not(feature = "xlarge")
        ))]
        pub type TierJourney = $j96;

        #[cfg(all(feature = "medium", not(feature = "large"), not(feature = "xlarge")))]
        pub type TierJourney = $j192;

        #[cfg(all(feature = "large", not(feature = "xlarge")))]
        pub type TierJourney = $j384;

        #[cfg(feature = "xlarge")]
        pub type TierJourney = $j768;
    };
}

pub mod animal01_journey {
    use super::*;
    select_tier_journey!(JourneyA96, JourneyA192, JourneyA384, JourneyA768);
}
pub mod animal02_journey {
    use super::*;
    select_tier_journey!(JourneyB96, JourneyB192, JourneyB384, JourneyB768);
}
pub mod animal03_journey {
    use super::*;
    select_tier_journey!(JourneyC96, JourneyC192, JourneyC384, JourneyC768);
}
pub mod animal04_journey {
    use super::*;
    select_tier_journey!(JourneyD96, JourneyD192, JourneyD384, JourneyD768);
}
pub mod animal05_journey {
    use super::*;
    select_tier_journey!(JourneyE96, JourneyE192, JourneyE384, JourneyE768);
}
pub mod animal06_journey {
    use super::*;
    select_tier_journey!(JourneyF96, JourneyF192, JourneyF384, JourneyF768);
}
pub mod animal07_journey {
    use super::*;
    select_tier_journey!(JourneyG96, JourneyG192, JourneyG384, JourneyG768);
}
pub mod animal08_journey {
    use super::*;
    select_tier_journey!(JourneyH96, JourneyH192, JourneyH384, JourneyH768);
}

pub struct Animal01;
#[jungle::animal(id = 1001, generation = 0)]
impl Animal for Animal01 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal01_journey::TierJourney;
}

pub struct Animal02;
#[jungle::animal(id = 1002, generation = 0)]
impl Animal for Animal02 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal02_journey::TierJourney;
}

pub struct Animal03;
#[jungle::animal(id = 1003, generation = 0)]
impl Animal for Animal03 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal03_journey::TierJourney;
}

pub struct Animal04;
#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Animal04 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal04_journey::TierJourney;
}

pub struct Animal05;
#[jungle::animal(id = 1005, generation = 0)]
impl Animal for Animal05 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal05_journey::TierJourney;
}

pub struct Animal06;
#[jungle::animal(id = 1006, generation = 0)]
impl Animal for Animal06 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal06_journey::TierJourney;
}

pub struct Animal07;
#[jungle::animal(id = 1007, generation = 0)]
impl Animal for Animal07 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal07_journey::TierJourney;
}

pub struct Animal08;
#[jungle::animal(id = 1008, generation = 0)]
impl Animal for Animal08 {
    type State = CompileState;
    type Seed = CompileState;
    type Flow = animal08_journey::TierJourney;
}

#[cfg(all(
    feature = "small",
    not(feature = "medium"),
    not(feature = "large"),
    not(feature = "xlarge")
))]
#[derive(Animals)]
pub struct CompileAnimals(Animal01);

#[cfg(all(feature = "medium", not(feature = "large"), not(feature = "xlarge")))]
#[derive(Animals)]
pub struct CompileAnimals(Animal01, Animal02);

#[cfg(all(feature = "large", not(feature = "xlarge")))]
#[derive(Animals)]
pub struct CompileAnimals(Animal01, Animal02, Animal03, Animal04);

#[cfg(feature = "xlarge")]
#[derive(Animals)]
pub struct CompileAnimals(
    Animal01,
    Animal02,
    Animal03,
    Animal04,
    Animal05,
    Animal06,
    Animal07,
    Animal08,
);

pub struct CompileZoo;
impl Ecosystem for CompileZoo {
    const NAME: &'static str = "compile-times";
    type Animals = CompileAnimals;
}

fn force_worker_typecheck() {
    let client = jungle_sdk::MockClient::default();
    let worker = jungle_sdk::core::JungleWorker::new(CompileZoo, client);
    black_box(worker);
}

fn force_journey_ast_typecheck() {
    let ast01 = <animal01_journey::TierJourney as JourneyAstSource>::journey_ast();
    black_box(ast01);

    #[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
    {
        let ast02 = <animal02_journey::TierJourney as JourneyAstSource>::journey_ast();
        black_box(ast02);
    }

    #[cfg(any(feature = "large", feature = "xlarge"))]
    {
        let ast03 = <animal03_journey::TierJourney as JourneyAstSource>::journey_ast();
        let ast04 = <animal04_journey::TierJourney as JourneyAstSource>::journey_ast();
        black_box(ast03);
        black_box(ast04);
    }

    #[cfg(feature = "xlarge")]
    {
        let ast05 = <animal05_journey::TierJourney as JourneyAstSource>::journey_ast();
        let ast06 = <animal06_journey::TierJourney as JourneyAstSource>::journey_ast();
        let ast07 = <animal07_journey::TierJourney as JourneyAstSource>::journey_ast();
        let ast08 = <animal08_journey::TierJourney as JourneyAstSource>::journey_ast();
        black_box(ast05);
        black_box(ast06);
        black_box(ast07);
        black_box(ast08);
    }
}

fn main() {
    force_worker_typecheck();
    force_journey_ast_typecheck();
}
