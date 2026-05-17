use jungle_sdk::animal;
use jungle_sdk::types::Animal;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    Act, ContextExecutor, EffectCompletion, EffectExec, EffectSchema, Either, Executor, Identity,
    Join, Select, Sleep, Step,
};
use jungle_sdk::typosaurus::num::consts::*;
use jungle_sdk::{Journey, Optic};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SelectJoinState {
    fast_ms: u64,
    slow_ms: u64,
    winner: i32,
    joined_sum: i32,
}

struct TimedValueEffect;
impl EffectSchema for TimedValueEffect {
    type Id = Id<U60>;
    type In = (u64, i32);
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for TimedValueEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

struct ContextTimedValueEffect;
impl EffectSchema for ContextTimedValueEffect {
    type Id = Id<U61>;
    type In = (u64, i32);
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for ContextTimedValueEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

struct SelectFast;
impl Act<SelectAnimal> for SelectFast {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("fast effect should succeed")
    }
}

struct SelectSlow;
impl Act<SelectAnimal> for SelectSlow {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("slow effect should succeed")
    }
}

struct CaptureSelectWinner;
impl Act<SelectAnimal> for CaptureSelectWinner {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = Either<i32, i32>;
    type Output = ();

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        let winner = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (0, winner)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.winner = output.expect("winner capture should succeed");
    }
}

#[derive(Journey)]
struct SelectJourney(
    Select<Step<SelectAnimal, SelectFast>, Step<SelectAnimal, SelectSlow>>,
    Step<SelectAnimal, CaptureSelectWinner>,
);

struct SelectAnimal;

#[animal]
impl Animal for SelectAnimal {
    type Id = Id<U0>;
    type Generation = U0;
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = SelectJourney;
}

struct JoinFast;
impl Act<JoinAnimal> for JoinFast {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join fast should succeed")
    }
}

struct JoinSlow;
impl Act<JoinAnimal> for JoinSlow {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join slow should succeed")
    }
}

struct CaptureJoinSum;
impl Act<JoinAnimal> for CaptureJoinSum {
    type Effect = TimedValueEffect;
    type Aspect = Identity;
    type Input = (i32, i32);
    type Output = ();

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        (0, input.0 + input.1)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.joined_sum = output.expect("join sum capture should succeed");
    }
}

#[derive(Journey)]
struct JoinJourney(
    Join<Step<JoinAnimal, JoinFast>, Step<JoinAnimal, JoinSlow>>,
    Step<JoinAnimal, CaptureJoinSum>,
);

struct JoinAnimal;

#[animal]
impl Animal for JoinAnimal {
    type Id = Id<U1>;
    type Generation = U0;
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = JoinJourney;
}

struct TimeoutSleep;
impl Act<TimeoutAnimal> for TimeoutSleep {
    type Effect = Sleep;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.fast_ms)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("timeout sleep should succeed");
        state.winner = -1;
        -1
    }
}

struct TimeoutSlow;
impl Act<TimeoutAnimal> for TimeoutSlow {
    type Effect = ContextTimedValueEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 9)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("timeout slow should succeed");
        state.winner = value;
        value
    }
}

type TimeoutJourney = Select<Step<TimeoutAnimal, TimeoutSleep>, Step<TimeoutAnimal, TimeoutSlow>>;

struct TimeoutAnimal;

#[animal]
impl Animal for TimeoutAnimal {
    type Id = Id<U2>;
    type Generation = U0;
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = TimeoutJourney;
}

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
