use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoopState {
    count: i32,
    seen: i32,
}

pub struct EchoI32Effect;
#[jungle::effect(id = 74)]
impl<J> Effect<J> for EchoI32Effect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct NoopIncrementSpec;
#[jungle::action]
impl Action for NoopIncrementSpec {
    type Effect = Noop;
    type Input = ();
    type Output = i32;

    fn emit(_state: &NoopState, _input: Self::Input) -> () {}

    fn absorb(state: &mut NoopState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("noop effect should succeed");
        state.count += 1;
        state.count
    }
}

pub struct CaptureValueSpec;
#[jungle::action]
impl Action for CaptureValueSpec {
    type Effect = EchoI32Effect;
    type Input = i32;
    type Output = ();

    fn emit(_state: &NoopState, input: Self::Input) -> i32 {
        input
    }

    fn absorb(state: &mut NoopState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.seen = output.expect("echo effect should succeed");
    }
}

#[derive(Flow)]
pub struct NoopFlowTemplate(Step<NoopIncrementSpec>, Step<CaptureValueSpec>);

pub struct NoopAnimal;

#[jungle::animal(id = 5, generation = 0)]
impl Animal for NoopAnimal {
    type State = NoopState;
    type Seed = NoopState;
    type Journey = NoopFlowTemplate;
}

#[tokio::test]
async fn noop_step_is_completed_inline_before_next_effect_request() {
    let mut executor = Executor::<NoopAnimal>::new(NoopState::default());

    let request = executor
        .next_executable_request(())
        .expect("request after noop should be available");

    assert_eq!(
        request.effect_type(),
        core::any::type_name::<EchoI32Effect>()
    );
    let request_value: i32 = request
        .deserialize_request()
        .expect("echo request should deserialize as i32");
    assert_eq!(request_value, 1);
    assert_eq!(executor.state().count, 1);

    let completion = request.run().await.expect("echo execution should succeed");
    let _ = executor
        .complete_serialized(completion)
        .expect("echo completion should advance executor");
    assert_eq!(executor.state().seen, 1);
}
