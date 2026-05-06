use jungle_sdk::instinct;
use jungle_sdk::types::{
    ActionCompletion, ActionStep, AspectStep, Identity, LoopCondition, Running, TestExecutor,
    Waiting, While,
};
use serde_json::json;
use std::future::ready;
use jungle_sdk::typosaurus::num::consts::U0;

action!(
    TickAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

animal!(Looper, U0, state = i32, instinct = LoopInstinct);

struct Tick;
impl AspectStep<Looper, TickAction> for Tick {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state + input
    }

    fn apply(state: &mut i32, output: ActionCompletion<TickAction>) -> Self::Out {
        let value = output.expect("tick action should succeed");
        *state = value;
        value
    }
}

type TickFlow = ActionStep<Looper, TickAction, Tick>;

struct LessThanThree;
impl LoopCondition<i32> for LessThanThree {
    fn should_continue(state: &i32) -> bool {
        *state < 3
    }
}

type WhileTickFlow = While<LessThanThree, TickFlow>;

#[instinct]
struct LoopInstinct(WhileTickFlow);

#[test]
fn while_running_checks_state_before_iteration() {
    let run = <WhileTickFlow as Running>::run((0, 1));
    match run {
        Some((_state, request)) => assert_eq!(request.into_input(), 1),
        None => panic!("expected iteration to run"),
    }

    let run = <WhileTickFlow as Running>::run((3, 1));
    assert!(run.is_none());
}

#[test]
fn while_waiting_passthroughs_optional_branch() {
    let waited = <WhileTickFlow as Waiting>::accept(Some((0, Ok(2))));
    match waited {
        Some((state, emitted)) => {
            assert_eq!(state, 2);
            assert_eq!(emitted, 2);
        }
        None => panic!("expected waiting output"),
    }

    let waited = <WhileTickFlow as Waiting>::accept(None);
    assert!(waited.is_none());
}

#[test]
fn test_executor_repeats_until_condition_fails() {
    let mut loop_executor = TestExecutor::<Looper>::new(0);
    let emitted = loop_executor
        .advance_to_end(vec![
            (json!(1), Ok(json!(1))),
            (json!(1), Ok(json!(2))),
            (json!(1), Ok(json!(3))),
        ])
        .expect("while loop should advance");
    assert_eq!(emitted, vec![json!(1), json!(2), json!(3)]);
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}

#[test]
fn test_executor_completes_zero_iteration_loop() {
    let loop_executor = TestExecutor::<Looper>::new(3);
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}
