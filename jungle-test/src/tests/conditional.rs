use jungle_sdk::prelude::*;
use std::future::ready;

pub struct LeftEffect;

#[jungle::effect(id = 0)]
impl<J> Effect<J> for LeftEffect {
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

pub struct RightEffect;

#[jungle::effect(id = 1)]
impl<J> Effect<J> for RightEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 2))
    }
}

pub struct ConditionalAnimal;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for ConditionalAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = ConditionalFlowTemplate;
}

pub struct ConditionalThenMergeAnimal;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for ConditionalThenMergeAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = ConditionalThenMergeFlowTemplate;
}

pub struct LeftSpec;
#[jungle::action]
impl Action for LeftSpec {
    type Effect = LeftEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<LeftEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let value = output.map_err(|_err| Failure::from("left effect should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_1)
    }
}

pub struct RightSpec;
#[jungle::action]
impl Action for RightSpec {
    type Effect = RightEffect;
    type Input = i32;
    type Output = bool;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state - input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<RightEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            let value = output.map_err(|_err| Failure::from("right effect should succeed"))?;
            *state = value;
            value % 2 == 0
        };
        Ok(__absorb_out_2)
    }
}

type LeftFlow = jungle_sdk::types::Step<LeftSpec>;
type RightFlow = jungle_sdk::types::Step<RightSpec>;

pub struct PreferLeftWhenStateIsNonNegative;
impl jungle_sdk::types::Predicate<(i32, i32)> for PreferLeftWhenStateIsNonNegative {
    fn eval((state, _): &(i32, i32)) -> bool {
        *state >= 0
    }
}

type ConditionalFlow = Conditional<PreferLeftWhenStateIsNonNegative, LeftFlow, RightFlow>;

#[derive(Flow)]
pub struct ConditionalFlowTemplate(ConditionalFlow);

pub struct EchoEffect;

#[jungle::effect(id = 2)]
impl<J> Effect<J> for EchoEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input))
    }
}

pub struct LeftIntSpec;
#[jungle::action]
impl Action for LeftIntSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<EchoEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            let value = output.map_err(|_err| Failure::from("left-int effect should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_3)
    }
}

pub struct RightIntSpec;
#[jungle::action]
impl Action for RightIntSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state - input
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<EchoEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            let value = output.map_err(|_err| Failure::from("right-int effect should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_4)
    }
}

pub struct MergeEitherSpec;
#[jungle::action]
impl Action for MergeEitherSpec {
    type Effect = EchoEffect;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<EchoEffect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
            let value =
                output.map_err(|_err| Failure::from("merge-either effect should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_5)
    }
}

#[derive(Flow)]
pub struct ConditionalThenMergeFlowTemplate(
    Conditional<PreferLeftWhenStateIsNonNegative, Step<LeftIntSpec>, Step<RightIntSpec>>,
    Step<MergeEitherSpec>,
);

type BoundConditionalFlow = Conditional<
    PreferLeftWhenStateIsNonNegative,
    BoundFlowStep<ConditionalAnimal, <LeftSpec as Action>::Bind<ConditionalAnimal>>,
    BoundFlowStep<ConditionalAnimal, <RightSpec as Action>::Bind<ConditionalAnimal>>,
>;

#[test]
fn conditional_run_selects_branch_from_predicate() {
    let left = <BoundConditionalFlow as Running>::run((5, 3));
    match left {
        Either::Left((_state, (request, ()))) => assert_eq!(request.into_input(), 8),
        Either::Right(_) => panic!("expected left branch"),
    }

    let right = <BoundConditionalFlow as Running>::run((-2, 3));
    match right {
        Either::Left(_) => panic!("expected right branch"),
        Either::Right((_state, (request, ()))) => assert_eq!(request.into_input(), -5),
    }
}

#[test]
fn conditional_waiting_accept_returns_either_branch_output() {
    let left = <BoundConditionalFlow as Waiting>::accept(Either::Left((1, Ok(9), ())));
    match left {
        Either::Left((state, emitted)) => {
            assert_eq!(state, 9);
            assert_eq!(emitted, 9);
        }
        Either::Right(_) => panic!("expected left output"),
    }

    let right = <BoundConditionalFlow as Waiting>::accept(Either::Right((1, Ok(6), ())));
    match right {
        Either::Left(_) => panic!("expected right output"),
        Either::Right((state, emitted)) => {
            assert_eq!(state, 6);
            assert!(emitted);
        }
    }
}

#[test]
fn executor_dynamically_selects_conditional_branch() {
    let mut left = ManualExecutor::<ConditionalAnimal>::new(5);
    let left_emitted: Either<i32, bool> = left
        .next_typed((true, 3), Ok::<i32, ()>(9))
        .expect("left branch");
    assert_eq!(left_emitted, Either::Left(9));
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = ManualExecutor::<ConditionalAnimal>::new(-2);
    let right_emitted: Either<i32, bool> = right
        .next_typed((false, 3), Ok::<i32, ()>(6))
        .expect("right branch");
    assert_eq!(right_emitted, Either::Right(true));
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}

#[test]
fn executor_requests_and_completes_conditional_branch() {
    let mut left = Executor::<ConditionalAnimal>::new(5);
    let left_request: i32 = left.next_request().expect("left request");
    assert_eq!(left_request, 5);
    let left_emitted: Either<i32, bool> = left.complete(Ok::<i32, ()>(9)).expect("left completion");
    assert_eq!(left_emitted, Either::Left(9));
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 9);

    let mut right = Executor::<ConditionalAnimal>::new(-2);
    let right_request: i32 = right.next_request().expect("right request");
    assert_eq!(right_request, -2);
    let right_emitted: Either<i32, bool> =
        right.complete(Ok::<i32, ()>(6)).expect("right completion");
    assert_eq!(right_emitted, Either::Right(true));
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 6);
}

#[tokio::test]
async fn executor_executable_request_runs_without_static_effect_dispatch() {
    let mut left = Executor::<ConditionalAnimal>::new(5);
    let request = left
        .next_executable_request((true, 0i32))
        .expect("left executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("left request should deserialize");
    assert_eq!(input, 5);
    let completion = request.run().await.expect("left effect should execute");
    let _left_emitted = left
        .complete_serialized(completion)
        .expect("left completion should process");
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 6);

    let mut right = Executor::<ConditionalAnimal>::new(-2);
    let request = right
        .next_executable_request((false, 0i32))
        .expect("right executable request");
    let input: i32 = request
        .deserialize_request()
        .expect("right request should deserialize");
    assert_eq!(input, -2);
    let completion = request.run().await.expect("right effect should execute");
    let _right_emitted = right
        .complete_serialized(completion)
        .expect("right completion should process");
    assert!(right.is_complete());
    assert_eq!(right.into_state(), 0);
}

#[test]
fn conditional_output_is_routed_as_either_for_follow_up_step() {
    let mut left = Executor::<ConditionalThenMergeAnimal>::new(5);
    let left_request_1: i32 = left.next_request().expect("left request 1");
    assert_eq!(left_request_1, 5);
    let left_emitted_1: Either<i32, i32> =
        left.complete(Ok::<i32, ()>(5)).expect("left completion 1");
    assert_eq!(left_emitted_1, Either::Left(5));
    let left_request_2: i32 = left.next_request().expect("left request 2");
    assert_eq!(left_request_2, 5);
    let left_emitted_2: i32 = left.complete(Ok::<i32, ()>(5)).expect("left completion 2");
    assert_eq!(left_emitted_2, 5);
    assert!(left.is_complete());
    assert_eq!(left.into_state(), 5);

    let mut right = Executor::<ConditionalThenMergeAnimal>::new(-2);
    let right_request_1: i32 = right.next_request().expect("right request 1");
    assert_eq!(right_request_1, -2);
    let right_emitted_1: Either<i32, i32> = right
        .complete(Ok::<i32, ()>(-2))
        .expect("right completion 1");
    assert_eq!(right_emitted_1, Either::Right(-2));
    let right_request_2: i32 = right.next_request().expect("right request 2");
    assert_eq!(right_request_2, -2);
    let right_emitted_2: i32 = right
        .complete(Ok::<i32, ()>(-2))
        .expect("right completion 2");
    assert_eq!(right_emitted_2, -2);
    assert!(right.is_complete());
    assert_eq!(right.into_state(), -2);
}

