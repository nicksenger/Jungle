use jungle_sdk::types::{
    Act, Conditional, EffectCompletion, Identity, LoopCondition, ReplaceStep, Step, TraverseStep,
    While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::{U20, U21, U22, U23, U24};

effect!(
    TraverseAEffect,
    U20,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
effect!(
    TraverseBEffect,
    U21,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
effect!(
    TraverseCEffect,
    U22,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
effect!(
    TraverseDEffect,
    U23,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);

struct TraverseAnimal;

struct StepA;
impl Act<TraverseAnimal> for StepA {
    type Effect = TraverseAEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step A should succeed");
    }
}

struct StepB;
impl Act<TraverseAnimal> for StepB {
    type Effect = TraverseBEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step B should succeed");
    }
}

struct StepC;
impl Act<TraverseAnimal> for StepC {
    type Effect = TraverseCEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step C should succeed");
    }
}

struct StepD;
impl Act<TraverseAnimal> for StepD {
    type Effect = TraverseDEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step D should succeed");
    }
}

struct KeepLooping;
impl LoopCondition<i32> for KeepLooping {
    type Arg = ();

    fn should_continue(state: &i32) -> bool {
        *state < 1
    }
}

type SourceFlow = Conditional<
    KeepLooping,
    Step<TraverseAnimal, StepA>,
    While<KeepLooping, Step<TraverseAnimal, StepB>>,
>;

struct Seen<T>(core::marker::PhantomData<T>);
struct TraverseMapper;
impl<Step> TraverseStep<Step> for TraverseMapper {
    type Output = Seen<Step>;
}

struct ReplaceMapper;
impl ReplaceStep<Step<TraverseAnimal, StepA>> for ReplaceMapper {
    type Output = Step<TraverseAnimal, StepC>;
}
impl ReplaceStep<Step<TraverseAnimal, StepB>> for ReplaceMapper {
    type Output = Step<TraverseAnimal, StepD>;
}

#[test]
fn traverse_and_replace_are_type_level_transformations() {
    type Traversed = jungle_sdk::types::Traversed<SourceFlow, TraverseMapper>;
    type ExpectedTraversed = Conditional<
        KeepLooping,
        Seen<Step<TraverseAnimal, StepA>>,
        While<KeepLooping, Seen<Step<TraverseAnimal, StepB>>>,
    >;
    assert_type_eq!(Traversed, ExpectedTraversed);

    type Replace = jungle_sdk::types::Replace<SourceFlow, ReplaceMapper>;
    type ExpectedReplaced = Conditional<
        KeepLooping,
        Step<TraverseAnimal, StepC>,
        While<KeepLooping, Step<TraverseAnimal, StepD>>,
    >;
    assert_type_eq!(Replace, ExpectedReplaced);
}

impl jungle_sdk::types::Animal for TraverseAnimal {
    type Id = jungle_sdk::types::Id<U24>;
    type Generation = jungle_sdk::typosaurus::num::consts::U0;
    type State = i32;
    type Seed = i32;
    type Journey = SourceFlow;
}

impl jungle_sdk::types::AnimalObservation for TraverseAnimal {
    type Bridge = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::AnimalPerturbation for TraverseAnimal {
    type Bridge = jungle_sdk::types::NoopPerturbation;
}
