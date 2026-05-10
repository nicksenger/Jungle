use jungle_sdk::types::{
    Act, ActionCompletion, Conditional, Identity, LoopCondition, ReplaceStep, Step, TraverseStep,
    While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::{U20, U21, U22, U23, U24};

action!(
    TraverseAAction,
    U20,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
action!(
    TraverseBAction,
    U21,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
action!(
    TraverseCAction,
    U22,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);
action!(
    TraverseDAction,
    U23,
    in = (),
    out = (),
    err = (),
    act = |_d, _input| std::future::ready(Ok(()))
);

struct TraverseAnimal;

struct StepA;
impl Act<TraverseAnimal> for StepA {
    type Action = TraverseAAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &i32, _input: Self::In) -> Self::In {}

    fn absorb(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step A should succeed");
    }
}

struct StepB;
impl Act<TraverseAnimal> for StepB {
    type Action = TraverseBAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &i32, _input: Self::In) -> Self::In {}

    fn absorb(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step B should succeed");
    }
}

struct StepC;
impl Act<TraverseAnimal> for StepC {
    type Action = TraverseCAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &i32, _input: Self::In) -> Self::In {}

    fn absorb(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step C should succeed");
    }
}

struct StepD;
impl Act<TraverseAnimal> for StepD {
    type Action = TraverseDAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &i32, _input: Self::In) -> Self::In {}

    fn absorb(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step D should succeed");
    }
}

struct KeepLooping;
impl LoopCondition<i32> for KeepLooping {
    type CarryIn = ();

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
    type State = i32;
    type Seed = i32;
    type Journey = SourceFlow;
}

impl jungle_sdk::types::AnimalObservation for TraverseAnimal {
    type Adapter = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::AnimalPerturbation for TraverseAnimal {
    type Adapter = jungle_sdk::types::NoopPerturbation;
}
