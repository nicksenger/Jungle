use jungle_sdk::act;
use jungle_sdk::animal;
use jungle_sdk::effect;
use jungle_sdk::types as jungle_types;
use jungle_sdk::types::Animal;
use jungle_sdk::types::{
    Act, Aspect, BoundAct, BoundFlowStep, Condition, Conditional, EffectCompletion, EffectExec,
    EffectSchema, Either, Executor, Identity, LoopCondition, Running, StateCarrier, Waiting, While,
    Step,
};
use jungle_sdk::Optic;
use serde::{Deserialize, Serialize};
use std::future::ready;
use std::marker::PhantomData;

struct Sleep;

#[effect(id = 0)]
impl<J> jungle_sdk::types::Effect<J> for Sleep {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}
struct Eat;

#[effect(id = 1)]
impl<J> jungle_sdk::types::Effect<J> for Eat {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}
struct Forage;

#[effect(id = 2)]
impl<J> jungle_sdk::types::Effect<J> for Forage {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input - 1))
    }
}
struct Hunt;

#[effect(id = 3)]
impl<J> jungle_sdk::types::Effect<J> for Hunt {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(1))
    }
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CoreState {
    energy: i32,
    age: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GorillaState {
    core: CoreState,
    bananas: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TigerState {
    stripes: u8,
    core: CoreState,
}

struct GorillaCoreCarrier;
impl StateCarrier<GorillaState> for GorillaCoreCarrier {
    type View = CoreState;

    fn view<'a>(state: &'a mut GorillaState) -> &'a mut Self::View {
        &mut state.core
    }
}

struct TigerCoreCarrier;
impl StateCarrier<TigerState> for TigerCoreCarrier {
    type View = CoreState;

    fn view<'a>(state: &'a mut TigerState) -> &'a mut Self::View {
        &mut state.core
    }
}

struct GorillaEnergyCarrier;
impl StateCarrier<GorillaState> for GorillaEnergyCarrier {
    type View = i32;

    fn view<'a>(state: &'a mut GorillaState) -> &'a mut Self::View {
        &mut state.core.energy
    }
}

struct TigerEnergyCarrier;
impl StateCarrier<TigerState> for TigerEnergyCarrier {
    type View = i32;

    fn view<'a>(state: &'a mut TigerState) -> &'a mut Self::View {
        &mut state.core.energy
    }
}

struct CoreEnergyStep<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, Focus> BoundAct<T> for CoreEnergyStep<Sleep, Focus>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Effect = Sleep;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;

    fn emit(core: &CoreState, input: Self::Input) -> i32 {
        core.energy + input
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Sleep>) -> Self::Output {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> BoundAct<T> for CoreEnergyStep<Eat, Focus>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Effect = Eat;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;

    fn emit(core: &CoreState, input: Self::Input) -> i32 {
        core.energy + input
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Eat>) -> Self::Output {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> BoundAct<T> for AddI32<Focus, A>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = i32>,
    A: EffectSchema<Out = i32>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = A::In;
    type Output = i32;

    fn emit(_value: &i32, input: Self::Input) -> A::In {
        input
    }

    fn absorb(value: &mut i32, output: EffectCompletion<A>) -> Self::Output {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("effect should succeed"),
        };
        *value += delta;
        *value
    }
}

struct SubI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> BoundAct<T> for SubI32<Focus, A>
where
    T: jungle_types::Animal,
    Focus: Aspect<T::State, View = i32>,
    A: EffectSchema<Out = i32>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = A::In;
    type Output = i32;

    fn emit(_value: &i32, input: Self::Input) -> A::In {
        input
    }

    fn absorb(value: &mut i32, output: EffectCompletion<A>) -> Self::Output {
        let delta = match output {
            Ok(delta) => delta,
            Err(_) => panic!("effect should succeed"),
        };
        *value -= delta;
        *value
    }
}

type GorillaEat = AddI32<GorillaEnergyCarrier, Eat>;
type GorillaForageStep = SubI32<GorillaEnergyCarrier, Forage>;

type TigerEat = AddI32<TigerEnergyCarrier, Eat>;
type TigerSleep = AddI32<TigerEnergyCarrier, Sleep>;

struct GorillaEatSpec;
impl Act for GorillaEatSpec {
    type Effect = Eat;
    type Input = i32;
    type Output = i32;
    type Bind<A: Animal> = AddI32<GorillaEnergyCarrier, Eat>;
}

struct GorillaSleepManualSpec;
#[act]
impl Act for GorillaSleepManualSpec {
    type Effect = Sleep;
    type Input = i32;
    type Output = i32;

    fn emit(state: &GorillaState, input: Self::Input) -> i32 {
        state.core.energy + input
    }

    fn absorb(state: &mut GorillaState, output: EffectCompletion<Sleep>) -> Self::Output {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        state.core.age += 1;
        value
    }
}

struct GorillaForageSpec;
impl Act for GorillaForageSpec {
    type Effect = Forage;
    type Input = i32;
    type Output = i32;
    type Bind<A: Animal> = SubI32<GorillaEnergyCarrier, Forage>;
}

#[derive(jungle_sdk::Flow)]
struct GorillaLoopTemplate(
    Step<GorillaEatSpec>,
    Step<GorillaSleepManualSpec>,
    Step<GorillaForageSpec>,
);

struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    type Arg = i32;

    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(jungle_sdk::Flow)]
struct GorillaJourneyTemplate(While<GorillaUnderAgeHundred, GorillaLoopTemplate>);

struct TigerStripesAreEven;
impl Condition<(TigerState, i32)> for TigerStripesAreEven {
    fn choose((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

struct TigerEatSpec;
impl Act for TigerEatSpec {
    type Effect = Eat;
    type Input = i32;
    type Output = i32;
    type Bind<A: Animal> = AddI32<TigerEnergyCarrier, Eat>;
}

struct TigerSleepSpec;
impl Act for TigerSleepSpec {
    type Effect = Sleep;
    type Input = i32;
    type Output = i32;
    type Bind<A: Animal> = AddI32<TigerEnergyCarrier, Sleep>;
}

struct TigerHuntSpec;
impl Act for TigerHuntSpec {
    type Effect = Hunt;
    type Input = ();
    type Output = i32;
    type Bind<A: Animal> = AddI32<TigerEnergyCarrier, Hunt>;
}

#[derive(jungle_sdk::Flow)]
struct TigerLoopTemplate(
    Conditional<TigerStripesAreEven, Step<TigerEatSpec>, Step<TigerSleepSpec>>,
    Step<TigerSleepSpec>,
    Step<TigerHuntSpec>,
);

struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    type Arg = i32;

    fn should_continue(state: &TigerState) -> bool {
        state.core.energy < 100
    }
}

#[derive(jungle_sdk::Flow)]
struct TigerJourneyTemplate(While<TigerUnderHundredStripes, TigerLoopTemplate>);

struct Gorilla;

#[animal(id = 1, generation = 0)]
impl Animal for Gorilla {
    type State = GorillaState;
    type Seed = GorillaState;
    type Journey = GorillaJourneyTemplate;
}
struct Tiger;

#[animal(id = 2, generation = 0)]
impl Animal for Tiger {
    type State = TigerState;
    type Seed = TigerState;
    type Journey = TigerJourneyTemplate;
}

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState {
            energy: 10,
            age: 25,
        },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) = <BoundFlowStep<
        Gorilla,
        CoreEnergyStep<Sleep, GorillaCoreCarrier>,
    > as Running>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) = <BoundFlowStep<
        Gorilla,
        CoreEnergyStep<Sleep, GorillaCoreCarrier>,
    > as Waiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.core.age, 25);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        stripes: 9,
        core: CoreState { energy: 6, age: 12 },
    };
    let (tiger_state, tiger_request) = <BoundFlowStep<
        Tiger,
        CoreEnergyStep<Sleep, TigerCoreCarrier>,
    > as Running>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <BoundFlowStep<
        Tiger,
        CoreEnergyStep<Sleep, TigerCoreCarrier>,
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
            0 => Eat::effect(&(), request).await.expect("eat should succeed"),
            1 => Sleep::effect(&(), request)
                .await
                .expect("sleep should succeed"),
            2 => Forage::effect(&(), request)
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
                Eat::effect(&(), request).await.expect("eat should succeed")
            }
            1 => {
                let request: i32 = match tiger.next_request() {
                    Ok(request) => request,
                    Err(jungle_sdk::types::ExecutorError::Complete) => break,
                    Err(err) => panic!("tiger request should advance: {err}"),
                };
                Sleep::effect(&(), request)
                    .await
                    .expect("sleep should succeed")
            }
            2 => {
                let request: () = match tiger.next_request() {
                    Ok(request) => request,
                    Err(jungle_sdk::types::ExecutorError::Complete) => break,
                    Err(err) => panic!("tiger request should advance: {err}"),
                };
                Hunt::effect(&(), request)
                    .await
                    .expect("hunt should succeed")
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
        BoundFlowStep<Tiger, TigerEat>,
        BoundFlowStep<Tiger, TigerSleep>,
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
        BoundFlowStep<Tiger, TigerEat>,
        BoundFlowStep<Tiger, TigerSleep>,
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
