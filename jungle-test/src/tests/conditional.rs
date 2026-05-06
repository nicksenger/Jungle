use jungle_sdk::types::{
    ActionCompletion, ActionStep, AspectStep, Condition, Conditional, Either, Executor, Identity,
    ManualExecutor, Running, Waiting,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1};
use jungle_sdk::Instinct;
use std::future::ready;

action!(
    LeftAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

action!(
    RightAction,
    U1,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 2))
);

animal!(
    ConditionalCreature,
    U0,
    state = i32,
    instinct = ConditionalInstinct
);

struct Left;
impl AspectStep<ConditionalCreature, LeftAction> for Left {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state + input
    }

    fn apply(state: &mut i32, output: ActionCompletion<LeftAction>) -> Self::Out {
        let value = output.expect("left action should succeed");
        *state = value;
        value
    }
}

struct Right;
impl AspectStep<ConditionalCreature, RightAction> for Right {
    type Aspect = Identity;
    type In = i32;
    type Out = bool;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state - input
    }

    fn apply(state: &mut i32, output: ActionCompletion<RightAction>) -> Self::Out {
        let value = output.expect("right action should succeed");
        *state = value;
        value % 2 == 0
    }
}

type LeftFlow = ActionStep<ConditionalCreature, LeftAction, Left>;
type RightFlow = ActionStep<ConditionalCreature, RightAction, Right>;

struct PreferLeftWhenStateIsNonNegative;
impl Condition<(i32, i32)> for PreferLeftWhenStateIsNonNegative {
    fn choose((state, _): &(i32, i32)) -> bool {
        *state >= 0
    }
}

type ConditionalFlow = Conditional<PreferLeftWhenStateIsNonNegative, LeftFlow, RightFlow>;

#[derive(Instinct)]
struct ConditionalInstinct(ConditionalFlow);

#[test]
fn conditional_run_selects_branch_from_predicate() {
    let left = <ConditionalFlow as Running>::run((5, 3));
    match left {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 8),
        Either::Right(_) => panic!("expected left branch"),
    }

    let right = <ConditionalFlow as Running>::run((-2, 3));
    match right {
        Either::Left(_) => panic!("expected right branch"),
        Either::Right((_state, request)) => assert_eq!(request.into_input(), -5),
    }
}

#[test]
fn conditional_waiting_accept_returns_either_branch_output() {
    let left = <ConditionalFlow as Waiting>::accept(Either::Left((1, Ok(9))));
    match left {
        Either::Left((state, emitted)) => {
            assert_eq!(state, 9);
            assert_eq!(emitted, 9);
        }
        Either::Right(_) => panic!("expected left output"),
    }

    let right = <ConditionalFlow as Waiting>::accept(Either::Right((1, Ok(6))));
    match right {
        Either::Left(_) => panic!("expected right output"),
        Either::Right((state, emitted)) => {
            assert_eq!(state, 6);
            assert!(emitted);
        }
    }
}

#[test]
fn executor_dynamically_selects_conditional_branch() {
    let mut left = ManualExecutor::<ConditionalCreature>::new(5);
    let left_emitted: i32 = left.next_typed(3, Ok::<i32, ()>(9)).expect("left branch");
    assert_eq!(left_emitted, 9);
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = ManualExecutor::<ConditionalCreature>::new(-2);
    let right_emitted: bool = right.next_typed(3, Ok::<i32, ()>(6)).expect("right branch");
    assert_eq!(right_emitted, true);
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}

#[test]
fn executor_requests_and_completes_conditional_branch() {
    let mut left = Executor::<ConditionalCreature>::new(5);
    let left_request: i32 = left.next_request().expect("left request");
    assert_eq!(left_request, 5);
    let left_emitted: i32 = left.complete(Ok::<i32, ()>(9)).expect("left completion");
    assert_eq!(left_emitted, 9);
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = Executor::<ConditionalCreature>::new(-2);
    let right_request: i32 = right.next_request().expect("right request");
    assert_eq!(right_request, -2);
    let right_emitted: bool = right.complete(Ok::<i32, ()>(6)).expect("right completion");
    assert!(right_emitted);
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}
