//! Reusable pulses and predicates for tiny test fixtures.

use crate::testing::actions;
use crate::testing::state::{CounterState, RaceState, SleepCycleState};
use jungle_types::{
    ActionCompletion, Animal, Condition, Either, Identity, LoopCondition, Pulse, Sleep,
};

pub struct CounterAddOne;
impl<T> Pulse<T> for CounterAddOne
where
    T: Animal<State = CounterState>,
{
    type Action = actions::AddOne;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(
        _state: &CounterState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
    }

    fn absorb(state: &mut CounterState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let delta = output.expect("counter add-one should succeed");
        state.value += delta;
        state.value
    }
}

pub struct CounterAddTwo;
impl<T> Pulse<T> for CounterAddTwo
where
    T: Animal<State = CounterState>,
{
    type Action = actions::AddTwo;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(
        _state: &CounterState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
    }

    fn absorb(state: &mut CounterState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let delta = output.expect("counter add-two should succeed");
        state.value += delta;
        state.value
    }
}

pub struct CounterStateIsEven;
impl Condition<(CounterState, ())> for CounterStateIsEven {
    fn choose((state, _): &(CounterState, ())) -> bool {
        state.value % 2 == 0
    }
}

pub struct RaceFastBranch;
impl<T> Pulse<T> for RaceFastBranch
where
    T: Animal<State = RaceState>,
{
    type Action = actions::TimedValue;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(
        state: &RaceState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
        (state.fast_ms, 1)
    }

    fn absorb(_state: &mut RaceState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        output.expect("fast race branch should succeed")
    }
}

pub struct RaceSlowBranch;
impl<T> Pulse<T> for RaceSlowBranch
where
    T: Animal<State = RaceState>,
{
    type Action = actions::TimedValue;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = i32;

    fn emit(
        state: &RaceState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
        (state.slow_ms, 2)
    }

    fn absorb(_state: &mut RaceState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        output.expect("slow race branch should succeed")
    }
}

pub struct CaptureSelectWinner;
impl<T> Pulse<T> for CaptureSelectWinner
where
    T: Animal<State = RaceState>,
{
    type Action = actions::TimedValue;
    type StateAspect = Identity;
    type Arg = Either<i32, i32>;
    type Ret = ();

    fn emit(
        _state: &RaceState,
        input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
        let winner = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (0, winner)
    }

    fn absorb(state: &mut RaceState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        state.winner = output.expect("winner capture should succeed");
    }
}

pub struct CaptureJoinSum;
impl<T> Pulse<T> for CaptureJoinSum
where
    T: Animal<State = RaceState>,
{
    type Action = actions::TimedValue;
    type StateAspect = Identity;
    type Arg = (i32, i32);
    type Ret = ();

    fn emit(
        _state: &RaceState,
        input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
        (0, input.0 + input.1)
    }

    fn absorb(state: &mut RaceState, output: ActionCompletion<Self::Action>) -> Self::Ret {
        state.joined_sum = output.expect("join sum capture should succeed");
    }
}

pub struct SleepAddBefore;
impl<T> Pulse<T> for SleepAddBefore
where
    T: Animal<State = SleepCycleState>,
{
    type Action = actions::AddOne;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(
        _state: &SleepCycleState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
    }

    fn absorb(
        state: &mut SleepCycleState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        state.counter += output.expect("pre-sleep increment should succeed");
        state.phase += 1;
    }
}

pub struct SleepForStateWake;
impl<T> Pulse<T> for SleepForStateWake
where
    T: Animal<State = SleepCycleState>,
{
    type Action = Sleep;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(
        state: &SleepCycleState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
        std::time::Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(
        state: &mut SleepCycleState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        output.expect("scheduler sleep should resume successfully");
        state.phase += 1;
    }
}

pub struct SleepAddAfter;
impl<T> Pulse<T> for SleepAddAfter
where
    T: Animal<State = SleepCycleState>,
{
    type Action = actions::AddOne;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(
        _state: &SleepCycleState,
        _input: Self::Arg,
    ) -> <Self::Action as jungle_types::Action>::In {
    }

    fn absorb(
        state: &mut SleepCycleState,
        output: ActionCompletion<Self::Action>,
    ) -> Self::Ret {
        state.counter += output.expect("post-sleep increment should succeed");
        state.phase += 1;
    }
}

pub struct SleepCycleNotComplete;
impl LoopCondition<SleepCycleState> for SleepCycleNotComplete {
    type Arg = ();

    fn should_continue(state: &SleepCycleState) -> bool {
        state.phase < 3
    }
}

pub struct SleepCyclePhaseZero;
impl Condition<(SleepCycleState, ())> for SleepCyclePhaseZero {
    fn choose((state, _): &(SleepCycleState, ())) -> bool {
        state.phase == 0
    }
}

pub struct SleepCyclePhaseOne;
impl Condition<(SleepCycleState, ())> for SleepCyclePhaseOne {
    fn choose((state, _): &(SleepCycleState, ())) -> bool {
        state.phase == 1
    }
}
