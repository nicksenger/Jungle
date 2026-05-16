use jungle_sdk::types::{
    Act, EffectCompletion, Executor, Identity, LoopCondition, ManualExecutor, Running, Step,
    Waiting, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2};
use jungle_sdk::Journey;
use std::future::ready;

effect!(
    TickEffect,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

animal!(Looper, U0, state = i32, journey = LoopJourney);

struct Tick;
impl Act<Looper> for Tick {
    type Effect = TickEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = (bool, i32);

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<TickEffect>) -> Self::Output {
        let value = output.expect("tick effect should succeed");
        *state = value;
        (*state < 3, value)
    }
}

type TickFlow = Step<Looper, Tick>;

struct LessThanThree;
impl LoopCondition<i32> for LessThanThree {
    type Arg = i32;

    fn should_continue(state: &i32) -> bool {
        *state < 3
    }
}
type WhileTickFlow = While<LessThanThree, TickFlow>;

#[derive(Journey)]
struct LoopJourney(WhileTickFlow);

effect!(
    TailEchoEffect,
    U1,
    in = (bool, i32),
    out = (bool, i32),
    err = (),
    act = |_dependency, input| ready(Ok(input))
);

animal!(
    LooperWithTail,
    U1,
    state = i32,
    journey = LoopWithTailJourney
);

struct TickWithTail;
impl Act<LooperWithTail> for TickWithTail {
    type Effect = TickEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = (bool, i32);

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<TickEffect>) -> Self::Output {
        let value = output.expect("tick effect should succeed");
        *state = value;
        (*state < 3, value)
    }
}

struct TailAfterLoop;
impl Act<LooperWithTail> for TailAfterLoop {
    type Effect = TailEchoEffect;
    type StateAspect = Identity;
    type Input = (bool, i32);
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<TailEchoEffect>) -> Self::Output {
        let (loop_should_continue, value) = output.expect("tail effect should succeed");
        *state = if loop_should_continue {
            -999
        } else {
            value + 10
        };
        *state
    }
}

type TickWithTailFlow = Step<LooperWithTail, TickWithTail>;
type WhileTickWithTailFlow = While<LessThanThree, TickWithTailFlow>;

#[derive(Journey)]
struct LoopWithTailJourney(WhileTickWithTailFlow, Step<LooperWithTail, TailAfterLoop>);

effect!(
    UnitEffect,
    U2,
    in = (),
    out = (),
    err = (),
    act = |_dependency, _input| ready(Ok(()))
);

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct NestedState {
    outer_round: u8,
    inner_step: u8,
    outer_iterations_done: u8,
}

animal!(
    NestedLooper,
    U2,
    state = NestedState,
    journey = NestedLoopJourney
);

struct InnerContinue;
impl LoopCondition<NestedState> for InnerContinue {
    type Arg = ();

    fn should_continue(state: &NestedState) -> bool {
        state.inner_step < 2
    }
}

struct OuterContinue;
impl LoopCondition<NestedState> for OuterContinue {
    type Arg = ();

    fn should_continue(state: &NestedState) -> bool {
        state.outer_round < 3
    }
}

struct InnerWork;
impl Act<NestedLooper> for InnerWork {
    type Effect = UnitEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &NestedState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut NestedState, _output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.inner_step = state.inner_step.saturating_add(1);
    }
}

struct FinishOuterRound;
impl Act<NestedLooper> for FinishOuterRound {
    type Effect = UnitEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &NestedState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut NestedState, _output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.outer_iterations_done = state.outer_iterations_done.saturating_add(1);
        state.outer_round = state.outer_round.saturating_add(1);
        state.inner_step = 0;
    }
}

type NestedInnerLoop = While<InnerContinue, Step<NestedLooper, InnerWork>>;

#[derive(Journey)]
struct NestedOuterBody(NestedInnerLoop, Step<NestedLooper, FinishOuterRound>);

type NestedOuterLoop = While<OuterContinue, NestedOuterBody>;

#[derive(Journey)]
struct NestedLoopJourney(NestedOuterLoop);

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
    assert!(matches!(done, jungle_sdk::types::ExecutorError::Complete));
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

#[test]
fn executor_advances_from_terminal_while_iteration_to_trailing_step_without_spurious_complete() {
    let mut loop_executor = Executor::<LooperWithTail>::new(0);

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

    let tail_request: (bool, i32) = loop_executor.next_request().expect("tail request");
    assert_eq!(tail_request, (false, 3));
    let tail_emitted: i32 = loop_executor
        .complete(Ok::<(bool, i32), ()>((false, 3)))
        .expect("tail completion");
    assert_eq!(tail_emitted, 13);

    assert!(loop_executor.next_request::<i32>().is_err());
    assert!(loop_executor.is_complete());
    assert_eq!(loop_executor.into_state(), 13);
}

#[test]
fn nested_while_with_trailing_step_repeats_outer_iterations() {
    let mut executor = Executor::<NestedLooper>::new(NestedState {
        outer_round: 0,
        inner_step: 0,
        outer_iterations_done: 0,
    });

    loop {
        let request = executor.next_request::<()>();
        match request {
            Ok(()) => {
                let _emitted: () = executor
                    .complete(Ok::<(), ()>(()))
                    .expect("completion should advance");
            }
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("unexpected request error: {err:?}"),
        }
    }

    assert!(executor.is_complete());
    let final_state = executor.into_state();
    assert_eq!(final_state.outer_iterations_done, 3);
    assert_eq!(final_state.outer_round, 3);
    assert_eq!(final_state.inner_step, 0);
}
