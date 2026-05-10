use jungle_sdk::types::{
    Act, ActionCompletion, Conditional, Either, Executor, Identity, ManualExecutor,
    Running, Step, Waiting,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1};
use jungle_sdk::Journey;
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
    ConditionalAnimal,
    U0,
    state = i32,
    journey = ConditionalJourney
);

struct Left;
#[jungle_sdk::detect]
impl Act<ConditionalAnimal> for Left {
    type Action = LeftAction;
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn emit(state: &i32, input: Self::In) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: ActionCompletion<LeftAction>) -> Self::Out {
        let value = output.expect("left action should succeed");
        *state = value;
        value
    }
}

struct Right;
impl Act<ConditionalAnimal> for Right {
    type Action = RightAction;
    type Aspect = Identity;
    type In = i32;
    type Out = bool;

    fn emit(state: &i32, input: Self::In) -> i32 {
        *state - input
    }

    fn absorb(state: &mut i32, output: ActionCompletion<RightAction>) -> Self::Out {
        let value = output.expect("right action should succeed");
        *state = value;
        value % 2 == 0
    }
}

type LeftFlow = Step<ConditionalAnimal, Left>;
type RightFlow = Step<ConditionalAnimal, Right>;

struct PreferLeftWhenStateIsNonNegative;

type ConditionalFlow = Conditional<PreferLeftWhenStateIsNonNegative, LeftFlow, RightFlow>;

#[derive(Journey)]
struct ConditionalJourney(ConditionalFlow);

#[test]
fn conditional_run_selects_branch_from_predicate() {
    let left = <ConditionalFlow as Running>::run((true, (5, 3)));
    match left {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 8),
        Either::Right(_) => panic!("expected left branch"),
    }

    let right = <ConditionalFlow as Running>::run((false, (-2, 3)));
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
    let mut left = ManualExecutor::<ConditionalAnimal>::new(5);
    let left_emitted: i32 = left
        .next_typed((true, 3), Ok::<i32, ()>(9))
        .expect("left branch");
    assert_eq!(left_emitted, 9);
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = ManualExecutor::<ConditionalAnimal>::new(-2);
    let right_emitted: bool = right
        .next_typed((false, 3), Ok::<i32, ()>(6))
        .expect("right branch");
    assert_eq!(right_emitted, true);
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}

#[test]
fn executor_requests_and_completes_conditional_branch() {
    let mut left = Executor::<ConditionalAnimal>::new(5);
    let left_request: i32 = left.next_request::<(bool, i32)>().expect("left request");
    assert_eq!(left_request, 5);
    let left_emitted: i32 = left.complete(Ok::<i32, ()>(9)).expect("left completion");
    assert_eq!(left_emitted, 9);
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = Executor::<ConditionalAnimal>::new(-2);
    let right_request: i32 = right.next_request::<(bool, i32)>().expect("right request");
    assert_eq!(right_request, -2);
    let right_emitted: bool = right.complete(Ok::<i32, ()>(6)).expect("right completion");
    assert!(right_emitted);
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}

#[tokio::test]
async fn executor_executable_request_runs_without_static_action_dispatch() {
    let mut left = Executor::<ConditionalAnimal>::new(5);
    let request = left
        .next_executable_request((true, 0i32))
        .expect("left executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("left request should deserialize");
    assert_eq!(input, 5);
    let completion = request.run().await.expect("left action should execute");
    let _left_emitted = left
        .complete_serialized(completion)
        .expect("left completion should process");
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 6);

    let mut right = Executor::<ConditionalAnimal>::new(-2);
    let request = right
        .next_executable_request((false, 0i32))
        .expect("right executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("right request should deserialize");
    assert_eq!(input, -2);
    let completion = request.run().await.expect("right action should execute");
    let _right_emitted = right
        .complete_serialized(completion)
        .expect("right completion should process");
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 0);
}
