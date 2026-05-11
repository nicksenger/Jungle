use jungle::types::{
    ActionCompletion, Executor, Identity, LoopCondition, ManualExecutor, Pulse, Running, Step,
    Waiting, While,
};
use jungle::typosaurus::num::consts::U0;
use jungle::Journey;
use std::future::ready;

action!(
    TickAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

animal!(Looper, U0, state = i32, journey = LoopJourney);

struct Tick;
impl Pulse<Looper> for Tick {
    type Action = TickAction;
    type Aspect = Identity;
    type CarryIn = i32;
    type CarryOut = (bool, i32);

    fn emit(state: &i32, input: Self::CarryIn) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: ActionCompletion<TickAction>) -> Self::CarryOut {
        let value = output.expect("tick action should succeed");
        *state = value;
        (*state < 3, value)
    }
}

type TickFlow = Step<Looper, Tick>;

struct LessThanThree;
impl LoopCondition<i32> for LessThanThree {
    type CarryIn = i32;

    fn should_continue(state: &i32) -> bool {
        *state < 3
    }
}
type WhileTickFlow = While<LessThanThree, TickFlow>;

#[derive(Journey)]
struct LoopJourney(WhileTickFlow);

#[test]
fn while_running_checks_state_before_iteration() {
    let run = <WhileTickFlow as Running>::run((true, (0, 1)));
    match run {
        Some((_state, request)) => assert_eq!(request.into_input(), 1),
        None => panic!("expected iteration to run"),
    }

    let run = <WhileTickFlow as Running>::run((false, (3, 1)));
    assert!(run.is_none());
}

#[test]
fn while_waiting_passthroughs_optional_branch() {
    let waited = <WhileTickFlow as Waiting>::accept(Some((0, Ok(2))));
    match waited {
        Some((state, emitted)) => {
            assert_eq!(state, 2);
            assert_eq!(emitted, (true, 2));
        }
        None => panic!("expected waiting output"),
    }

    let waited = <WhileTickFlow as Waiting>::accept(None);
    assert!(waited.is_none());
}

#[test]
fn executor_repeats_until_condition_fails() {
    let mut loop_executor = ManualExecutor::<Looper>::new(0);
    let mut emitted = Vec::new();
    emitted.push(
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(1))
            .expect("first tick should advance"),
    );
    emitted.push(
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(2))
            .expect("second tick should advance"),
    );
    emitted.push(
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(3))
            .expect("third tick should advance"),
    );
    assert_eq!(emitted, vec![(true, 1), (true, 2), (false, 3)]);
    let done = loop_executor
        .next_request_typed::<_, i32>((false, 3))
        .expect_err("terminal carry should end loop");
    assert!(matches!(done, jungle::types::ExecutorError::Complete));
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}

#[test]
fn executor_completes_zero_iteration_loop() {
    let mut loop_executor = Executor::<Looper>::new(3);
    let run = loop_executor.next_request::<i32>();
    assert!(run.is_err());
    assert!(loop_executor.is_complete());
    assert!(loop_executor.next_executable_request(1).is_err());
}

#[test]
fn executor_threads_loop_inputs_from_previous_emitted_output() {
    let mut loop_executor = Executor::<Looper>::new(0);

    let request1: i32 = loop_executor.next_request().expect("request 1");
    assert_eq!(request1, 0);
    let emitted1: (bool, i32) = loop_executor
        .complete(Ok::<i32, ()>(1))
        .expect("complete 1");
    assert_eq!(emitted1, (true, 1));

    let request2: i32 = loop_executor.next_request().expect("request 2");
    assert_eq!(request2, 2);
    let emitted2: (bool, i32) = loop_executor
        .complete(Ok::<i32, ()>(2))
        .expect("complete 2");
    assert_eq!(emitted2, (true, 2));

    let request3: i32 = loop_executor.next_request().expect("request 3");
    assert_eq!(request3, 4);
    let emitted3: (bool, i32) = loop_executor
        .complete(Ok::<i32, ()>(3))
        .expect("complete 3");
    assert_eq!(emitted3, (false, 3));

    assert!(!loop_executor.is_complete());
    assert!(loop_executor.next_request::<i32>().is_err());
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}
