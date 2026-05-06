use jungle_sdk::types::{
    ActionCompletion, Task, AspectStep, Condition, Conditional, Either, Executor, Identity,
    ManualExecutor, Running, Waiting,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1};
use jungle_sdk::Instinct;
use std::future::ready;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

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

    fn process(state: &mut i32, output: ActionCompletion<LeftAction>) -> Self::Out {
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

    fn process(state: &mut i32, output: ActionCompletion<RightAction>) -> Self::Out {
        let value = output.expect("right action should succeed");
        *state = value;
        value % 2 == 0
    }
}

type LeftFlow = Task<ConditionalCreature, LeftAction, Left>;
type RightFlow = Task<ConditionalCreature, RightAction, Right>;

struct PreferLeftWhenStateIsNonNegative;
impl Condition<(i32, i32)> for PreferLeftWhenStateIsNonNegative {
    fn choose((state, _): &(i32, i32)) -> bool {
        *state >= 0
    }
}

type ConditionalFlow = Conditional<PreferLeftWhenStateIsNonNegative, LeftFlow, RightFlow>;

#[derive(Instinct)]
struct ConditionalInstinct(ConditionalFlow);

fn run_now<F: Future>(future: F) -> F::Output {
    fn raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            raw_waker()
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}
        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
        )
    }

    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    match Future::poll(future.as_mut(), &mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test action future must resolve immediately"),
    }
}

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

#[test]
fn executor_executable_request_runs_without_static_action_dispatch() {
    let mut left = Executor::<ConditionalCreature>::new(5);
    let request = left
        .next_executable_request(0i32)
        .expect("left executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("left request should deserialize");
    assert_eq!(input, 5);
    let completion = run_now(request.run()).expect("left action should execute");
    let _left_emitted = left
        .complete_serialized(completion)
        .expect("left completion should process");
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 6);

    let mut right = Executor::<ConditionalCreature>::new(-2);
    let request = right
        .next_executable_request(0i32)
        .expect("right executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("right request should deserialize");
    assert_eq!(input, -2);
    let completion = run_now(request.run()).expect("right action should execute");
    let _right_emitted = right
        .complete_serialized(completion)
        .expect("right completion should process");
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 0);
}
