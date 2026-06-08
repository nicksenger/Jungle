use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::future::ready;

pub struct EmptyVecStartEffect;

#[jungle::effect(id = 200)]
impl<J> Effect<J> for EmptyVecStartEffect {
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

pub struct EmptyVecTailEffect;

#[jungle::effect(id = 201)]
impl<J> Effect<J> for EmptyVecTailEffect {
    type In = Vec<u8>;
    type Out = usize;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input.len()))
    }
}

pub struct PromptStartEffect;

#[jungle::effect(id = 202)]
impl<J> Effect<J> for PromptStartEffect {
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

pub struct PromptToEmptyVecEffect;

#[jungle::effect(id = 203)]
impl<J> Effect<J> for PromptToEmptyVecEffect {
    type In = PromptLike;
    type Out = Vec<u8>;
    type Err = ();

    fn effect(
        _dependency: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(Vec::new()))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptLike {
    token: u8,
}

pub struct EmptyVecRegressionAnimal;

#[jungle::animal(id = 200, generation = 0)]
impl Animal for EmptyVecRegressionAnimal {
    type State = bool;
    type Seed = bool;
    type Flow = EmptyVecRegressionFlow;
}

pub struct ChainedEmptyVecRegressionAnimal;

#[jungle::animal(id = 201, generation = 0)]
impl Animal for ChainedEmptyVecRegressionAnimal {
    type State = bool;
    type Seed = bool;
    type Flow = ChainedEmptyVecRegressionFlow;
}

pub struct LoopedChainedEmptyVecRegressionAnimal;

#[jungle::animal(id = 202, generation = 0)]
impl Animal for LoopedChainedEmptyVecRegressionAnimal {
    type State = bool;
    type Seed = bool;
    type Flow = LoopedChainedEmptyVecRegressionFlow;
}

pub struct EmitEmptyVec;
#[jungle::action]
impl Action for EmitEmptyVec {
    type Effect = EmptyVecStartEffect;
    type Input = ();
    type Output = Vec<u8>;

    fn emit(_state: &bool, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut bool,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(Vec::new())
    }
}

pub struct ConsumeEmptyVec;
#[jungle::action]
impl Action for ConsumeEmptyVec {
    type Effect = EmptyVecTailEffect;
    type Input = Vec<u8>;
    type Output = usize;

    fn emit(_state: &bool, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(
        state: &mut bool,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let len = output.map_err(|_err| Failure::from("empty vec tail effect should succeed"))?;
        *state = true;
        Ok(len)
    }
}

pub struct KeepLoopingUntilConsumed;
impl Predicate<(&bool, &())> for KeepLoopingUntilConsumed {
    fn eval((state, _): &(&bool, &())) -> bool {
        !*state
    }
}

pub struct EmitPromptLike;
#[jungle::action]
impl Action for EmitPromptLike {
    type Effect = PromptStartEffect;
    type Input = ();
    type Output = PromptLike;

    fn emit(_state: &bool, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut bool,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(PromptLike { token: 1 })
    }
}

pub struct EmitEmptyVecFromPrompt;
#[jungle::action]
impl Action for EmitEmptyVecFromPrompt {
    type Effect = PromptToEmptyVecEffect;
    type Input = PromptLike;
    type Output = Vec<u8>;

    fn emit(_state: &bool, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(
        _state: &mut bool,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("prompt to empty vec effect should succeed"))
    }
}

#[derive(Flow)]
pub struct EmptyVecRegressionFlow(Step<EmitEmptyVec>, Step<ConsumeEmptyVec>);

#[derive(Flow)]
pub struct ChainedEmptyVecRegressionFlow(
    Step<EmitPromptLike>,
    Step<EmitEmptyVecFromPrompt>,
    Step<ConsumeEmptyVec>,
);

#[derive(Flow)]
pub struct LoopedChainedEmptyVecRegressionFlow(
    While<KeepLoopingUntilConsumed, ChainedEmptyVecRegressionFlow>,
);

#[test]
fn executor_threads_empty_vec_output_into_trailing_step() {
    let mut executor = Executor::<EmptyVecRegressionAnimal>::new(false);

    let first_request: () = executor.next_request().expect("first request");
    assert_eq!(first_request, ());
    let emitted: Vec<u8> = executor
        .complete(Ok::<(), ()>(()))
        .expect("first completion");
    assert!(emitted.is_empty());

    let second_request: Vec<u8> = executor
        .next_request()
        .expect("empty emitted vec should still reach the trailing step");
    assert!(second_request.is_empty());

    let tail_emitted: usize = executor
        .complete(Ok::<usize, ()>(0))
        .expect("tail completion");
    assert_eq!(tail_emitted, 0);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), true);
}

#[test]
fn manual_executor_threads_empty_vec_output_into_trailing_step() {
    let mut executor = ManualExecutor::<EmptyVecRegressionAnimal>::new(false);

    let first_request: () = executor
        .next_request_typed::<_, ()>(())
        .expect("first request");
    assert_eq!(first_request, ());
    let emitted: Vec<u8> = executor
        .complete_typed::<(), (), Vec<u8>>(Ok(()))
        .expect("first completion");
    assert!(emitted.is_empty());

    let second_request: Vec<u8> = executor
        .next_request_typed::<_, Vec<u8>>(())
        .expect("empty emitted vec should still reach the trailing step");
    assert!(second_request.is_empty());

    let tail_emitted: usize = executor
        .complete_typed::<usize, (), usize>(Ok(0))
        .expect("tail completion");
    assert_eq!(tail_emitted, 0);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), true);
}

#[test]
fn manual_executor_threads_empty_vec_after_nonempty_emitted_output() {
    let mut executor = ManualExecutor::<ChainedEmptyVecRegressionAnimal>::new(false);

    let first_request: () = executor
        .next_request_typed::<_, ()>(())
        .expect("first request");
    assert_eq!(first_request, ());
    let emitted_prompt: PromptLike = executor
        .complete_typed::<(), (), PromptLike>(Ok(()))
        .expect("first completion");
    assert_eq!(emitted_prompt, PromptLike { token: 1 });

    let second_request: PromptLike = executor
        .next_request_typed::<_, PromptLike>(())
        .expect("second request");
    assert_eq!(second_request, emitted_prompt);
    let emitted_empty_vec: Vec<u8> = executor
        .complete_typed::<Vec<u8>, (), Vec<u8>>(Ok(Vec::new()))
        .expect("second completion");
    assert!(emitted_empty_vec.is_empty());

    let third_request: Vec<u8> = executor
        .next_request_typed::<_, Vec<u8>>(())
        .expect("empty emitted vec should still reach the trailing step");
    assert!(third_request.is_empty());
}

#[test]
fn manual_executor_threads_empty_vec_inside_while_body() {
    let mut executor = ManualExecutor::<LoopedChainedEmptyVecRegressionAnimal>::new(false);

    let first_request: () = executor
        .next_request_typed::<_, ()>(())
        .expect("first request");
    assert_eq!(first_request, ());
    let emitted_prompt: PromptLike = executor
        .complete_typed::<(), (), PromptLike>(Ok(()))
        .expect("first completion");
    assert_eq!(emitted_prompt, PromptLike { token: 1 });

    let second_request: PromptLike = executor
        .next_request_typed::<_, PromptLike>(())
        .expect("second request");
    assert_eq!(second_request, emitted_prompt);
    let emitted_empty_vec: Vec<u8> = executor
        .complete_typed::<Vec<u8>, (), Vec<u8>>(Ok(Vec::new()))
        .expect("second completion");
    assert!(emitted_empty_vec.is_empty());

    let third_request: Vec<u8> = executor
        .next_request_typed::<_, Vec<u8>>(())
        .expect("empty emitted vec should still reach the trailing step inside the while body");
    assert!(third_request.is_empty());
}

