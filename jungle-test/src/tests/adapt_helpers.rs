use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HelperState {
    value: i32,
    pulse_count: i32,
}

struct EchoEffect;

#[jungle::effect(id = 70)]
impl<J> Effect<J> for EchoEffect {
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

#[jungle::effect(id = 71)]
impl<J> Effect<J> for PulseEffect {
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
#[jungle::action(bind = Fuse<
        PassthroughEmit<EchoEffect, Identity>,
        AbsorbFn<Identity, EchoEffect, i32, StoreValueAbsorb>,
    >)]
impl Action for PassthroughSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;
}

struct UnitSpec;
#[jungle::action(bind = Fuse<
        UnitEmit<PulseEffect, Identity>,
        AbsorbFn<Identity, PulseEffect, (), CountPulseAbsorb>,
    >)]
impl Action for UnitSpec {
    type Effect = PulseEffect;
    type Input = ();
    type Output = ();
}

pub struct BridgeToUnitEffect;

#[jungle::effect(id = 72)]
impl<J> Effect<J> for BridgeToUnitEffect {
    type In = i32;
    type Out = ();
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

struct BridgeToUnitSpec;
#[jungle::action]
impl Action for BridgeToUnitSpec {
    type Effect = BridgeToUnitEffect;
    type Input = i32;
    type Output = ();

    fn emit(_state: &HelperState, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(_state: &mut HelperState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("bridge-to-unit should succeed");
    }
}

pub struct BridgeFromUnitEffect;

#[jungle::effect(id = 73)]
impl<J> Effect<J> for BridgeFromUnitEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(2))
    }
}

struct BridgeFromUnitSpec;
#[jungle::action]
impl Action for BridgeFromUnitSpec {
    type Effect = BridgeFromUnitEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &HelperState, input: Self::Input) -> Self::Input {
        input
    }

    fn absorb(state: &mut HelperState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("bridge-from-unit should succeed");
        state.value += value;
        value
    }
}

struct FunctionEmitSpec;
#[jungle::action(bind = Fuse<
        EmitFn<Identity, EchoEffect, i32, EmitUsingState>,
        AbsorbFn<Identity, EchoEffect, i32, StoreValueAbsorb>,
    >)]
impl Action for FunctionEmitSpec {
    type Effect = EchoEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
struct AdaptHelpersFlowTemplate(
    Step<PassthroughSpec>,
    Step<BridgeToUnitSpec>,
    Step<UnitSpec>,
    Step<BridgeFromUnitSpec>,
    Step<FunctionEmitSpec>,
);

struct HelperAnimal;

#[jungle::animal(id = 0, generation = 0)]
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

    let step1: () = executor
        .next_typed(step0, Ok::<(), ()>(()))
        .expect("step 1");
    assert_eq!(step1, ());

    let step2: () = executor
        .next_typed(step1, Ok::<i32, ()>(5))
        .expect("step 2");
    assert_eq!(step2, ());
    assert_eq!(executor.state().pulse_count, 1);
    assert_eq!(executor.state().value, 10);

    let step3: i32 = executor
        .next_typed(step2, Ok::<i32, ()>(2))
        .expect("step 3");
    assert_eq!(step3, 2);

    let step4: i32 = executor
        .next_typed(step3, Ok::<i32, ()>(13))
        .expect("step 4");
    assert_eq!(step4, 13);
    assert_eq!(executor.state().value, 13);
    assert!(executor.is_complete());
}
