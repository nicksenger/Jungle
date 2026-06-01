use futures::StreamExt;
use jungle_sdk::prelude::*;
use jungle_sdk::Optic;
use jungle_sdk::{Animals, JungleClient};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectJoinState {
    fast_ms: u64,
    slow_ms: u64,
    winner: i32,
    joined_sum: i32,
}

pub struct TimedValueEffect;
#[jungle::effect(id = 60)]
impl<J> Effect<J> for TimedValueEffect {
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    #[allow(clippy::manual_async_fn)]
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

pub struct ContextTimedValueEffect;
#[jungle::effect(id = 61)]
impl<J> Effect<J> for ContextTimedValueEffect {
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    #[allow(clippy::manual_async_fn)]
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

pub struct SelectFastSpec;
#[jungle::action]
impl Action for SelectFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = output.map_err(|_err| Failure::from("fast effect should succeed"))?;
        Ok(__absorb_out_1)
    }
}

pub struct SelectSlowSpec;
#[jungle::action]
impl Action for SelectSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = output.map_err(|_err| Failure::from("slow effect should succeed"))?;
        Ok(__absorb_out_2)
    }
}

pub struct CaptureSelectWinnerSpec;
#[jungle::action]
impl Action for CaptureSelectWinnerSpec {
    type Effect = TimedValueEffect;
    type Input = Either<i32, i32>;
    type Output = ();

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        let winner = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (0, winner)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            state.winner = output.map_err(|_err| Failure::from("winner capture should succeed"))?;
        };
        Ok(__absorb_out_3)
    }
}

#[derive(Flow)]
pub struct SelectFlowTemplate(
    Select<Step<SelectFastSpec>, Step<SelectSlowSpec>>,
    Step<CaptureSelectWinnerSpec>,
);

pub struct SelectAnimal;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for SelectAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = SelectFlowTemplate;
}

pub struct JoinFastSpec;
#[jungle::action]
impl Action for JoinFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.fast_ms, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = output.map_err(|_err| Failure::from("join fast should succeed"))?;
        Ok(__absorb_out_4)
    }
}

pub struct JoinSlowSpec;
#[jungle::action]
impl Action for JoinSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = output.map_err(|_err| Failure::from("join slow should succeed"))?;
        Ok(__absorb_out_5)
    }
}

pub struct CaptureJoinSumSpec;
#[jungle::action]
impl Action for CaptureJoinSumSpec {
    type Effect = TimedValueEffect;
    type Input = (i32, i32);
    type Output = ();

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        (0, input.0 + input.1)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_6 = {
            state.joined_sum = output.map_err(|_err| Failure::from("join sum capture should succeed"))?;
        };
        Ok(__absorb_out_6)
    }
}

#[derive(Flow)]
pub struct JoinFlowTemplate(
    Join<Step<JoinFastSpec>, Step<JoinSlowSpec>>,
    Step<CaptureJoinSumSpec>,
);

pub struct JoinAnimal;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for JoinAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = JoinFlowTemplate;
}

pub struct TimeoutSleepSpec;
#[jungle::action]
impl Action for TimeoutSleepSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.fast_ms)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_7 = {
            output.map_err(|_err| Failure::from("timeout sleep should succeed"))?;
            state.winner = -1;
            -1
        };
        Ok(__absorb_out_7)
    }
}

pub struct TimeoutSlowSpec;
#[jungle::action]
impl Action for TimeoutSlowSpec {
    type Effect = ContextTimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (state.slow_ms, 9)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_8 = {
            let value = output.map_err(|_err| Failure::from("timeout slow should succeed"))?;
            state.winner = value;
            value
        };
        Ok(__absorb_out_8)
    }
}

#[derive(Flow)]
pub struct TimeoutFlowTemplate(Select<Step<TimeoutSleepSpec>, Step<TimeoutSlowSpec>>);

pub struct TimeoutAnimal;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for TimeoutAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = TimeoutFlowTemplate;
}

pub struct SelectBranchPrefixFastSpec;
#[jungle::action]
impl Action for SelectBranchPrefixFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_9 = {
            output.map_err(|_err| Failure::from("select fast prefix should succeed"))?;
        };
        Ok(__absorb_out_9)
    }
}

pub struct SelectBranchPrefixSlowSpec;
#[jungle::action]
impl Action for SelectBranchPrefixSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (40, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_10 = {
            output.map_err(|_err| Failure::from("select slow prefix should succeed"))?;
        };
        Ok(__absorb_out_10)
    }
}

pub struct SelectBranchWinnerFastSpec;
#[jungle::action]
impl Action for SelectBranchWinnerFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (3, 7)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_11 = output.map_err(|_err| Failure::from("select fast winner should succeed"))?;
        Ok(__absorb_out_11)
    }
}

pub struct SelectBranchWinnerSlowSpec;
#[jungle::action]
impl Action for SelectBranchWinnerSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (60, 9)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_12 = output.map_err(|_err| Failure::from("select slow winner should succeed"))?;
        Ok(__absorb_out_12)
    }
}

#[derive(Flow)]
pub struct SelectBranchFastFlow(
    Step<SelectBranchPrefixFastSpec>,
    Step<SelectBranchWinnerFastSpec>,
);

#[derive(Flow)]
pub struct SelectBranchSlowFlow(
    Step<SelectBranchPrefixSlowSpec>,
    Step<SelectBranchWinnerSlowSpec>,
);

#[derive(Flow)]
pub struct SelectComposableFlowTemplate(
    Select<SelectBranchFastFlow, SelectBranchSlowFlow>,
    Step<CaptureSelectWinnerSpec>,
);

pub struct SelectComposableAnimal;

#[jungle::animal(id = 3, generation = 0)]
impl Animal for SelectComposableAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = SelectComposableFlowTemplate;
}

pub struct JoinBranchLeftPrefixSpec;
#[jungle::action]
impl Action for JoinBranchLeftPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (2, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_13 = {
            output.map_err(|_err| Failure::from("join left prefix should succeed"))?;
        };
        Ok(__absorb_out_13)
    }
}

pub struct JoinBranchRightPrefixSpec;
#[jungle::action]
impl Action for JoinBranchRightPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (2, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_14 = {
            output.map_err(|_err| Failure::from("join right prefix should succeed"))?;
        };
        Ok(__absorb_out_14)
    }
}

pub struct JoinBranchLeftValueSpec;
#[jungle::action]
impl Action for JoinBranchLeftValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (8, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_15 = output.map_err(|_err| Failure::from("join left value should succeed"))?;
        Ok(__absorb_out_15)
    }
}

pub struct JoinBranchRightValueSpec;
#[jungle::action]
impl Action for JoinBranchRightValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (10, 5)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_16 = output.map_err(|_err| Failure::from("join right value should succeed"))?;
        Ok(__absorb_out_16)
    }
}

#[derive(Flow)]
pub struct JoinBranchLeftFlow(
    Step<JoinBranchLeftPrefixSpec>,
    Step<JoinBranchLeftValueSpec>,
);

#[derive(Flow)]
pub struct JoinBranchRightFlow(
    Step<JoinBranchRightPrefixSpec>,
    Step<JoinBranchRightValueSpec>,
);

#[derive(Flow)]
pub struct JoinComposableFlowTemplate(
    Join<JoinBranchLeftFlow, JoinBranchRightFlow>,
    Step<CaptureJoinSumSpec>,
);

pub struct JoinComposableAnimal;

#[jungle::animal(id = 4, generation = 0)]
impl Animal for JoinComposableAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = JoinComposableFlowTemplate;
}

pub struct ConditionalPrefersLeft;
impl Predicate<(SelectJoinState, ())> for ConditionalPrefersLeft {
    fn eval((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct ConditionalLeftPassthroughSpec;
#[jungle::action]
impl Action for ConditionalLeftPassthroughSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_17 = output.map_err(|_err| Failure::from("conditional left passthrough should succeed"))?;
        Ok(__absorb_out_17)
    }
}

pub struct ConditionalRightPassthroughSpec;
#[jungle::action]
impl Action for ConditionalRightPassthroughSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 8)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_18 = output.map_err(|_err| Failure::from("conditional right passthrough should succeed"))?;
        Ok(__absorb_out_18)
    }
}

pub struct JoinFromConditionalLeftSpec;
#[jungle::action]
impl Action for JoinFromConditionalLeftSpec {
    type Effect = TimedValueEffect;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        let value = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (1, value)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_19 = output.map_err(|_err| Failure::from("join from conditional left should succeed"))?;
        Ok(__absorb_out_19)
    }
}

pub struct JoinFromConditionalRightSpec;
#[jungle::action]
impl Action for JoinFromConditionalRightSpec {
    type Effect = TimedValueEffect;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        let value = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (1, value + 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_20 = output.map_err(|_err| Failure::from("join from conditional right should succeed"))?;
        Ok(__absorb_out_20)
    }
}

#[derive(Flow)]
pub struct ConditionalBranchWithJoinFlow(
    Join<Step<JoinFromConditionalLeftSpec>, Step<JoinFromConditionalRightSpec>>,
    Step<CaptureJoinSumSpec>,
);

#[derive(Flow)]
pub struct ConditionalThenJoinTemplate(
    Conditional<
        ConditionalPrefersLeft,
        Step<ConditionalLeftPassthroughSpec>,
        Step<ConditionalRightPassthroughSpec>,
    >,
    ConditionalBranchWithJoinFlow,
);

pub struct ConditionalThenJoinAnimal;

#[jungle::animal(id = 5, generation = 0)]
impl Animal for ConditionalThenJoinAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = ConditionalThenJoinTemplate;
}

pub struct JoinMutatesWinnerSpec;
#[jungle::action]
impl Action for JoinMutatesWinnerSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 1)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_21 = {
            let value = output.map_err(|_err| Failure::from("join mutates winner should succeed"))?;
            state.winner = value;
            value
        };
        Ok(__absorb_out_21)
    }
}

pub struct RightBranchUsesWinnerZero;
impl Predicate<(SelectJoinState, ())> for RightBranchUsesWinnerZero {
    fn eval((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct RightZeroPrefixSpec;
#[jungle::action]
impl Action for RightZeroPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_22 = {
            output.map_err(|_err| Failure::from("right zero prefix should succeed"))?;
        };
        Ok(__absorb_out_22)
    }
}

pub struct RightZeroValueSpec;
#[jungle::action]
impl Action for RightZeroValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 20)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_23 = output.map_err(|_err| Failure::from("right zero value should succeed"))?;
        Ok(__absorb_out_23)
    }
}

pub struct RightNonZeroValueSpec;
#[jungle::action]
impl Action for RightNonZeroValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 30)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_24 = output.map_err(|_err| Failure::from("right non-zero value should succeed"))?;
        Ok(__absorb_out_24)
    }
}

#[derive(Flow)]
pub struct RightZeroFlow(Step<RightZeroPrefixSpec>, Step<RightZeroValueSpec>);

pub struct RightBranchMergeValueSpec;
#[jungle::action]
impl Action for RightBranchMergeValueSpec {
    type Effect = TimedValueEffect;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        let value = match input {
            Either::Left(value) | Either::Right(value) => value,
        };
        (0, value)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_25 = output.map_err(|_err| Failure::from("right branch merge value should succeed"))?;
        Ok(__absorb_out_25)
    }
}

#[derive(Flow)]
pub struct JoinStateDependentRightFlow(
    Conditional<RightBranchUsesWinnerZero, RightZeroFlow, Step<RightNonZeroValueSpec>>,
    Step<RightBranchMergeValueSpec>,
);

#[derive(Flow)]
pub struct JoinStateDependentFlowTemplate(
    Join<Step<JoinMutatesWinnerSpec>, JoinStateDependentRightFlow>,
    Step<CaptureJoinSumSpec>,
);

pub struct JoinStateDependentAnimal;

#[jungle::animal(id = 6, generation = 0)]
impl Animal for JoinStateDependentAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = JoinStateDependentFlowTemplate;
}

pub struct LocalConditionalPrefersLeft;
impl Predicate<(SelectJoinState, ())> for LocalConditionalPrefersLeft {
    fn eval((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct LocalJoinLeftStubASpec;
#[jungle::action]
impl Action for LocalJoinLeftStubASpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_26 = {
            output.map_err(|_err| Failure::from("local left join stub A should succeed"))?;
        };
        Ok(__absorb_out_26)
    }
}

pub struct LocalJoinLeftStubBSpec;
#[jungle::action]
impl Action for LocalJoinLeftStubBSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_27 = {
            output.map_err(|_err| Failure::from("local left join stub B should succeed"))?;
        };
        Ok(__absorb_out_27)
    }
}

pub struct LocalJoinRightStubASpec;
#[jungle::action]
impl Action for LocalJoinRightStubASpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 3)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_28 = {
            output.map_err(|_err| Failure::from("local right join stub A should succeed"))?;
        };
        Ok(__absorb_out_28)
    }
}

pub struct LocalJoinRightStubBSpec;
#[jungle::action]
impl Action for LocalJoinRightStubBSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_29 = {
            output.map_err(|_err| Failure::from("local right join stub B should succeed"))?;
        };
        Ok(__absorb_out_29)
    }
}

pub struct FlattenJoinedUnitTupleSpec;
#[jungle::action]
impl Action for FlattenJoinedUnitTupleSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_30 = {
            output.map_err(|_err| Failure::from("flatten joined unit tuple should succeed"))?;
        };
        Ok(__absorb_out_30)
    }
}

pub struct LocalTailStubSpec;
#[jungle::action]
impl Action for LocalTailStubSpec {
    type Effect = TimedValueEffect;
    type Input = Either<(), ()>;
    type Output = Either<(), ()>;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 5)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_31 = {
            output.map_err(|_err| Failure::from("local tail stub should succeed"))?;
            state.joined_sum = state.joined_sum.saturating_add(1);
            Either::Left(())
        };
        Ok(__absorb_out_31)
    }
}

#[derive(Flow)]
pub struct LocalConditionalLeftBranch(
    Join<Step<LocalJoinLeftStubASpec>, Step<LocalJoinLeftStubBSpec>>,
    Step<FlattenJoinedUnitTupleSpec>,
);

#[derive(Flow)]
pub struct LocalConditionalRightBranch(
    Join<Step<LocalJoinRightStubASpec>, Step<LocalJoinRightStubBSpec>>,
    Step<FlattenJoinedUnitTupleSpec>,
);

#[derive(Flow)]
pub struct LocalConditionalJoinTailFlow(
    Conditional<
        LocalConditionalPrefersLeft,
        LocalConditionalLeftBranch,
        LocalConditionalRightBranch,
    >,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
    Step<LocalTailStubSpec>,
);

pub struct LocalConditionalJoinTailAnimal;

#[jungle::animal(id = 7, generation = 0)]
impl Animal for LocalConditionalJoinTailAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = LocalConditionalJoinTailFlow;
}

pub struct NestedJoinInnerLeftSpec;
#[jungle::action]
impl Action for NestedJoinInnerLeftSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_32 = {
            let _ = output.map_err(|_err| Failure::from("nested join inner left should succeed"))?;
        };
        Ok(__absorb_out_32)
    }
}

pub struct NestedJoinInnerRightSpec;
#[jungle::action]
impl Action for NestedJoinInnerRightSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_33 = {
            let _ = output.map_err(|_err| Failure::from("nested join inner right should succeed"))?;
        };
        Ok(__absorb_out_33)
    }
}

pub struct NestedJoinInnerMergeSpec;
#[jungle::action]
impl Action for NestedJoinInnerMergeSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_34 = {
            output.map_err(|_err| Failure::from("nested join inner merge should succeed"))?;
        };
        Ok(__absorb_out_34)
    }
}

pub struct NestedJoinOuterRightSpec;
#[jungle::action]
impl Action for NestedJoinOuterRightSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 3)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_35 = {
            let _ = output.map_err(|_err| Failure::from("nested join outer right should succeed"))?;
        };
        Ok(__absorb_out_35)
    }
}

pub struct NestedJoinOuterMergeSpec;
#[jungle::action]
impl Action for NestedJoinOuterMergeSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_36 = {
            output.map_err(|_err| Failure::from("nested join outer merge should succeed"))?;
        };
        Ok(__absorb_out_36)
    }
}

pub struct NestedJoinTailCaptureSpec;
#[jungle::action]
impl Action for NestedJoinTailCaptureSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 1)
    }

    fn absorb(
        state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_37 = {
            let value = output.map_err(|_err| Failure::from("nested join tail capture should succeed"))?;
            state.joined_sum = state.joined_sum.saturating_add(value);
        };
        Ok(__absorb_out_37)
    }
}

#[derive(Flow)]
pub struct NestedJoinInnerFlow(
    Join<Step<NestedJoinInnerLeftSpec>, Step<NestedJoinInnerRightSpec>>,
    Step<NestedJoinInnerMergeSpec>,
);

#[derive(Flow)]
pub struct NestedJoinWithInnerNoopFlow(
    Join<NestedJoinInnerFlow, Step<NestedJoinOuterRightSpec>>,
    Step<NestedJoinOuterMergeSpec>,
    Step<NestedJoinTailCaptureSpec>,
);

pub struct NestedJoinWithInnerNoopAnimal;

#[jungle::animal(id = 8, generation = 0)]
impl Animal for NestedJoinWithInnerNoopAnimal {
    type State = SelectJoinState;
    type Seed = SelectJoinState;
    type Journey = NestedJoinWithInnerNoopFlow;
}

#[derive(Animals)]
struct SelectJoinFusedClientAnimals(LocalConditionalJoinTailAnimal);

struct SelectJoinFusedClientZoo;
impl Ecosystem for SelectJoinFusedClientZoo {
    const NAME: &'static str = "select-join-local-client-zoo";
    type Animals = SelectJoinFusedClientAnimals;
}

impl From<SelectJoinState> for () {
    fn from(_value: SelectJoinState) -> Self {}
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

#[tokio::test]
async fn select_supports_composed_multi_step_branches() {
    let mut executor = Executor::<SelectComposableAnimal>::new(SelectJoinState {
        fast_ms: 0,
        slow_ms: 0,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("composed select executor should complete");
    assert_eq!(executor.state().winner, 7);
}

#[tokio::test]
async fn join_supports_composed_multi_step_branches() {
    let mut executor = Executor::<JoinComposableAnimal>::new(SelectJoinState {
        fast_ms: 0,
        slow_ms: 0,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("composed join executor should complete");
    assert_eq!(executor.state().joined_sum, 9);
}

#[tokio::test]
async fn conditional_then_join_does_not_hang() {
    let mut executor = Executor::<ConditionalThenJoinAnimal>::new(SelectJoinState {
        fast_ms: 0,
        slow_ms: 0,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("conditional then join executor should complete");
    assert_eq!(executor.state().joined_sum, 9);
}

#[tokio::test]
async fn join_state_dependent_right_branch_does_not_wedge() {
    let mut executor = Executor::<JoinStateDependentAnimal>::new(SelectJoinState {
        fast_ms: 0,
        slow_ms: 0,
        winner: 0,
        joined_sum: 0,
    });

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("state-dependent join executor should complete");
    assert_eq!(executor.state().winner, 1);
    assert_eq!(executor.state().joined_sum, 31);
}

#[tokio::test]
async fn conditional_join_then_tail_streams_events_and_completes_with_local_client() {
    let client = jungle_sdk::FusedClient::builder()
        .namespace("select-join-conditional-join-tail")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(SelectJoinFusedClientZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<LocalConditionalJoinTailAnimal>(
            &SelectJoinState::default(),
        )
        .await
        .expect("journey should start");
    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");

    let step_event_count = tokio::time::timeout(Duration::from_secs(8), async {
        let mut total_count = 0_u32;
        while let Some(next) = subscription.next().await {
            let update = next.expect("streamed journey update should succeed");
            let (update_journey_id, should_count) = match update.event {
                RunnerUpdateOut::EffectInput { uuid, .. }
                | RunnerUpdateOut::EffectSuccessOutput { uuid, .. }
                | RunnerUpdateOut::EffectFailureOutput { uuid, .. } => (uuid, true),
                RunnerUpdateOut::SleepScheduled { uuid, .. }
                | RunnerUpdateOut::SleepFired { uuid, .. } => (uuid, false),
            };
            assert_eq!(
                update_journey_id, journey_id,
                "stream update should match journey"
            );
            if should_count {
                total_count += 1;
            }
        }
        total_count
    })
    .await
    .expect("journey update stream should finish before timeout");

    let status = client
        .journey_details(journey_id)
        .await
        .expect("journey details should succeed");
    assert_eq!(status, JourneyStatus::Completed);
    assert!(
        step_event_count >= 10,
        "expected at least 10 subscribed journey step events, got {step_event_count}"
    );

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn conditional_join_tail_direct_executor_runs_all_tail_steps() {
    let mut executor = Executor::<LocalConditionalJoinTailAnimal>::new(SelectJoinState::default());
    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("direct executor should complete");
    assert_eq!(
        executor.state().joined_sum,
        10,
        "all 10 tail stub steps should run"
    );
}

#[tokio::test]
async fn conditional_join_tail_does_not_complete_early_before_tail_progress_finishes() {
    let mut executor = Executor::<LocalConditionalJoinTailAnimal>::new(SelectJoinState::default());
    let mut observed_tail_progress = 0_i32;

    while !executor.is_complete() {
        let before = executor.state().joined_sum;
        let _ = executor
            .next_and_complete_with(())
            .await
            .expect("step should complete while journey is active");
        let after = executor.state().joined_sum;
        observed_tail_progress =
            observed_tail_progress.saturating_add(after.saturating_sub(before));
    }

    assert_eq!(
        observed_tail_progress, 10,
        "executor should report completion only after all 10 tail steps progress"
    );
    assert_eq!(
        executor.state().joined_sum,
        10,
        "final joined_sum should reflect every tail step"
    );
}

#[tokio::test]
async fn nested_join_with_inner_join_ending_in_noop_does_not_hang() {
    let mut executor = Executor::<NestedJoinWithInnerNoopAnimal>::new(SelectJoinState::default());

    let run = tokio::time::timeout(Duration::from_secs(2), async {
        while !executor.is_complete() {
            let request = executor
                .next_executable_request(())
                .expect("nested join flow should yield a request");
            let completion = request.run().await.expect("effect should run");
            executor
                .complete_serialized(completion)
                .expect("request completion should advance nested join flow");
        }
    })
    .await;
    run.expect("nested join flow should not hang");
    assert_eq!(
        executor.state().joined_sum,
        1,
        "tail step should run after nested join completion"
    );
}
