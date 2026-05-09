use jungle_sdk::types::{ActionCompletion, Conditional, Identity, Impulse, LoopCondition, Reflex, ReplaceStep, TraverseStep, While};
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

struct TraverseAnima;

struct StepA;
impl Reflex<TraverseAnima> for StepA {
    type Action = TraverseAAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &i32, _input: Self::In) -> Self::In {}

    fn process(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step A should succeed");
    }
}

struct StepB;
impl Reflex<TraverseAnima> for StepB {
    type Action = TraverseBAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &i32, _input: Self::In) -> Self::In {}

    fn process(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step B should succeed");
    }
}

struct StepC;
impl Reflex<TraverseAnima> for StepC {
    type Action = TraverseCAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &i32, _input: Self::In) -> Self::In {}

    fn process(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step C should succeed");
    }
}

struct StepD;
impl Reflex<TraverseAnima> for StepD {
    type Action = TraverseDAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &i32, _input: Self::In) -> Self::In {}

    fn process(_state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("step D should succeed");
    }
}

struct KeepLooping;
impl LoopCondition<i32> for KeepLooping {
    fn should_continue(state: &i32) -> bool {
        *state < 1
    }
}

type SourceFlow = Conditional<
    KeepLooping,
    Impulse<TraverseAnima, StepA>,
    While<KeepLooping, Impulse<TraverseAnima, StepB>>,
>;

struct Seen<T>(core::marker::PhantomData<T>);
struct TraverseMapper;
impl<Step> TraverseStep<Step> for TraverseMapper {
    type Output = Seen<Step>;
}

struct ReplaceMapper;
impl ReplaceStep<Impulse<TraverseAnima, StepA>> for ReplaceMapper {
    type Output = Impulse<TraverseAnima, StepC>;
}
impl ReplaceStep<Impulse<TraverseAnima, StepB>> for ReplaceMapper {
    type Output = Impulse<TraverseAnima, StepD>;
}

#[test]
fn traverse_and_replace_are_type_level_transformations() {
    type Traversed = jungle_sdk::types::Traversed<SourceFlow, TraverseMapper>;
    type ExpectedTraversed = Conditional<
        KeepLooping,
        Seen<Impulse<TraverseAnima, StepA>>,
        While<KeepLooping, Seen<Impulse<TraverseAnima, StepB>>>,
    >;
    assert_type_eq!(Traversed, ExpectedTraversed);

    type Replaced = jungle_sdk::types::Replaced<SourceFlow, ReplaceMapper>;
    type ExpectedReplaced = Conditional<
        KeepLooping,
        Impulse<TraverseAnima, StepC>,
        While<KeepLooping, Impulse<TraverseAnima, StepD>>,
    >;
    assert_type_eq!(Replaced, ExpectedReplaced);
}

impl jungle_sdk::types::Anima for TraverseAnima {
    type Id = jungle_sdk::types::Id<U24>;
    type State = i32;
    type Seed = i32;
    type Journey = SourceFlow;
}
