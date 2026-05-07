use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    Action, ActionCompletion, Impulse, Aspect, Condition, Conditional, Either, Executor,
    Identity, Lens, LoopCondition, Running, Task, Waiting, While,
};
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3};
use jungle_sdk::{Instinct, Optic};
use std::future::ready;
use std::marker::PhantomData;

action!(Sleep, U0, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
action!(Eat, U1, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
action!(Forage, U2, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input - 1)));
action!(Hunt, U3, in = (), out = i32, err = (), act = |_d, _input| ready(Ok(1)));

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CoreState {
    energy: i32,
    age: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct GorillaState {
    core: CoreState,
    bananas: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct TigerState {
    stripes: u8,
    core: CoreState,
}

struct CoreEnergyStep<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, Focus> Task<T> for CoreEnergyStep<Sleep, Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Action = Sleep;
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn process(core: &mut CoreState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> Task<T> for CoreEnergyStep<Eat, Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Action = Eat;
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn process(core: &mut CoreState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Task<T> for AddI32<Focus, A>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = i32>,
    A: Action<Out = i32>,
{
    type Action = A;
    type Aspect = Focus;
    type In = A::In;
    type Out = i32;

    fn prepare(_value: &i32, input: Self::In) -> A::In {
        input
    }

    fn process(value: &mut i32, output: ActionCompletion<A>) -> Self::Out {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("action should succeed"),
        };
        *value += delta;
        *value
    }
}

struct SubI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Task<T> for SubI32<Focus, A>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = i32>,
    A: Action<Out = i32>,
{
    type Action = A;
    type Aspect = Focus;
    type In = A::In;
    type Out = i32;

    fn prepare(_value: &i32, input: Self::In) -> A::In {
        input
    }

    fn process(value: &mut i32, output: ActionCompletion<A>) -> Self::Out {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("action should succeed"),
        };
        *value -= delta;
        *value
    }
}

struct GorillaSleepManual;
impl Task<Gorilla> for GorillaSleepManual {
    type Action = Sleep;
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn process(state: &mut GorillaState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        state.core.age += 1;
        value
    }
}

type GorillaEat = AddI32<Lens<GorillaState, list![U0, U0]>, Eat>;
type GorillaForageStep = AddI32<Lens<GorillaState, list![U0, U0]>, Forage>;

type TigerEat = AddI32<Lens<TigerState, list![U1, U0]>, Eat>;
type TigerSleep = AddI32<Lens<TigerState, list![U1, U0]>, Sleep>;

#[derive(Instinct)]
struct GorillaLoopSequence(
    Impulse<Gorilla, GorillaEat>,
    Impulse<Gorilla, GorillaSleepManual>,
    Impulse<Gorilla, GorillaForageStep>,
);

struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(Instinct)]
struct GorillaInstinct(While<GorillaUnderAgeHundred, GorillaLoopSequence>);

struct TigerStripesAreEven;
impl Condition<(TigerState, i32)> for TigerStripesAreEven {
    fn choose((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

#[derive(Instinct)]
struct TigerLoopSequence(
    Conditional<
        TigerStripesAreEven,
        Impulse<Tiger, TigerEat>,
        Impulse<Tiger, TigerSleep>,
    >,
    Impulse<Tiger, TigerSleep>,
    Impulse<Tiger, AddI32<Lens<TigerState, list![U1, U0]>, Hunt>>,
);

struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    fn should_continue(state: &TigerState) -> bool {
        state.core.energy < 100
    }
}

#[derive(Instinct)]
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
    let (gorilla_state, gorilla_request) = <Impulse<
        Gorilla,
        CoreEnergyStep<Sleep, Lens<GorillaState, U0>>,
    > as Running>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) = <Impulse<
        Gorilla,
        CoreEnergyStep<Sleep, Lens<GorillaState, U0>>,
    > as Waiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.core.age, 25);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        stripes: 9,
        core: CoreState { energy: 6, age: 12 },
    };
    let (tiger_state, tiger_request) = <Impulse<
        Tiger,
        CoreEnergyStep<Sleep, Lens<TigerState, U1>>,
    > as Running>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <Impulse<
        Tiger,
        CoreEnergyStep<Sleep, Lens<TigerState, U1>>,
    > as Waiting>::accept((tiger_state, Ok(15)));
    assert_eq!(tiger_emitted, 15);
    assert_eq!(tiger_state.core.energy, 15);
    assert_eq!(tiger_state.core.age, 12);
    assert_eq!(tiger_state.stripes, 9);
}

#[tokio::test]
async fn executor_runs_aspected_steps() {
    let mut gorilla = Executor::<Gorilla>::new(GorillaState {
        core: CoreState { energy: 5, age: 97 },
        bananas: 2,
    });
    assert!(!gorilla.is_complete());

    let mut gorilla_emitted: Vec<i32> = Vec::new();
    for step in 0..8 {
        let request: i32 = gorilla
            .next_request()
            .expect("gorilla request should advance");
        let completion: i32 = match step % 3 {
            0 => Eat::act(&(), request).await.expect("eat should succeed"),
            1 => Sleep::act(&(), request).await.expect("sleep should succeed"),
            2 => Forage::act(&(), request).await.expect("forage should succeed"),
            _ => unreachable!(),
        };
        let emitted: i32 = gorilla
            .complete(Ok::<i32, ()>(completion))
            .expect("gorilla completion should advance");
        gorilla_emitted.push(emitted);
    }
    assert_eq!(gorilla_emitted, vec![6, 13, 25, 51, 103, 205, 411, 823]);
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 823);
    assert_eq!(gorilla_state.core.age, 100);
    assert_eq!(gorilla_state.bananas, 2);

    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });
    assert!(!tiger.is_complete());

    let mut tiger_emitted: Vec<i32> = Vec::new();
    while !tiger.is_complete() {
        let step = tiger_emitted.len() % 3;
        let completion: i32 = match step % 3 {
            0 => {
                let request: i32 = tiger.next_request().expect("tiger request should advance");
                Eat::act(&(), request).await.expect("eat should succeed")
            }
            1 => {
                let request: i32 = tiger.next_request().expect("tiger request should advance");
                Sleep::act(&(), request).await.expect("sleep should succeed")
            }
            2 => {
                let request: () = tiger.next_request().expect("tiger request should advance");
                Hunt::act(&(), request).await.expect("hunt should succeed")
            }
            _ => unreachable!(),
        };
        let emitted: i32 = tiger
            .complete(Ok::<i32, ()>(completion))
            .expect("tiger completion should advance");
        tiger_emitted.push(emitted);
    }
    assert_eq!(tiger_emitted, vec![9, 19, 20, 41, 83, 84, 169]);
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 169);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 98);
}

#[test]
fn tiger_first_step_conditional_selects_branch_from_stripe_parity() {
    let even = <Conditional<
        TigerStripesAreEven,
        Impulse<Tiger, TigerEat>,
        Impulse<Tiger, TigerSleep>,
    > as Running>::run((
        TigerState {
            stripes: 8,
            core: CoreState { energy: 5, age: 1 },
        },
        0,
    ));
    match even {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 0),
        Either::Right(_) => panic!("expected eat branch"),
    }

    let odd = <Conditional<
        TigerStripesAreEven,
        Impulse<Tiger, TigerEat>,
        Impulse<Tiger, TigerSleep>,
    > as Running>::run((
        TigerState {
            stripes: 9,
            core: CoreState { energy: 5, age: 1 },
        },
        0,
    ));
    match odd {
        Either::Left(_) => panic!("expected sleep branch"),
        Either::Right((_state, request)) => assert_eq!(request.into_input(), 0),
    }
}

#[tokio::test]
async fn executor_advances_with_executable_requests_and_dynamic_action_order() {
    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });

    let emitted = tiger
        .advance_to_end_with(0i32)
        .await
        .expect("tiger flow should execute through dynamic requests");
    assert_eq!(emitted.len(), 7);
    assert!(tiger.is_complete());

    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 169);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 98);
}
