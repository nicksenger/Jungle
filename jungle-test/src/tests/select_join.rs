use jungle_sdk::types::{
    Action, ActionCompletion, ContextExecutor, Either, Executor, Identity, IoLens, IoMapper, Join,
    Pulse, Select, Sleep, SleepDependency, Step,
};
use jungle_sdk::{Journey, Optic};
use jungle_sdk::typosaurus::num::consts::U0;
use std::time::Duration;

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SelectJoinState {
    fast_ms: u64,
    slow_ms: u64,
    winner: i32,
    joined_sum: i32,
}

struct TimedValueAction;
impl jungle_sdk::types::ActionMember for TimedValueAction {}
impl Action for TimedValueAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U60>;
    type Dependency = ();
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

struct ContextTimedValueAction;
impl jungle_sdk::types::ActionMember for ContextTimedValueAction {}
impl Action for ContextTimedValueAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U61>;
    type Dependency = SleepDependency;
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

struct SelectFast;
impl Pulse<SelectAnimal> for SelectFast {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("fast action should succeed")
    }
}

struct SelectSlow;
impl Pulse<SelectAnimal> for SelectSlow {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("slow action should succeed")
    }
}

struct CaptureWinnerValue;
impl Pulse<SelectAnimal> for CaptureWinnerValue {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = i32;
    type Ret = i32;

    fn emit(_state: &SelectJoinState, input: Self::Arg) -> (u64, i32) {
        (0, input)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        let winner = output.expect("winner capture should succeed");
        state.winner = winner;
        winner
    }
}

struct WinnerEitherMap;
impl IoMapper<i32, i32, U0> for WinnerEitherMap {
    type Arg = Either<i32, i32>;
    type Ret = Either<i32, i32>;
    type Carry = bool;

    fn split(input: Self::Arg) -> (Self::Carry, i32) {
        match input {
            Either::Left(value) => (true, value),
            Either::Right(value) => (false, value),
        }
    }

    fn merge(carry: Self::Carry, focus: i32) -> Self::Ret {
        if carry {
            Either::Left(focus)
        } else {
            Either::Right(focus)
        }
    }
}

type CaptureSelectWinner = IoLens<CaptureWinnerValue, U0, WinnerEitherMap>;

#[derive(Journey)]
struct SelectJourney(
    Select<Step<SelectAnimal, SelectFast>, Step<SelectAnimal, SelectSlow>>,
    Step<SelectAnimal, CaptureSelectWinner>,
);

animal!(
    SelectAnimal,
    jungle_sdk::typosaurus::num::consts::U0,
    SelectJoinState,
    SelectJourney
);

struct JoinFast;
impl Pulse<JoinAnimal> for JoinFast {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("join fast should succeed")
    }
}

struct JoinSlow;
impl Pulse<JoinAnimal> for JoinSlow {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("join slow should succeed")
    }
}

struct CaptureJoinSumValue;
impl Pulse<JoinAnimal> for CaptureJoinSumValue {
    type Action = TimedValueAction;
    type Aspect = Identity;
    type Arg = (i32, i32);
    type Ret = i32;

    fn emit(_state: &SelectJoinState, input: Self::Arg) -> (u64, i32) {
        (0, input.0 + input.1)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        let total = output.expect("join sum capture should succeed");
        state.joined_sum = total;
        total
    }
}

struct JoinTupleMap;
impl IoMapper<(i32, i32), i32, U0> for JoinTupleMap {
    type Arg = (i32, i32);
    type Ret = (i32, i32);
    type Carry = i32;

    fn split(input: Self::Arg) -> (Self::Carry, (i32, i32)) {
        (input.0, input)
    }

    fn merge(carry: Self::Carry, focus: i32) -> Self::Ret {
        (carry, focus)
    }
}

type CaptureJoinSum = IoLens<CaptureJoinSumValue, U0, JoinTupleMap>;

#[derive(Journey)]
struct JoinJourney(
    Join<Step<JoinAnimal, JoinFast>, Step<JoinAnimal, JoinSlow>>,
    Step<JoinAnimal, CaptureJoinSum>,
);

animal!(
    JoinAnimal,
    jungle_sdk::typosaurus::num::consts::U1,
    SelectJoinState,
    JoinJourney
);

struct TimeoutSleep;
impl Pulse<TimeoutAnimal> for TimeoutSleep {
    type Action = Sleep;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> Duration {
        Duration::from_millis(state.fast_ms)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("timeout sleep should succeed");
        state.winner = -1;
        -1
    }
}

struct TimeoutSlow;
impl Pulse<TimeoutAnimal> for TimeoutSlow {
    type Action = ContextTimedValueAction;
    type Aspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(state: &SelectJoinState, _input: Self::Arg) -> (u64, i32) {
        (state.slow_ms, 9)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        let value = output.expect("timeout slow should succeed");
        state.winner = value;
        value
    }
}

type TimeoutJourney = Select<Step<TimeoutAnimal, TimeoutSleep>, Step<TimeoutAnimal, TimeoutSlow>>;

animal!(
    TimeoutAnimal,
    jungle_sdk::typosaurus::num::consts::U2,
    SelectJoinState,
    TimeoutJourney
);

#[tokio::test]
async fn select_returns_first_completed_branch() {
    let mut executor = Executor::<SelectAnimal>::new(SelectJoinState {
        fast_ms: 10,
        slow_ms: 60,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("select executor should complete");
    assert_eq!(executor.state().winner, 1);
}

#[tokio::test]
async fn join_waits_for_both_and_returns_tuple_outputs() {
    let mut executor = Executor::<JoinAnimal>::new(SelectJoinState {
        fast_ms: 10,
        slow_ms: 40,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("join executor should complete");
    assert_eq!(executor.state().joined_sum, 3);
}

#[tokio::test]
async fn select_fast_branch_wins_in_race() {
    let mut executor = Executor::<SelectAnimal>::new(SelectJoinState {
        fast_ms: 1,
        slow_ms: 90,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("select race executor should complete");
    assert_eq!(executor.state().winner, 1);
}

#[tokio::test]
async fn select_supports_sleep_as_timeout_branch() {
    let mut executor = ContextExecutor::<_, TimeoutAnimal>::new(
        std::sync::Arc::new(()),
        SelectJoinState {
            fast_ms: 15,
            slow_ms: 120,
            winner: 0,
            joined_sum: 0,
        },
    );

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("timeout select executor should complete");
    assert_eq!(executor.state().winner, -1);
}
