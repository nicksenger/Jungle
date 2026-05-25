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
#[jungle::act]
impl Act for SelectFastSpec {
    type Effect = TimedValueEffect;
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

pub struct SelectSlowSpec;
#[jungle::act]
impl Act for SelectSlowSpec {
    type Effect = TimedValueEffect;
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

pub struct CaptureSelectWinnerSpec;
#[jungle::act]
impl Act for CaptureSelectWinnerSpec {
    type Effect = TimedValueEffect;
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
#[jungle::act]
impl Act for JoinFastSpec {
    type Effect = TimedValueEffect;
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

pub struct JoinSlowSpec;
#[jungle::act]
impl Act for JoinSlowSpec {
    type Effect = TimedValueEffect;
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

pub struct CaptureJoinSumSpec;
#[jungle::act]
impl Act for CaptureJoinSumSpec {
    type Effect = TimedValueEffect;
    type Input = (i32, i32);
    type Output = ();

    fn emit(_state: &SelectJoinState, input: Self::Input) -> (u64, i32) {
        (0, input.0 + input.1)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.joined_sum = output.expect("join sum capture should succeed");
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
#[jungle::act]
impl Act for TimeoutSleepSpec {
    type Effect = Sleep;
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

pub struct TimeoutSlowSpec;
#[jungle::act]
impl Act for TimeoutSlowSpec {
    type Effect = ContextTimedValueEffect;
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
#[jungle::act]
impl Act for SelectBranchPrefixFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("select fast prefix should succeed");
    }
}

pub struct SelectBranchPrefixSlowSpec;
#[jungle::act]
impl Act for SelectBranchPrefixSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (40, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("select slow prefix should succeed");
    }
}

pub struct SelectBranchWinnerFastSpec;
#[jungle::act]
impl Act for SelectBranchWinnerFastSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (3, 7)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("select fast winner should succeed")
    }
}

pub struct SelectBranchWinnerSlowSpec;
#[jungle::act]
impl Act for SelectBranchWinnerSlowSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (60, 9)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("select slow winner should succeed")
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
#[jungle::act]
impl Act for JoinBranchLeftPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (2, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join left prefix should succeed");
    }
}

pub struct JoinBranchRightPrefixSpec;
#[jungle::act]
impl Act for JoinBranchRightPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (2, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join right prefix should succeed");
    }
}

pub struct JoinBranchLeftValueSpec;
#[jungle::act]
impl Act for JoinBranchLeftValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (8, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join left value should succeed")
    }
}

pub struct JoinBranchRightValueSpec;
#[jungle::act]
impl Act for JoinBranchRightValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (10, 5)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join right value should succeed")
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
impl Condition<(SelectJoinState, ())> for ConditionalPrefersLeft {
    fn choose((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct ConditionalLeftPassthroughSpec;
#[jungle::act]
impl Act for ConditionalLeftPassthroughSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("conditional left passthrough should succeed")
    }
}

pub struct ConditionalRightPassthroughSpec;
#[jungle::act]
impl Act for ConditionalRightPassthroughSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 8)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("conditional right passthrough should succeed")
    }
}

pub struct JoinFromConditionalLeftSpec;
#[jungle::act]
impl Act for JoinFromConditionalLeftSpec {
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
    ) -> Self::Output {
        output.expect("join from conditional left should succeed")
    }
}

pub struct JoinFromConditionalRightSpec;
#[jungle::act]
impl Act for JoinFromConditionalRightSpec {
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
    ) -> Self::Output {
        output.expect("join from conditional right should succeed")
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
#[jungle::act]
impl Act for JoinMutatesWinnerSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 1)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("join mutates winner should succeed");
        state.winner = value;
        value
    }
}

pub struct RightBranchUsesWinnerZero;
impl Condition<(SelectJoinState, ())> for RightBranchUsesWinnerZero {
    fn choose((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct RightZeroPrefixSpec;
#[jungle::act]
impl Act for RightZeroPrefixSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 0)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("right zero prefix should succeed");
    }
}

pub struct RightZeroValueSpec;
#[jungle::act]
impl Act for RightZeroValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 20)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("right zero value should succeed")
    }
}

pub struct RightNonZeroValueSpec;
#[jungle::act]
impl Act for RightNonZeroValueSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 30)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("right non-zero value should succeed")
    }
}

#[derive(Flow)]
pub struct RightZeroFlow(Step<RightZeroPrefixSpec>, Step<RightZeroValueSpec>);

pub struct RightBranchMergeValueSpec;
#[jungle::act]
impl Act for RightBranchMergeValueSpec {
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
    ) -> Self::Output {
        output.expect("right branch merge value should succeed")
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
impl Condition<(SelectJoinState, ())> for LocalConditionalPrefersLeft {
    fn choose((state, _): &(SelectJoinState, ())) -> bool {
        state.winner == 0
    }
}

pub struct LocalJoinLeftStubASpec;
#[jungle::act]
impl Act for LocalJoinLeftStubASpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("local left join stub A should succeed");
    }
}

pub struct LocalJoinLeftStubBSpec;
#[jungle::act]
impl Act for LocalJoinLeftStubBSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("local left join stub B should succeed");
    }
}

pub struct LocalJoinRightStubASpec;
#[jungle::act]
impl Act for LocalJoinRightStubASpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 3)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("local right join stub A should succeed");
    }
}

pub struct LocalJoinRightStubBSpec;
#[jungle::act]
impl Act for LocalJoinRightStubBSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (1, 4)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("local right join stub B should succeed");
    }
}

pub struct FlattenJoinedUnitTupleSpec;
#[jungle::act]
impl Act for FlattenJoinedUnitTupleSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("flatten joined unit tuple should succeed");
    }
}

pub struct LocalTailStubSpec;
#[jungle::act]
impl Act for LocalTailStubSpec {
    type Effect = TimedValueEffect;
    type Input = Either<(), ()>;
    type Output = Either<(), ()>;

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 5)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("local tail stub should succeed");
        state.joined_sum = state.joined_sum.saturating_add(1);
        Either::Left(())
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
#[jungle::act]
impl Act for NestedJoinInnerLeftSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 1)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let _ = output.expect("nested join inner left should succeed");
    }
}

pub struct NestedJoinInnerRightSpec;
#[jungle::act]
impl Act for NestedJoinInnerRightSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 2)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let _ = output.expect("nested join inner right should succeed");
    }
}

pub struct NestedJoinInnerMergeSpec;
#[jungle::act]
impl Act for NestedJoinInnerMergeSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("nested join inner merge should succeed");
    }
}

pub struct NestedJoinOuterRightSpec;
#[jungle::act]
impl Act for NestedJoinOuterRightSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 3)
    }

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let _ = output.expect("nested join outer right should succeed");
    }
}

pub struct NestedJoinOuterMergeSpec;
#[jungle::act]
impl Act for NestedJoinOuterMergeSpec {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SelectJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("nested join outer merge should succeed");
    }
}

pub struct NestedJoinTailCaptureSpec;
#[jungle::act]
impl Act for NestedJoinTailCaptureSpec {
    type Effect = TimedValueEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SelectJoinState, _input: Self::Input) -> (u64, i32) {
        (0, 1)
    }

    fn absorb(state: &mut SelectJoinState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("nested join tail capture should succeed");
        state.joined_sum = state.joined_sum.saturating_add(value);
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
struct SelectJoinLocalClientAnimals(LocalConditionalJoinTailAnimal);

struct SelectJoinLocalClientZoo;
impl Ecosystem for SelectJoinLocalClientZoo {
    const NAME: &'static str = "select-join-local-client-zoo";
    type Animals = SelectJoinLocalClientAnimals;
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
    let client = jungle_sdk::LocalClient::builder()
        .namespace("select-join-conditional-join-tail")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(SelectJoinLocalClientZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .start_journey::<LocalConditionalJoinTailAnimal>(
            postcard::to_allocvec(&SelectJoinState::default()).expect("seed should serialize"),
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
