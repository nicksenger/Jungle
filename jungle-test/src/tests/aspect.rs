use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    Effect, EffectCompletion, Aspect, Condition, Conditional, Either, Executor, Identity,
    LoopCondition, Pulse, Running, StateLens, Step, Waiting, While,
};
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3};
use jungle_sdk::{Journey, Optic};
use std::future::ready;
use std::marker::PhantomData;

effect!(Sleep, U0, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
effect!(Eat, U1, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
effect!(Forage, U2, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input - 1)));
effect!(Hunt, U3, in = (), out = i32, err = (), act = |_d, _input| ready(Ok(1)));

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

impl<T, Focus> Pulse<T> for CoreEnergyStep<Sleep, Focus>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Effect = Sleep;
    type StateAspect = Focus;
    type Arg = i32;
    type Ret = i32;

    fn emit(core: &CoreState, input: Self::Arg) -> i32 {
        core.energy + input
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Sleep>) -> Self::Ret {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> Pulse<T> for CoreEnergyStep<Eat, Focus>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Effect = Eat;
    type StateAspect = Focus;
    type Arg = i32;
    type Ret = i32;

    fn emit(core: &CoreState, input: Self::Arg) -> i32 {
        core.energy + input
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Eat>) -> Self::Ret {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Pulse<T> for AddI32<Focus, A>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = i32>,
    A: Effect<Out = i32>,
{
    type Effect = A;
    type StateAspect = Focus;
    type Arg = A::In;
    type Ret = i32;

    fn emit(_value: &i32, input: Self::Arg) -> A::In {
        input
    }

    fn absorb(value: &mut i32, output: EffectCompletion<A>) -> Self::Ret {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("effect should succeed"),
        };
        *value += delta;
        *value
    }
}

struct SubI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> Pulse<T> for SubI32<Focus, A>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = i32>,
    A: Effect<Out = i32>,
{
    type Effect = A;
    type StateAspect = Focus;
    type Arg = A::In;
    type Ret = i32;

    fn emit(_value: &i32, input: Self::Arg) -> A::In {
        input
    }

    fn absorb(value: &mut i32, output: EffectCompletion<A>) -> Self::Ret {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("effect should succeed"),
        };
        *value -= delta;
        *value
    }
}

struct GorillaSleepManual;
impl Pulse<Gorilla> for GorillaSleepManual {
    type Effect = Sleep;
    type StateAspect = Identity;
    type Arg = i32;
    type Ret = i32;

    fn emit(state: &GorillaState, input: Self::Arg) -> i32 {
        state.core.energy + input
    }

    fn absorb(state: &mut GorillaState, output: EffectCompletion<Sleep>) -> Self::Ret {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        state.core.age += 1;
        value
    }
}

type GorillaEat = AddI32<StateLens<GorillaState, list![U0, U0]>, Eat>;
type GorillaForageStep = SubI32<StateLens<GorillaState, list![U0, U0]>, Forage>;

type TigerEat = AddI32<StateLens<TigerState, list![U1, U0]>, Eat>;
type TigerSleep = AddI32<StateLens<TigerState, list![U1, U0]>, Sleep>;

#[derive(Journey)]
struct GorillaLoopSequence(
    Step<Gorilla, GorillaEat>,
    Step<Gorilla, GorillaSleepManual>,
    Step<Gorilla, GorillaForageStep>,
);

struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    type Arg = i32;

    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(Journey)]
struct GorillaJourney(While<GorillaUnderAgeHundred, GorillaLoopSequence>);

struct TigerStripesAreEven;
impl Condition<(TigerState, i32)> for TigerStripesAreEven {
    fn choose((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

#[derive(Journey)]
struct TigerLoopSequence(
    Conditional<TigerStripesAreEven, Step<Tiger, TigerEat>, Step<Tiger, TigerSleep>>,
    Step<Tiger, TigerSleep>,
    Step<Tiger, AddI32<StateLens<TigerState, list![U1, U0]>, Hunt>>,
);

struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    type Arg = i32;

    fn should_continue(state: &TigerState) -> bool {
        state.core.energy < 100
    }
}

#[derive(Journey)]
struct TigerJourney(While<TigerUnderHundredStripes, TigerLoopSequence>);

animal!(Gorilla, U1, state = GorillaState, journey = GorillaJourney);
animal!(Tiger, U2, state = TigerState, journey = TigerJourney);

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState {
            energy: 10,
            age: 25,
        },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) = <Step<
        Gorilla,
        CoreEnergyStep<Sleep, StateLens<GorillaState, U0>>,
    > as Running>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) = <Step<
        Gorilla,
        CoreEnergyStep<Sleep, StateLens<GorillaState, U0>>,
    > as Waiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.core.age, 25);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        stripes: 9,
        core: CoreState { energy: 6, age: 12 },
    };
    let (tiger_state, tiger_request) = <Step<
        Tiger,
        CoreEnergyStep<Sleep, StateLens<TigerState, U1>>,
    > as Running>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <Step<
        Tiger,
        CoreEnergyStep<Sleep, StateLens<TigerState, U1>>,
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
            1 => Sleep::act(&(), request)
                .await
                .expect("sleep should succeed"),
            2 => Forage::act(&(), request)
                .await
                .expect("forage should succeed"),
            _ => unreachable!(),
        };
        let emitted: i32 = gorilla
            .complete(Ok::<i32, ()>(completion))
            .expect("gorilla completion should advance");
        gorilla_emitted.push(emitted);
    }
    assert_eq!(gorilla_emitted, vec![6, 13, 1, 3, 7, 1, 3, 7]);
    assert!(!gorilla.is_complete());
    assert!(gorilla.next_request::<i32>().is_err());
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 7);
    assert_eq!(gorilla_state.core.age, 100);
    assert_eq!(gorilla_state.bananas, 2);

    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });
    assert!(!tiger.is_complete());

    let mut tiger_emitted: Vec<i32> = Vec::new();
    loop {
        let step = tiger_emitted.len() % 3;
        let completion: i32 = match step % 3 {
            0 => {
                let request: i32 = match tiger.next_request() {
                    Ok(request) => request,
                    Err(jungle_sdk::types::ExecutorError::Complete) => break,
                    Err(err) => panic!("tiger request should advance: {err}"),
                };
                Eat::act(&(), request).await.expect("eat should succeed")
            }
            1 => {
                let request: i32 = match tiger.next_request() {
                    Ok(request) => request,
                    Err(jungle_sdk::types::ExecutorError::Complete) => break,
                    Err(err) => panic!("tiger request should advance: {err}"),
                };
                Sleep::act(&(), request)
                    .await
                    .expect("sleep should succeed")
            }
            2 => {
                let request: () = match tiger.next_request() {
                    Ok(request) => request,
                    Err(jungle_sdk::types::ExecutorError::Complete) => break,
                    Err(err) => panic!("tiger request should advance: {err}"),
                };
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
        Step<Tiger, TigerEat>,
        Step<Tiger, TigerSleep>,
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
        Step<Tiger, TigerEat>,
        Step<Tiger, TigerSleep>,
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
async fn executor_advances_with_executable_requests_and_dynamic_effect_order() {
    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });

    let mut emitted = Vec::new();
    loop {
        let request = match tiger.next_executable_request(0i32) {
            Ok(request) => request,
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("tiger flow should execute through dynamic requests: {err}"),
        };
        let completion = request.run().await.expect("tiger effect should execute");
        let emitted_step = tiger
            .complete_serialized(completion)
            .expect("tiger completion should process");
        emitted.push(emitted_step);
    }
    assert_eq!(emitted.len(), 7);
    assert!(tiger.is_complete());

    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 169);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 98);
}
