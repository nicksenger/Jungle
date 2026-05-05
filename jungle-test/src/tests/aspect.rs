use inception::Inception;
use jungle_types::{
    ActionCompletion, ActionStep, Aspect, AspectStep, Awaiting, ErasedStep, JungleFlowActions,
    TestExecutor, TestFlow, TypedErasedStep, Yielding,
};
use serde_json::json;
use std::marker::PhantomData;
use typosaurus::num::consts::{U0, U1, U2};

action!(
    AdjustEnergy,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| {
        std::future::ready(Ok(input + 1))
    }
);

#[derive(Clone, Debug, PartialEq, Eq)]
struct CoreState {
    energy: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GorillaState {
    core: CoreState,
    bananas: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TigerState {
    core: CoreState,
    stripes: u8,
}

struct GorillaCoreAspect;
impl Aspect<GorillaState> for GorillaCoreAspect {
    type View = CoreState;

    fn view(state: &GorillaState) -> &Self::View {
        &state.core
    }

    fn view_mut(state: &mut GorillaState) -> &mut Self::View {
        &mut state.core
    }
}

struct TigerCoreAspect;
impl Aspect<TigerState> for TigerCoreAspect {
    type View = CoreState;

    fn view(state: &TigerState) -> &Self::View {
        &state.core
    }

    fn view_mut(state: &mut TigerState) -> &mut Self::View {
        &mut state.core
    }
}

struct CoreEnergyStep<Focus>(PhantomData<fn() -> Focus>);

impl<T, Focus> AspectStep<T, AdjustEnergy> for CoreEnergyStep<Focus>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn apply(core: &mut CoreState, output: ActionCompletion<AdjustEnergy>) -> Self::Out {
        let value = output.expect("adjust energy should succeed");
        core.energy = value;
        value
    }
}

#[derive(Inception)]
#[inception(properties = [JungleFlowActions])]
struct GorillaInstinct(ActionStep<GorillaAnimal, AdjustEnergy, CoreEnergyStep<GorillaCoreAspect>>);

#[derive(Inception)]
#[inception(properties = [JungleFlowActions])]
struct TigerInstinct(ActionStep<TigerAnimal, AdjustEnergy, CoreEnergyStep<TigerCoreAspect>>);

animal!(
    GorillaAnimal,
    U1,
    state = GorillaState,
    instinct = GorillaInstinct
);
animal!(
    TigerAnimal,
    U2,
    state = TigerState,
    instinct = TigerInstinct
);

type GorillaStep = ActionStep<GorillaAnimal, AdjustEnergy, CoreEnergyStep<GorillaCoreAspect>>;
type TigerStep = ActionStep<TigerAnimal, AdjustEnergy, CoreEnergyStep<TigerCoreAspect>>;

impl TestFlow for GorillaInstinct {
    type State = GorillaState;

    fn build_steps() -> Vec<Box<dyn ErasedStep<Self::State>>> {
        vec![Box::new(TypedErasedStep::<GorillaStep>::new())]
    }
}

impl TestFlow for TigerInstinct {
    type State = TigerState;

    fn build_steps() -> Vec<Box<dyn ErasedStep<Self::State>>> {
        vec![Box::new(TypedErasedStep::<TigerStep>::new())]
    }
}

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState { energy: 10 },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) = <GorillaStep as Yielding>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) =
        <GorillaStep as Awaiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        core: CoreState { energy: 6 },
        stripes: 9,
    };
    let (tiger_state, tiger_request) = <TigerStep as Yielding>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <TigerStep as Awaiting>::accept((tiger_state, Ok(15)));
    assert_eq!(tiger_emitted, 15);
    assert_eq!(tiger_state.core.energy, 15);
    assert_eq!(tiger_state.stripes, 9);
}

#[test]
fn test_executor_runs_aspected_steps() {
    let mut gorilla = TestExecutor::<GorillaAnimal>::new(GorillaState {
        core: CoreState { energy: 5 },
        bananas: 2,
    });
    let gorilla_emitted = gorilla
        .next(json!(3), Ok(json!(11)))
        .expect("gorilla aspect step");
    assert_eq!(gorilla_emitted, json!(11));
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 11);
    assert_eq!(gorilla_state.bananas, 2);

    let mut tiger = TestExecutor::<TigerAnimal>::new(TigerState {
        core: CoreState { energy: 8 },
        stripes: 7,
    });
    let tiger_emitted = tiger
        .next(json!(1), Ok(json!(9)))
        .expect("tiger aspect step");
    assert_eq!(tiger_emitted, json!(9));
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 9);
    assert_eq!(tiger_state.stripes, 7);
}
