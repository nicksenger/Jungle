
use jungle_sdk::{Instinct, Jungle};
use jungle_types::{
    ActionCompletion, ActionStep, Aspect, AspectStep, Running, TestExecutor, Waiting,
};
use serde_json::json;
use std::marker::PhantomData;
use typosaurus::num::consts::{U0, U1, U2};

action!(
    Sleep,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| {
        std::future::ready(Ok(input + 1))
    }
);

action!(
    Eat,
    U1,
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

impl<T, Focus> AspectStep<T, Sleep> for CoreEnergyStep<Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn apply(core: &mut CoreState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> AspectStep<T, Eat> for CoreEnergyStep<Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn apply(core: &mut CoreState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

type CoreEnergySleepActionStep<T, Focus> = ActionStep<T, Sleep, CoreEnergyStep<Focus>>;
type CoreEnergyEatActionStep<T, Focus> = ActionStep<T, Eat, CoreEnergyStep<Focus>>;

struct CoreEnergyConditionalEatStep<Focus>(PhantomData<fn() -> Focus>);

impl<T, Focus> AspectStep<T, Eat> for CoreEnergyConditionalEatStep<Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        if core.energy < 10 {
            core.energy + input
        } else {
            input
        }
    }

    fn apply(core: &mut CoreState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("conditional eat should succeed");
        core.energy = value;
        value
    }
}

type CoreEnergyConditionalEatActionStep<T, Focus> =
    ActionStep<T, Eat, CoreEnergyConditionalEatStep<Focus>>;

#[derive(Jungle, Instinct)]
struct GorillaInstinct(
    CoreEnergySleepActionStep<Gorilla, GorillaCoreAspect>,
    CoreEnergyConditionalEatActionStep<Gorilla, GorillaCoreAspect>,
    CoreEnergySleepActionStep<Gorilla, GorillaCoreAspect>,
);

#[derive(Jungle, Instinct)]
struct TigerInstinct(
    CoreEnergySleepActionStep<Tiger, TigerCoreAspect>,
    CoreEnergyEatActionStep<Tiger, TigerCoreAspect>,
);

animal!(
    Gorilla,
    U1,
    state = GorillaState,
    instinct = GorillaInstinct
);
animal!(
    Tiger,
    U2,
    state = TigerState,
    instinct = TigerInstinct
);

type GorillaStep = CoreEnergySleepActionStep<Gorilla, GorillaCoreAspect>;
type TigerStep = CoreEnergySleepActionStep<Tiger, TigerCoreAspect>;
type GorillaConditionalEatStep = CoreEnergyConditionalEatActionStep<Gorilla, GorillaCoreAspect>;

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState { energy: 10 },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) = <GorillaStep as Running>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) =
        <GorillaStep as Waiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        core: CoreState { energy: 6 },
        stripes: 9,
    };
    let (tiger_state, tiger_request) = <TigerStep as Running>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <TigerStep as Waiting>::accept((tiger_state, Ok(15)));
    assert_eq!(tiger_emitted, 15);
    assert_eq!(tiger_state.core.energy, 15);
    assert_eq!(tiger_state.stripes, 9);
}

#[test]
fn test_executor_runs_aspected_steps() {
    let mut gorilla = TestExecutor::<Gorilla>::new(GorillaState {
        core: CoreState { energy: 5 },
        bananas: 2,
    });
    let gorilla_emitted = gorilla
        .next(json!(3), Ok(json!(11)))
        .expect("gorilla aspect step");
    assert_eq!(gorilla_emitted, json!(11));
    assert!(!gorilla.is_complete());
    let gorilla_emitted = gorilla
        .next(json!(2), Ok(json!(13)))
        .expect("gorilla aspect step");
    assert_eq!(gorilla_emitted, json!(13));
    assert!(!gorilla.is_complete());
    let gorilla_emitted = gorilla
        .next(json!(1), Ok(json!(14)))
        .expect("gorilla aspect step");
    assert_eq!(gorilla_emitted, json!(14));
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 14);
    assert_eq!(gorilla_state.bananas, 2);

    let mut tiger = TestExecutor::<Tiger>::new(TigerState {
        core: CoreState { energy: 8 },
        stripes: 7,
    });
    let tiger_emitted = tiger
        .next(json!(1), Ok(json!(9)))
        .expect("tiger aspect step");
    assert_eq!(tiger_emitted, json!(9));
    assert!(!tiger.is_complete());
    let tiger_emitted = tiger
        .next(json!(2), Ok(json!(11)))
        .expect("tiger aspect step");
    assert_eq!(tiger_emitted, json!(11));
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 11);
    assert_eq!(tiger_state.stripes, 7);
}

#[test]
fn conditional_mapper_branches_by_core_energy() {
    let low_energy_state = GorillaState {
        core: CoreState { energy: 4 },
        bananas: 1,
    };
    let (_state, request) = <GorillaConditionalEatStep as Running>::run((low_energy_state, 3));
    assert_eq!(request.into_input(), 7);

    let high_energy_state = GorillaState {
        core: CoreState { energy: 12 },
        bananas: 1,
    };
    let (_state, request) = <GorillaConditionalEatStep as Running>::run((high_energy_state, 3));
    assert_eq!(request.into_input(), 3);
}

#[test]
fn repeated_steps_form_a_simple_loop_pattern() {
    let mut gorilla = TestExecutor::<Gorilla>::new(GorillaState {
        core: CoreState { energy: 1 },
        bananas: 5,
    });

    let emitted = gorilla
        .advance_to_end(vec![
            (json!(1), Ok(json!(2))),
            (json!(2), Ok(json!(4))),
            (json!(1), Ok(json!(5))),
        ])
        .expect("gorilla loop-like flow");

    assert_eq!(emitted, vec![json!(2), json!(4), json!(5)]);
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 5);
    assert_eq!(gorilla_state.bananas, 5);
}
