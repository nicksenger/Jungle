use jungle_sdk::types::{
    ActionCompletion, ActionStep, AspectStep, Executor, Identity, LoopCondition, Running, Waiting,
    While,
};
use jungle_sdk::typosaurus::num::consts::U0;
use jungle_sdk::Instinct;
use std::future::ready;

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

#[derive(Instinct)]
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
fn executor_repeats_until_condition_fails() {
    let mut loop_executor = Executor::<Looper>::new(0);
    let emitted: Vec<i32> = loop_executor
        .advance_to_end_typed(vec![
            (1, Ok::<i32, ()>(1)),
            (1, Ok::<i32, ()>(2)),
            (1, Ok::<i32, ()>(3)),
        ])
        .expect("while loop should advance");
    assert_eq!(emitted, vec![1, 2, 3]);
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}

#[test]
fn executor_completes_zero_iteration_loop() {
    let loop_executor = Executor::<Looper>::new(3);
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 3);
}
