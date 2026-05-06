use jungle_sdk::{Instinct, Jungle};
use jungle_sdk::types::{
    ActionCompletion, ActionStep, AspectStep, Condition, Conditional, Either, Identity, Running,
    TestExecutor, Waiting,
};
use serde_json::json;
use std::future::ready;
use jungle_sdk::typosaurus::num::consts::{U0, U1};

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

#[derive(Jungle, Instinct)]
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
fn test_executor_dynamically_selects_conditional_branch() {
    let mut left = TestExecutor::<ConditionalCreature>::new(5);
    let left_emitted = left.next(json!(3), Ok(json!(9))).expect("left branch");
    assert_eq!(left_emitted, json!(9));
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = TestExecutor::<ConditionalCreature>::new(-2);
    let right_emitted = right.next(json!(3), Ok(json!(6))).expect("right branch");
    assert_eq!(right_emitted, json!(true));
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}
