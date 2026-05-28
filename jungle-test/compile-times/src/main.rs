#![recursion_limit = "1024"]

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

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

pub struct FocusTickSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for FocusTickSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &FocusState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut FocusState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("focused noop effect should succeed");
    }
}

pub struct FocusJoinMergeSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for FocusJoinMergeSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &FocusState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut FocusState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("focused join merge noop effect should succeed");
    }
}

pub struct JoinTickSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for JoinTickSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join branch noop effect should succeed");
    }
}

pub struct JoinFlattenSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for JoinFlattenSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut CompileState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join flatten noop effect should succeed");
    }
}

pub struct CompileChooseLeft<const SEGMENT_ID: usize>;
impl<const SEGMENT_ID: usize> Condition<(CompileState, ())> for CompileChooseLeft<SEGMENT_ID> {
    fn choose(input: &(CompileState, ())) -> bool {
        let _ = input;
        SEGMENT_ID.is_multiple_of(2)
    }
}

pub struct CompileLoopOnce<const SEGMENT_ID: usize>;
impl<const SEGMENT_ID: usize> LoopCondition<CompileState> for CompileLoopOnce<SEGMENT_ID> {
    type Arg = ();

    fn should_continue(state: &CompileState) -> bool {
        let _ = SEGMENT_ID;
        state.counter == 0
    }
}

pub struct IncrementCounterSpec<const EFFECT_ID: usize>;
#[jungle::act]
impl<const EFFECT_ID: usize> Act for IncrementCounterSpec<EFFECT_ID> {
    type Effect = CompileNoop<EFFECT_ID>;
    type Input = ();
    type Output = ();

    fn emit(_state: &CompileState, _input: Self::Input) -> () {}

    fn absorb(
        state: &mut CompileState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("counter increment noop effect should succeed");
        state.counter = state.counter.saturating_add(1);
    }
}

impl From<CompileState> for () {
    fn from(_value: CompileState) -> Self {}
}

#[derive(Flow)]
#[jungle(focus = FocusState)]
pub struct FocusedSegment(
    Join<Step<FocusTickSpec<0>>, Step<FocusTickSpec<1>>>,
    Step<FocusJoinMergeSpec<2>>,
);

pub struct CompileMeta;
impl NodeMetadata for CompileMeta {
    const METADATA: &'static str = "compile-times";
}

type ConditionalSegment = Transparent<
    CompileMeta,
    Conditional<CompileChooseLeft<1>, Step<TickSpec<10>>, Step<TickSpec<11>>>,
>;

#[derive(Flow)]
pub struct LoopBody(
    ConditionalSegment,
    Join<Step<JoinTickSpec<12>>, Step<JoinTickSpec<13>>>,
    Step<JoinFlattenSpec<14>>,
    Step<IncrementCounterSpec<15>>,
);

macro_rules! define_journey_24 {
    ($name:ident) => {
        #[derive(Flow)]
        pub struct $name(
            Transparent<CompileMeta, FocusedSegment>,
            While<CompileLoopOnce<1>, LoopBody>,
            Step<TickSpec<16>>,
            Step<TickSpec<17>>,
            Step<TickSpec<18>>,
            Step<TickSpec<19>>,
            Step<TickSpec<20>>,
            Step<TickSpec<21>>,
            Step<TickSpec<22>>,
            Step<TickSpec<23>>,
            Step<TickSpec<24>>,
            Step<TickSpec<25>>,
            Step<TickSpec<26>>,
            Step<TickSpec<27>>,
            Step<TickSpec<28>>,
            Step<TickSpec<29>>,
            Step<TickSpec<30>>,
            Step<TickSpec<31>>,
            Step<TickSpec<32>>,
            Step<TickSpec<33>>,
            Step<TickSpec<34>>,
            Step<TickSpec<35>>,
            Step<TickSpec<36>>,
            Step<TickSpec<37>>,
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
define_journey_double!(Journey48, Journey24);
define_journey_double!(Journey96, Journey48);
#[cfg(any(feature = "medium", feature = "large", feature = "xlarge"))]
define_journey_double!(Journey192, Journey96);
#[cfg(any(feature = "large", feature = "xlarge"))]
define_journey_double!(Journey384, Journey192);
#[cfg(feature = "xlarge")]
define_journey_double!(Journey768, Journey384);

#[cfg(all(
    feature = "small",
    not(feature = "medium"),
    not(feature = "large"),
    not(feature = "xlarge")
))]
type TierJourney = Journey96;

#[cfg(all(feature = "medium", not(feature = "large"), not(feature = "xlarge")))]
type TierJourney = Journey192;

#[cfg(all(feature = "large", not(feature = "xlarge")))]
type TierJourney = Journey384;

#[cfg(feature = "xlarge")]
type TierJourney = Journey768;

pub struct Animal01;
#[jungle::animal(id = 1001, generation = 0)]
impl Animal for Animal01 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal02;
#[jungle::animal(id = 1002, generation = 0)]
impl Animal for Animal02 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal03;
#[jungle::animal(id = 1003, generation = 0)]
impl Animal for Animal03 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal04;
#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Animal04 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal05;
#[jungle::animal(id = 1005, generation = 0)]
impl Animal for Animal05 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal06;
#[jungle::animal(id = 1006, generation = 0)]
impl Animal for Animal06 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal07;
#[jungle::animal(id = 1007, generation = 0)]
impl Animal for Animal07 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
}

pub struct Animal08;
#[jungle::animal(id = 1008, generation = 0)]
impl Animal for Animal08 {
    type State = CompileState;
    type Seed = CompileState;
    type Journey = TierJourney;
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
