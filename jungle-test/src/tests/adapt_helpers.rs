use jungle_sdk::animal;
use jungle_sdk::effect;
use jungle_sdk::types::Animal;
use jungle_sdk::types::{
    AbsorbFn, AbsorbMapper, Act, EffectCompletion, EmitFn, EmitMapper, Fuse, Identity,
    ManualExecutor, PassthroughEmit, Step, UnitEmit,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HelperState {
    value: i32,
    pulse_count: i32,
}

struct EchoEffect;

#[effect(id = 70)]
impl<J> jungle_sdk::types::Effect<J> for EchoEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}
struct PulseEffect;

#[effect(id = 71)]
impl<J> jungle_sdk::types::Effect<J> for PulseEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(5))
    }
}

struct StoreValueAbsorb;
impl AbsorbMapper<HelperState, EchoEffect, i32> for StoreValueAbsorb {
    fn absorb(state: &mut HelperState, output: EffectCompletion<EchoEffect>) -> i32 {
        let value = output.expect("echo should succeed");
        state.value = value;
        value
    }
}

struct CountPulseAbsorb;
impl AbsorbMapper<HelperState, PulseEffect, ()> for CountPulseAbsorb {
    fn absorb(state: &mut HelperState, output: EffectCompletion<PulseEffect>) {
        let value = output.expect("pulse should succeed");
        state.pulse_count += 1;
        state.value += value;
    }
}

struct EmitUsingState;
impl EmitMapper<HelperState, EchoEffect, i32> for EmitUsingState {
    fn emit(state: &HelperState, input: i32) -> i32 {
        state.value + input
    }
}

struct PassthroughSpec;
#[jungle_sdk::act(bind = Fuse<
        PassthroughEmit<EchoEffect, Identity>,
        AbsorbFn<Identity, EchoEffect, i32, StoreValueAbsorb>,
    >)]
impl Act for PassthroughSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;
}

struct UnitSpec;
#[jungle_sdk::act(bind = Fuse<
        UnitEmit<PulseEffect, Identity>,
        AbsorbFn<Identity, PulseEffect, (), CountPulseAbsorb>,
    >)]
impl Act for UnitSpec {
    type Effect = PulseEffect;
    type Input = ();
    type Output = ();
}

struct FunctionEmitSpec;
#[jungle_sdk::act(bind = Fuse<
        EmitFn<Identity, EchoEffect, i32, EmitUsingState>,
        AbsorbFn<Identity, EchoEffect, i32, StoreValueAbsorb>,
    >)]
impl Act for FunctionEmitSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(jungle_sdk::Flow)]
struct AdaptHelpersFlowTemplate(Step<PassthroughSpec>, Step<UnitSpec>, Step<FunctionEmitSpec>);

struct HelperAnimal;

#[animal(id = 0, generation = 0)]
impl Animal for HelperAnimal {
    type State = HelperState;
    type Seed = HelperState;
    type Journey = AdaptHelpersFlowTemplate;
}

#[test]
fn helper_emit_absorb_adapters_work_in_flow() {
    let mut executor = ManualExecutor::<HelperAnimal>::new(HelperState {
        value: 3,
        pulse_count: 0,
    });

    let step0: i32 = executor.next_typed(4, Ok::<i32, ()>(5)).expect("step 0");
    assert_eq!(step0, 5);

    let step1: () = executor.next_typed((), Ok::<i32, ()>(5)).expect("step 1");
    assert_eq!(step1, ());
    assert_eq!(executor.state().pulse_count, 1);
    assert_eq!(executor.state().value, 10);

    let step2: i32 = executor.next_typed(2, Ok::<i32, ()>(13)).expect("step 2");
    assert_eq!(step2, 13);
    assert_eq!(executor.state().value, 13);
    assert!(executor.is_complete());
}
