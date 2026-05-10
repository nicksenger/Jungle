use jungle_sdk::types::{
    AbsorbFn, AbsorbMapper, ActionCompletion, EmitFn, EmitMapper, FocusedStep, Identity,
    IdentityStep, Adapt, ManualExecutor, PassthroughEmit, Step, UnitEmit,
};
use jungle_sdk::typosaurus::num::consts::{U0, U70, U71};
use jungle_sdk::Journey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct HelperState {
    value: i32,
    pulse_count: i32,
}

action!(EchoAction, U70, in = i32, out = i32, err = (), act = |_d, input| std::future::ready(Ok(input + 1)));
action!(PulseAction, U71, in = (), out = i32, err = (), act = |_d, _input| std::future::ready(Ok(5)));

struct StoreValueAbsorb;
impl AbsorbMapper<HelperState, EchoAction, i32> for StoreValueAbsorb {
    fn absorb(state: &mut HelperState, output: ActionCompletion<EchoAction>) -> i32 {
        let value = output.expect("echo should succeed");
        state.value = value;
        value
    }
}

struct CountPulseAbsorb;
impl AbsorbMapper<HelperState, PulseAction, ()> for CountPulseAbsorb {
    fn absorb(state: &mut HelperState, output: ActionCompletion<PulseAction>) {
        let value = output.expect("pulse should succeed");
        state.pulse_count += 1;
        state.value += value;
    }
}

struct EmitUsingState;
impl EmitMapper<HelperState, EchoAction, i32> for EmitUsingState {
    fn emit(state: &HelperState, input: i32) -> i32 {
        state.value + input
    }
}

type PassthroughStep = FocusedStep<
    HelperAnimal,
    Identity,
    PassthroughEmit<EchoAction, Identity>,
    AbsorbFn<Identity, EchoAction, i32, StoreValueAbsorb>,
>;

type UnitStep = IdentityStep<
    HelperAnimal,
    UnitEmit<PulseAction, Identity>,
    AbsorbFn<Identity, PulseAction, (), CountPulseAbsorb>,
>;

type FunctionEmitStep = Step<
    HelperAnimal,
    Adapt<
        EmitFn<Identity, EchoAction, i32, EmitUsingState>,
        AbsorbFn<Identity, EchoAction, i32, StoreValueAbsorb>,
    >,
>;

#[derive(Journey)]
struct AdaptHelpersJourney(PassthroughStep, UnitStep, FunctionEmitStep);

animal!(
    HelperAnimal,
    U0,
    state = HelperState,
    journey = AdaptHelpersJourney
);

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
