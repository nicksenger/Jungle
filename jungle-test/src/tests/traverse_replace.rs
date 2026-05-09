use jungle_sdk::types::{
    ActionCompletion, Condition, Conditional, Identity, Impulse, LoopCondition, Reflex, ReplaceFlow,
    ReplaceImpulse, TraverseFlow, TraverseImpulse, While,
};
use jungle_sdk::typosaurus::num::consts::{U20, U21, U22, U23, U24};
use jungle_sdk::Journey;

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

struct UseLeftBranch;
impl Condition<(i32, ())> for UseLeftBranch {
    fn choose((state, _): &(i32, ())) -> bool {
        *state >= 0
    }
}

struct KeepLooping;
impl LoopCondition<i32> for KeepLooping {
    fn should_continue(state: &i32) -> bool {
        *state < 1
    }
}

#[derive(jungle_sdk::Flow)]
struct TraverseFlowShape(
    Impulse<TraverseAnima, StepA>,
    Conditional<
        UseLeftBranch,
        Impulse<TraverseAnima, StepB>,
        Impulse<TraverseAnima, StepC>,
    >,
    While<KeepLooping, Impulse<TraverseAnima, StepD>>,
);

#[derive(Journey)]
struct TraverseJourney(TraverseFlowShape);

impl jungle_sdk::types::Anima for TraverseAnima {
    type Id = jungle_sdk::types::Id<U24>;
    type State = i32;
    type Seed = i32;
    type Journey = TraverseJourney;
}

#[derive(Clone, Copy)]
struct TraverseCounter;

impl<Step> TraverseImpulse<Step, usize> for TraverseCounter {
    type Output = usize;

    fn traverse(input: usize) -> Self::Output {
        input + 1
    }
}

#[derive(Clone, Copy)]
struct ReplaceCounter;

impl<Step> ReplaceImpulse<Step, usize> for ReplaceCounter {
    type Output = usize;

    fn replace(input: usize) -> Self::Output {
        input + 1
    }
}

#[test]
fn traverse_and_replace_visit_each_impulse_leaf() {
    let (_, traversed) =
        <TraverseJourney as TraverseFlow<(TraverseCounter, usize)>>::traverse((TraverseCounter, 0));
    let (_, replaced) =
        <TraverseJourney as ReplaceFlow<(ReplaceCounter, usize)>>::replace((ReplaceCounter, 0));

    assert_eq!(traversed, 4);
    assert_eq!(replaced, 4);
}
