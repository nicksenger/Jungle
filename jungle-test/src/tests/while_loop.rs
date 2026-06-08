use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::future::ready;
use std::sync::Arc;

pub struct TickEffect;

#[jungle::effect(id = 0)]
impl<J> Effect<J> for TickEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}

pub struct Looper;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for Looper {
    type State = i32;
    type Seed = i32;
    type Flow = LoopFlowTemplate;
}

pub struct TickSpec;
#[jungle::action]
impl Action for TickSpec {
    type Effect = TickEffect;
    type Input = i32;
    type Output = (bool, i32);

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<TickEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let value = output.map_err(|_err| Failure::from("tick effect should succeed"))?;
            *state = value;
            (*state < 3, value)
        };
        Ok(__absorb_out_1)
    }
}

type TickFlow = BoundFlowStep<Looper, <TickSpec as Action>::Bind<Looper>>;

pub struct LessThanThree;
impl Predicate<(&i32, &i32)> for LessThanThree {
    fn eval((state, _): &(&i32, &i32)) -> bool {
        **state < 3
    }
}
type WhileTickFlow = While<LessThanThree, TickFlow>;

#[derive(Flow)]
pub struct LoopFlowTemplate(While<LessThanThree, Step<TickSpec>>);

pub struct TailEchoEffect;

#[jungle::effect(id = 1)]
impl<J> Effect<J> for TailEchoEffect {
    type In = (bool, i32);
    type Out = (bool, i32);
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input))
    }
}

pub struct LooperWithTail;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for LooperWithTail {
    type State = i32;
    type Seed = i32;
    type Flow = LoopWithTailFlowTemplate;
}

pub struct TailAfterLoopSpec;
#[jungle::action]
impl Action for TailAfterLoopSpec {
    type Effect = TailEchoEffect;
    type Input = (bool, i32);
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<TailEchoEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            let (loop_should_continue, value) =
                output.map_err(|_err| Failure::from("tail effect should succeed"))?;
            *state = if loop_should_continue {
                -999
            } else {
                value + 10
            };
            *state
        };
        Ok(__absorb_out_2)
    }
}

#[derive(Flow)]
pub struct LoopWithTailFlowTemplate(
    While<LessThanThree, Step<TickSpec>>,
    Step<TailAfterLoopSpec>,
);

pub struct UnitEffect;

#[jungle::effect(id = 2)]
impl<J> Effect<J> for UnitEffect {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _dependency: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(()))
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestedState {
    outer_round: u8,
    inner_step: u8,
    outer_iterations_done: u8,
}

pub struct NestedLooper;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for NestedLooper {
    type State = NestedState;
    type Seed = NestedState;
    type Flow = NestedLoopFlowTemplate;
}

pub struct InnerContinue;
impl Predicate<(&NestedState, &())> for InnerContinue {
    fn eval((state, _): &(&NestedState, &())) -> bool {
        state.inner_step < 2
    }
}

pub struct OuterContinue;
impl Predicate<(&NestedState, &())> for OuterContinue {
    fn eval((state, _): &(&NestedState, &())) -> bool {
        state.outer_round < 3
    }
}

pub struct InnerWorkSpec;
#[jungle::action]
impl Action for InnerWorkSpec {
    type Effect = UnitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &NestedState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut NestedState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            state.inner_step = state.inner_step.saturating_add(1);
        };
        Ok(__absorb_out_3)
    }
}

pub struct FinishOuterRoundSpec;
#[jungle::action]
impl Action for FinishOuterRoundSpec {
    type Effect = UnitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &NestedState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut NestedState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            state.outer_iterations_done = state.outer_iterations_done.saturating_add(1);
            state.outer_round = state.outer_round.saturating_add(1);
            state.inner_step = 0;
        };
        Ok(__absorb_out_4)
    }
}

pub struct EchoBoolEffect;

#[jungle::effect(id = 91)]
impl<J> Effect<J> for EchoBoolEffect {
    type In = bool;
    type Out = bool;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input))
    }
}

pub struct InlineNoEffectFalseSpec;
#[jungle::action]
impl Action for InlineNoEffectFalseSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = bool;

    fn emit(_state: &u8, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut u8,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
            output.map_err(|_err| Failure::from("no-effect should succeed"))?;
            false
        };
        Ok(__absorb_out_5)
    }
}

pub struct EchoBoolSpec;
#[jungle::action]
impl Action for EchoBoolSpec {
    type Effect = EchoBoolEffect;
    type Input = bool;
    type Output = ();

    fn emit(_state: &u8, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(
        state: &mut u8,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_6 = {
            let echoed = output.map_err(|_err| Failure::from("echo bool should succeed"))?;
            assert!(!echoed, "expected inline no-effect output to feed false");
            *state = state.saturating_add(1);
        };
        Ok(__absorb_out_6)
    }
}

pub struct RunOnce;
impl Predicate<(&u8, &())> for RunOnce {
    fn eval((state, _): &(&u8, &())) -> bool {
        **state == 0
    }
}

#[derive(Flow)]
pub struct WhileInlineNoEffectThenEffectFlow(While<RunOnce, WhileInlineNoEffectThenEffectBody>);

#[derive(Flow)]
pub struct WhileInlineNoEffectThenEffectBody(Step<InlineNoEffectFalseSpec>, Step<EchoBoolSpec>);

pub struct WhileInlineNoEffectThenEffectAnimal;

#[jungle::animal(id = 91, generation = 0)]
impl Animal for WhileInlineNoEffectThenEffectAnimal {
    type State = u8;
    type Seed = u8;
    type Flow = WhileInlineNoEffectThenEffectFlow;
}

pub struct InnerRunOnce;
impl Predicate<(&u8, &())> for InnerRunOnce {
    fn eval((state, _): &(&u8, &())) -> bool {
        **state == 0
    }
}

pub struct OuterRunOnce;
impl Predicate<(&u8, &())> for OuterRunOnce {
    fn eval((state, _): &(&u8, &())) -> bool {
        **state == 0
    }
}

#[derive(Flow)]
pub struct NestedWhileInlineNoEffectThenEffectFlow(While<OuterRunOnce, NestedWhileOuterBody>);

#[derive(Flow)]
pub struct NestedWhileOuterBody(While<InnerRunOnce, WhileInlineNoEffectThenEffectBody>);

pub struct NestedWhileInlineNoEffectThenEffectAnimal;

#[jungle::animal(id = 92, generation = 0)]
impl Animal for NestedWhileInlineNoEffectThenEffectAnimal {
    type State = u8;
    type Seed = u8;
    type Flow = NestedWhileInlineNoEffectThenEffectFlow;
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestedInlineCarryState {
    inner_done: bool,
    outer_done: bool,
}

pub struct NestedInlineInnerRunOnce;
impl Predicate<(&NestedInlineCarryState, &())> for NestedInlineInnerRunOnce {
    fn eval((state, _): &(&NestedInlineCarryState, &())) -> bool {
        !state.inner_done
    }
}

pub struct NestedInlineOuterRunOnce;
impl Predicate<(&NestedInlineCarryState, &())> for NestedInlineOuterRunOnce {
    fn eval((state, _): &(&NestedInlineCarryState, &())) -> bool {
        !state.outer_done
    }
}

pub struct NestedInlineInnerNoEffectFalseSpec;
#[jungle::action]
impl Action for NestedInlineInnerNoEffectFalseSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = bool;

    fn emit(_state: &NestedInlineCarryState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut NestedInlineCarryState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_7 = {
            output.map_err(|_err| Failure::from("nested inner no-effect should succeed"))?;
            state.inner_done = true;
            false
        };
        Ok(__absorb_out_7)
    }
}

pub struct NestedInlineOuterEchoBoolSpec;
#[jungle::action]
impl Action for NestedInlineOuterEchoBoolSpec {
    type Effect = EchoBoolEffect;
    type Input = bool;
    type Output = ();

    fn emit(_state: &NestedInlineCarryState, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(
        state: &mut NestedInlineCarryState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_8 = {
            let echoed =
                output.map_err(|_err| Failure::from("nested outer echo bool should succeed"))?;
            assert!(!echoed, "nested outer step should receive inline false");
            state.outer_done = true;
        };
        Ok(__absorb_out_8)
    }
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RhythmLikeLoopState {
    loops_remaining: u8,
    choose_final_tail: bool,
    #[jungle(focus)]
    focused: i32,
}

pub struct RhythmLikeLoopRemaining;
impl Predicate<(&RhythmLikeLoopState, &())> for RhythmLikeLoopRemaining {
    fn eval((state, _): &(&RhythmLikeLoopState, &())) -> bool {
        state.loops_remaining > 0
    }
}

pub struct UseRhythmLikeFinalTail;
impl Predicate<(RhythmLikeLoopState, ())> for UseRhythmLikeFinalTail {
    fn eval((state, _): &(RhythmLikeLoopState, ())) -> bool {
        state.choose_final_tail
    }
}

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct RhythmLikeJoinLeftSpec;
#[jungle::action]
impl Action for RhythmLikeJoinLeftSpec {
    type Effect = TickEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        _state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_9 = {
            let _ = output.map_err(|_err| Failure::from("rhythm-like join left should succeed"))?;
        };
        Ok(__absorb_out_9)
    }
}

pub struct RhythmLikeJoinRightSpec;
#[jungle::action]
impl Action for RhythmLikeJoinRightSpec {
    type Effect = TickEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> i32 {
        1
    }

    fn absorb(
        _state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_10 = {
            let _ =
                output.map_err(|_err| Failure::from("rhythm-like join right should succeed"))?;
        };
        Ok(__absorb_out_10)
    }
}

pub struct RhythmLikeMergeUnitSpec;
#[jungle::action]
impl Action for RhythmLikeMergeUnitSpec {
    type Effect = NoEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_11 = {
            output.map_err(|_err| Failure::from("rhythm-like merge unit should succeed"))?;
        };
        Ok(__absorb_out_11)
    }
}

pub struct RhythmLikePostMergeRestSpec;
#[jungle::action]
impl Action for RhythmLikePostMergeRestSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> std::time::Duration {
        std::time::Duration::from_millis(1)
    }

    fn absorb(
        _state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_12 = {
            output.map_err(|_err| Failure::from("rhythm-like post merge rest should succeed"))?;
        };
        Ok(__absorb_out_12)
    }
}

pub struct RhythmLikeDecrementLoopSpec;
#[jungle::action]
impl Action for RhythmLikeDecrementLoopSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmLikeLoopState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut RhythmLikeLoopState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_13 = {
            output.map_err(|_err| Failure::from("rhythm-like decrement should succeed"))?;
            state.loops_remaining = state.loops_remaining.saturating_sub(1);
        };
        Ok(__absorb_out_13)
    }
}

pub struct RhythmLikeMergeChoiceSpec;
#[jungle::action]
impl Action for RhythmLikeMergeChoiceSpec {
    type Effect = NoEffect;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &RhythmLikeLoopState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut RhythmLikeLoopState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_14 = {
            output.map_err(|_err| Failure::from("rhythm-like conditional merge should succeed"))?;
        };
        Ok(__absorb_out_14)
    }
}

#[derive(Flow)]
#[jungle(focus = i32)]
pub struct RhythmLikeFocusedTurnaround(
    Join<Step<RhythmLikeJoinLeftSpec>, Step<RhythmLikeJoinRightSpec>>,
    Step<RhythmLikeMergeUnitSpec>,
    Step<RhythmLikePostMergeRestSpec>,
);

#[derive(Flow)]
pub struct RhythmLikeNormalTail(Step<RhythmLikeDecrementLoopSpec>);

#[derive(Flow)]
pub struct RhythmLikeFinalTail(
    Transparent<IntroSectionMeta, RhythmLikeFocusedTurnaround>,
    Step<RhythmLikeDecrementLoopSpec>,
);

#[derive(Flow)]
pub struct RhythmLikeConditionalLoopBody(
    Conditional<UseRhythmLikeFinalTail, RhythmLikeFinalTail, RhythmLikeNormalTail>,
    Step<RhythmLikeMergeChoiceSpec>,
);

#[derive(Flow)]
pub struct RhythmLikeConditionalLoopFlow(
    While<RhythmLikeLoopRemaining, RhythmLikeConditionalLoopBody>,
);

pub struct RhythmLikeConditionalLoopAnimal;

#[jungle::animal(id = 94, generation = 0)]
impl Animal for RhythmLikeConditionalLoopAnimal {
    type State = RhythmLikeLoopState;
    type Seed = RhythmLikeLoopState;
    type Flow = RhythmLikeConditionalLoopFlow;
}

#[derive(Flow)]
pub struct NestedInlineInnerOnlyBody(Step<NestedInlineInnerNoEffectFalseSpec>);

#[derive(Flow)]
pub struct NestedInlineOuterBody(
    While<NestedInlineInnerRunOnce, NestedInlineInnerOnlyBody>,
    Step<NestedInlineOuterEchoBoolSpec>,
);

#[derive(Flow)]
pub struct NestedInlineWhileCarryFlow(While<NestedInlineOuterRunOnce, NestedInlineOuterBody>);

pub struct NestedInlineWhileCarryAnimal;

#[jungle::animal(id = 93, generation = 0)]
impl Animal for NestedInlineWhileCarryAnimal {
    type State = NestedInlineCarryState;
    type Seed = NestedInlineCarryState;
    type Flow = NestedInlineWhileCarryFlow;
}

#[derive(Flow)]
pub struct NestedOuterBodyTemplate(
    While<InnerContinue, Step<InnerWorkSpec>>,
    Step<FinishOuterRoundSpec>,
);

#[derive(Flow)]
pub struct NestedLoopFlowTemplate(While<OuterContinue, NestedOuterBodyTemplate>);

#[derive(Flow)]
pub struct ExampleFlow(While<OuterContinue, ExampleWhileBody>);

#[derive(Flow)]
pub struct ExampleWhileBody(
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
    AnotherExampleFlow,
);

#[derive(Flow)]
pub struct AnotherExampleFlow(
    While<InnerContinue, Step<InnerWorkSpec>>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
    Step<FinishOuterRoundSpec>,
);

#[test]
fn example_flow_for_dumb_robot() {}

#[test]
fn while_running_checks_state_before_iteration() {
    let run = <WhileTickFlow as Running>::run((true, (0, 1)));
    match run {
        Some((_state, (request, ()))) => assert_eq!(request.into_input(), 1),
        None => panic!("expected iteration to run"),
    }

    let run = <WhileTickFlow as Running>::run((false, (3, 1)));
    assert!(run.is_none());
}

#[test]
fn while_waiting_passthroughs_optional_branch() {
    let waited = <WhileTickFlow as Waiting>::accept(Some((0, Ok(2), ())));
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
    let emitted = vec![
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(1))
            .expect("first tick should advance"),
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(2))
            .expect("second tick should advance"),
        loop_executor
            .next_typed::<_, i32, (), (bool, i32)>(1, Ok(3))
            .expect("third tick should advance"),
    ];
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

#[test]
fn while_executable_inline_no_effect_then_effect_keeps_request_completion_handshake() {
    let mut executor = Executor::<WhileInlineNoEffectThenEffectAnimal>::new(0);
    let request = executor
        .next_executable_request(())
        .expect("while body should advance from inline no-effect to effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("request input should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("completing requested effect should not fail with no pending request");
}

#[test]
fn while_context_executable_inline_no_effect_then_effect_keeps_request_completion_handshake() {
    let mut executor =
        ContextExecutor::<(), WhileInlineNoEffectThenEffectAnimal>::new(Arc::new(()), 0);
    let request = executor
        .next_executable_request(())
        .expect("context while body should advance from inline no-effect to effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("request input should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("completing requested effect should not fail with no pending request");
}

#[test]
fn nested_while_executable_inline_no_effect_then_effect_keeps_request_completion_handshake() {
    let mut executor = Executor::<NestedWhileInlineNoEffectThenEffectAnimal>::new(0);
    let request = executor
        .next_executable_request(())
        .expect("nested while body should advance from inline no-effect to effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("request input should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("completing nested while requested effect should not fail with no pending request");
}

#[test]
fn nested_while_context_executable_inline_no_effect_then_effect_keeps_request_completion_handshake()
{
    let mut executor =
        ContextExecutor::<(), NestedWhileInlineNoEffectThenEffectAnimal>::new(Arc::new(()), 0);
    let request = executor
        .next_executable_request(())
        .expect("nested context while body should advance from inline no-effect to effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("request input should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("completing nested context while requested effect should not fail with no pending request");
}

#[test]
fn nested_inline_while_executable_propagates_inner_emitted_to_outer_sibling() {
    let mut executor =
        Executor::<NestedInlineWhileCarryAnimal>::new(NestedInlineCarryState::default());
    let request = executor
        .next_executable_request(())
        .expect("nested inline while should still produce outer sibling effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("nested outer sibling request should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("nested inline while completion should not fail with no pending request");
}

#[test]
fn nested_inline_while_context_executable_propagates_inner_emitted_to_outer_sibling() {
    let mut executor = ContextExecutor::<(), NestedInlineWhileCarryAnimal>::new(
        Arc::new(()),
        NestedInlineCarryState::default(),
    );
    let request = executor
        .next_executable_request(())
        .expect("nested context inline while should still produce outer sibling effect request");
    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoBoolEffect>()
    );
    let request_input: bool = request
        .deserialize_request()
        .expect("nested context outer sibling request should deserialize as bool");
    assert!(!request_input);

    let completion = futures::executor::block_on(request.run());
    let _emitted = executor
        .complete_serialized(completion.expect("effect should run"))
        .expect("nested context inline while completion should not fail with no pending request");
}

#[tokio::test]
async fn while_conditional_final_tail_with_focused_join_merge_rest_does_not_hang() {
    let mut executor = ContextExecutor::<(), RhythmLikeConditionalLoopAnimal>::new(
        Arc::new(()),
        RhythmLikeLoopState {
            loops_remaining: 1,
            choose_final_tail: true,
            focused: 0,
        },
    );

    let first = executor
        .next_executable_request(())
        .expect("while/conditional final-tail flow should produce an executable request");
    let completion = first.run().await.expect("first effect should run");
    let _ = executor
        .complete_serialized(completion)
        .expect("completion should advance while/conditional final-tail flow");

    let _ = executor
        .advance_to_end_with(())
        .await
        .expect("while/conditional final-tail flow should complete");
    assert!(executor.is_complete());
    assert_eq!(executor.state().loops_remaining, 0);
}

