use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoEffectState {
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

pub struct NoEffectIncrementSpec;
#[jungle::action]
impl Action for NoEffectIncrementSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &NoEffectState, _input: Self::Input) -> () {}

    fn absorb(
        state: &mut NoEffectState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            output.map_err(|_err| Failure::from("no-effect should succeed"))?;
            state.count += 1;
            state.count
        };
        Ok(__absorb_out_1)
    }
}

pub struct CaptureValueSpec;
#[jungle::action]
impl Action for CaptureValueSpec {
    type Effect = EchoI32Effect;
    type Input = i32;
    type Output = ();

    fn emit(_state: &NoEffectState, input: Self::Input) -> i32 {
        input
    }

    fn absorb(
        state: &mut NoEffectState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            state.seen = output.map_err(|_err| Failure::from("echo effect should succeed"))?;
        };
        Ok(__absorb_out_2)
    }
}

#[derive(Flow)]
pub struct NoEffectFlowTemplate(Step<NoEffectIncrementSpec>, Step<CaptureValueSpec>);

pub struct NoEffectAnimal;

#[jungle::animal(id = 5, generation = 0)]
impl Animal for NoEffectAnimal {
    type State = NoEffectState;
    type Seed = NoEffectState;
    type Flow = NoEffectFlowTemplate;
}

#[tokio::test]
async fn no_effect_step_is_completed_inline_before_next_effect_request() {
    let mut executor = Executor::<NoEffectAnimal>::new(NoEffectState::default());

    let request = executor
        .next_executable_request(())
        .expect("request after no-effect should be available");

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

