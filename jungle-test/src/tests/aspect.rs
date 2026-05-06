use jungle_sdk::{Instinct, Jungle};
use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    ActionCompletion, ActionStep, Aspect, AspectStep, Condition, Conditional, Either, Identity,
    LoopCondition, Running, TestExecutor, Waiting, While,
};
use serde_json::json;
use std::future::ready;
use std::marker::PhantomData;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3};

action!(Sleep, U0, in = i32, out = i32, err = (), act = |_dependency, input| ready(Ok(input + 1)));
action!(Eat, U1, in = i32, out = i32, err = (), act = |_dependency, input| ready(Ok(input + 1)));
action!(Forage, U2, in = i32, out = i32, err = (), act = |_dependency, input| ready(Ok(input - 1)));
action!(Hunt, U3, in = i32, out = i32, err = (), act = |_dependency, input| ready(Ok(input - 1)));

#[derive(Clone, Debug, PartialEq, Eq)]
struct CoreState {
    energy: i32,
    age: i32,
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

    fn view(state: &mut GorillaState) -> &mut Self::View {
        &mut state.core
    }
}

struct TigerCoreAspect;
impl Aspect<TigerState> for TigerCoreAspect {
    type View = CoreState;

    fn view(state: &mut TigerState) -> &mut Self::View {
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

struct GorillaEat;
impl AspectStep<Gorilla, Eat> for GorillaEat {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn apply(state: &mut GorillaState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        state.core.energy = value;
        state.bananas -= 1;
        value
    }
}

struct GorillaSleep;
impl AspectStep<Gorilla, Sleep> for GorillaSleep {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn apply(state: &mut GorillaState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        state.core.age += 1;
        value
    }
}

struct GorillaForage;
impl AspectStep<Gorilla, Forage> for GorillaForage {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn apply(state: &mut GorillaState, output: ActionCompletion<Forage>) -> Self::Out {
        let value = output.expect("forage should succeed");
        state.core.energy = value;
        state.bananas += 1;
        value
    }
}

struct TigerEat;
impl AspectStep<Tiger, Eat> for TigerEat {
    type Aspect = TigerCoreAspect;
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

struct TigerSleep;
impl AspectStep<Tiger, Sleep> for TigerSleep {
    type Aspect = TigerCoreAspect;
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

struct TigerHunt;
impl AspectStep<Tiger, Hunt> for TigerHunt {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &TigerState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn apply(state: &mut TigerState, output: ActionCompletion<Hunt>) -> Self::Out {
        let value = output.expect("hunt should succeed");
        state.core.energy = value;
        state.stripes += 1;
        value
    }
}

#[derive(Jungle, Instinct)]
struct GorillaLoopSequence(
    ActionStep<Gorilla, Eat, GorillaEat>,
    ActionStep<Gorilla, Sleep, GorillaSleep>,
    ActionStep<Gorilla, Forage, GorillaForage>,
);

struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(Jungle, Instinct)]
struct GorillaInstinct(While<GorillaUnderAgeHundred, GorillaLoopSequence>);

struct TigerStripesAreEven;
impl Condition<(TigerState, i32)> for TigerStripesAreEven {
    fn choose((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

#[derive(Jungle, Instinct)]
struct TigerLoopSequence(
    Conditional<
        TigerStripesAreEven,
        ActionStep<Tiger, Eat, TigerEat>,
        ActionStep<Tiger, Sleep, TigerSleep>,
    >,
    ActionStep<Tiger, Sleep, TigerSleep>,
    ActionStep<Tiger, Hunt, TigerHunt>,
);

struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    fn should_continue(state: &TigerState) -> bool {
        state.stripes < 100
    }
}

#[derive(Jungle, Instinct)]
struct TigerInstinct(While<TigerUnderHundredStripes, TigerLoopSequence>);

animal!(
    Gorilla,
    U1,
    state = GorillaState,
    instinct = GorillaInstinct
);
animal!(Tiger, U2, state = TigerState, instinct = TigerInstinct);

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState {
            energy: 10,
            age: 25,
        },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) =
        <ActionStep<Gorilla, Sleep, CoreEnergyStep<GorillaCoreAspect>> as Running>::run((
            gorilla_state,
            2,
        ));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) =
        <ActionStep<Gorilla, Sleep, CoreEnergyStep<GorillaCoreAspect>> as Waiting>::accept((
            gorilla_state,
            Ok(20),
        ));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.core.age, 25);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        core: CoreState { energy: 6, age: 12 },
        stripes: 9,
    };
    let (tiger_state, tiger_request) =
        <ActionStep<Tiger, Sleep, CoreEnergyStep<TigerCoreAspect>> as Running>::run((
            tiger_state,
            4,
        ));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) =
        <ActionStep<Tiger, Sleep, CoreEnergyStep<TigerCoreAspect>> as Waiting>::accept((
            tiger_state,
            Ok(15),
        ));
    assert_eq!(tiger_emitted, 15);
    assert_eq!(tiger_state.core.energy, 15);
    assert_eq!(tiger_state.core.age, 12);
    assert_eq!(tiger_state.stripes, 9);
}

#[test]
fn test_executor_runs_aspected_steps() {
    let mut gorilla = TestExecutor::<Gorilla>::new(GorillaState {
        core: CoreState { energy: 5, age: 97 },
        bananas: 2,
    });
    assert!(!gorilla.is_complete());

    let mut gorilla_emitted = vec![
        gorilla
            .next(json!(0), Ok(json!(6)))
            .expect("gorilla step 1 should advance"),
        gorilla
            .next(json!(0), Ok(json!(7)))
            .expect("gorilla step 2 should advance"),
        gorilla
            .next(json!(0), Ok(json!(6)))
            .expect("gorilla step 3 should advance"),
    ];
    gorilla_emitted.extend(
        gorilla
            .advance_to_end(vec![
                (json!(0), Ok(json!(7))),
                (json!(0), Ok(json!(8))),
                (json!(0), Ok(json!(7))),
                (json!(0), Ok(json!(8))),
                (json!(0), Ok(json!(9))),
            ])
            .expect("gorilla loop should advance"),
    );
    assert_eq!(
        gorilla_emitted,
        vec![
            json!(6),
            json!(7),
            json!(6),
            json!(7),
            json!(8),
            json!(7),
            json!(8),
            json!(9),
        ]
    );
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 9);
    assert_eq!(gorilla_state.core.age, 100);
    assert_eq!(gorilla_state.bananas, 1);

    let mut tiger = TestExecutor::<Tiger>::new(TigerState {
        core: CoreState { energy: 8, age: 4 },
        stripes: 98,
    });
    assert!(!tiger.is_complete());

    let tiger_emitted = tiger
        .advance_to_end(vec![
            (json!(0), Ok(json!(9))),
            (json!(0), Ok(json!(10))),
            (json!(0), Ok(json!(9))),
            (json!(0), Ok(json!(10))),
            (json!(0), Ok(json!(11))),
            (json!(0), Ok(json!(10))),
        ])
        .expect("tiger loop should advance");
    assert_eq!(
        tiger_emitted,
        vec![
            json!(9),
            json!(10),
            json!(9),
            json!(10),
            json!(11),
            json!(10)
        ]
    );
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 10);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 100);
}

#[test]
fn tiger_first_step_conditional_selects_branch_from_stripe_parity() {
    let even = <Conditional<
        TigerStripesAreEven,
        ActionStep<Tiger, Eat, TigerEat>,
        ActionStep<Tiger, Sleep, TigerSleep>,
    > as Running>::run((
        TigerState {
            core: CoreState { energy: 5, age: 1 },
            stripes: 8,
        },
        0,
    ));
    match even {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 5),
        Either::Right(_) => panic!("expected eat branch"),
    }

    let odd = <Conditional<
        TigerStripesAreEven,
        ActionStep<Tiger, Eat, TigerEat>,
        ActionStep<Tiger, Sleep, TigerSleep>,
    > as Running>::run((
        TigerState {
            core: CoreState { energy: 5, age: 1 },
            stripes: 9,
        },
        0,
    ));
    match odd {
        Either::Left(_) => panic!("expected sleep branch"),
        Either::Right((_state, request)) => assert_eq!(request.into_input(), 5),
    }
}
