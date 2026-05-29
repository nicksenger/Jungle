use jungle_sdk::prelude::*;
use jungle_sdk::Optic;
use serde::{Deserialize, Serialize};
use std::future::ready;
use std::marker::PhantomData;

pub struct Sleep;

#[jungle::effect(id = 0)]
impl<J> Effect<J> for Sleep {
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
pub struct Eat;

#[jungle::effect(id = 1)]
impl<J> Effect<J> for Eat {
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
pub struct Forage;

#[jungle::effect(id = 2)]
impl<J> Effect<J> for Forage {
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
pub struct Hunt;

#[jungle::effect(id = 3)]
impl<J> Effect<J> for Hunt {
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
pub struct CoreState {
    energy: i32,
    age: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GorillaState {
    core: CoreState,
    bananas: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TigerState {
    stripes: u8,
    core: CoreState,
}

pub struct GorillaCoreCarrier;
impl StateCarrier<GorillaState> for GorillaCoreCarrier {
    type Focus = CoreState;

    fn focus(state: &mut GorillaState) -> &mut Self::Focus {
        &mut state.core
    }
}

pub struct TigerCoreCarrier;
impl StateCarrier<TigerState> for TigerCoreCarrier {
    type Focus = CoreState;

    fn focus(state: &mut TigerState) -> &mut Self::Focus {
        &mut state.core
    }
}

pub struct GorillaEnergyCarrier;
impl StateCarrier<GorillaState> for GorillaEnergyCarrier {
    type Focus = i32;

    fn focus(state: &mut GorillaState) -> &mut Self::Focus {
        &mut state.core.energy
    }
}

pub struct TigerEnergyCarrier;
impl StateCarrier<TigerState> for TigerEnergyCarrier {
    type Focus = i32;

    fn focus(state: &mut TigerState) -> &mut Self::Focus {
        &mut state.core.energy
    }
}

pub struct CoreEnergyStep<A, Focus>(PhantomData<fn() -> (A, Focus)>);

impl<T, Focus> BoundAction<T> for CoreEnergyStep<Sleep, Focus>
where
    T: Animal,
    Focus: Aspect<T::State, Focus = CoreState>,
{
    type Effect = Sleep;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(core: &CoreState, input: Self::Input) -> i32 {
        core.energy + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Sleep>) -> Self::Output {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> BoundAction<T> for CoreEnergyStep<Eat, Focus>
where
    T: Animal,
    Focus: Aspect<T::State, Focus = CoreState>,
{
    type Effect = Eat;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(core: &CoreState, input: Self::Input) -> i32 {
        core.energy + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
    }

    fn absorb(core: &mut CoreState, output: EffectCompletion<Eat>) -> Self::Output {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

pub struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> BoundAction<T> for AddI32<Focus, A>
where
    T: Animal,
    Focus: Aspect<T::State, Focus = i32>,
    A: EffectSchema<Out = i32>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = A::In;
    type Output = i32;
    type Carry = ();

    fn emit(_value: &i32, input: Self::Input) -> A::In {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
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

pub struct SubI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);

impl<T, Focus, A> BoundAction<T> for SubI32<Focus, A>
where
    T: Animal,
    Focus: Aspect<T::State, Focus = i32>,
    A: EffectSchema<Out = i32>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = A::In;
    type Output = i32;
    type Carry = ();

    fn emit(_value: &i32, input: Self::Input) -> A::In {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
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

#[allow(dead_code)]
type GorillaEat = AddI32<GorillaEnergyCarrier, Eat>;
#[allow(dead_code)]
type GorillaForageStep = SubI32<GorillaEnergyCarrier, Forage>;

type TigerEat = AddI32<TigerEnergyCarrier, Eat>;
type TigerSleep = AddI32<TigerEnergyCarrier, Sleep>;

pub struct GorillaEatSpec;
#[jungle::action(bind = AddI32<GorillaEnergyCarrier, Eat>)]
impl Action for GorillaEatSpec {
    type Effect = Eat;
    type Input = i32;
    type Output = i32;
}

pub struct GorillaSleepManualSpec;
#[jungle::action]
impl Action for GorillaSleepManualSpec {
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

pub struct GorillaForageSpec;
#[jungle::action(bind = SubI32<GorillaEnergyCarrier, Forage>)]
impl Action for GorillaForageSpec {
    type Effect = Forage;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
pub struct GorillaLoopTemplate(
    Step<GorillaEatSpec>,
    Step<GorillaSleepManualSpec>,
    Step<GorillaForageSpec>,
);

pub struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    type Arg = i32;

    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(Flow)]
pub struct GorillaJourneyTemplate(While<GorillaUnderAgeHundred, GorillaLoopTemplate>);

pub struct TigerStripesAreEven;
impl Predicate<(TigerState, i32)> for TigerStripesAreEven {
    fn eval((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

pub struct TigerEatSpec;
#[jungle::action(bind = AddI32<TigerEnergyCarrier, Eat>)]
impl Action for TigerEatSpec {
    type Effect = Eat;
    type Input = i32;
    type Output = i32;
}

pub struct TigerSleepSpec;
#[jungle::action(bind = AddI32<TigerEnergyCarrier, Sleep>)]
impl Action for TigerSleepSpec {
    type Effect = Sleep;
    type Input = i32;
    type Output = i32;
}

pub struct TigerSleepFromEitherSpec;
#[jungle::action]
impl Action for TigerSleepFromEitherSpec {
    type Effect = Sleep;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &TigerState, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn absorb(state: &mut TigerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        value
    }
}

pub struct TigerHuntFromEnergySpec;
#[jungle::action]
impl Action for TigerHuntFromEnergySpec {
    type Effect = Hunt;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &TigerState, _input: Self::Input) -> () {}

    fn absorb(state: &mut TigerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let delta = output.expect("hunt should succeed");
        state.core.energy += delta;
        state.core.energy
    }
}

#[derive(Flow)]
pub struct TigerLoopTemplate(
    Conditional<TigerStripesAreEven, Step<TigerEatSpec>, Step<TigerSleepSpec>>,
    Step<TigerSleepFromEitherSpec>,
    Step<TigerHuntFromEnergySpec>,
);

pub struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    type Arg = i32;

    fn should_continue(state: &TigerState) -> bool {
        state.core.energy < 100
    }
}

#[derive(Flow)]
pub struct TigerJourneyTemplate(While<TigerUnderHundredStripes, TigerLoopTemplate>);

pub struct Gorilla;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for Gorilla {
    type State = GorillaState;
    type Seed = GorillaState;
    type Journey = GorillaJourneyTemplate;
}
pub struct Tiger;

#[jungle::animal(id = 2, generation = 0)]
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
    assert_eq!(gorilla_request.0.into_input(), 12);
    let (gorilla_state, gorilla_emitted) = <BoundFlowStep<
        Gorilla,
        CoreEnergyStep<Sleep, GorillaCoreCarrier>,
    > as Waiting>::accept((gorilla_state, Ok(20), ()));
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
    assert_eq!(tiger_request.0.into_input(), 10);
    let (tiger_state, tiger_emitted) = <BoundFlowStep<
        Tiger,
        CoreEnergyStep<Sleep, TigerCoreCarrier>,
    > as Waiting>::accept((tiger_state, Ok(15), ()));
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

    let mut tiger_emitted: Vec<SerializedTag> = Vec::new();
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
        let emitted = match step % 3 {
            0 => {
                let either = tiger
                    .complete::<i32, (), Either<i32, i32>>(Ok(completion))
                    .expect("tiger completion should advance");
                match either {
                    Either::Left(value) | Either::Right(value) => SerializedTag::EitherInt(value),
                }
            }
            1 | 2 => {
                let value: i32 = tiger
                    .complete(Ok::<i32, ()>(completion))
                    .expect("tiger completion should advance");
                SerializedTag::Int(value)
            }
            _ => unreachable!(),
        };
        tiger_emitted.push(emitted);
    }
    assert_eq!(
        tiger_emitted,
        vec![
            SerializedTag::EitherInt(9),
            SerializedTag::Int(10),
            SerializedTag::Int(11),
            SerializedTag::EitherInt(23),
            SerializedTag::Int(24),
            SerializedTag::Int(25),
            SerializedTag::EitherInt(51),
            SerializedTag::Int(52),
            SerializedTag::Int(53),
            SerializedTag::EitherInt(107),
        ]
    );
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 107);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 98);
}

#[derive(Debug, PartialEq, Eq)]
enum SerializedTag {
    Int(i32),
    EitherInt(i32),
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
        Either::Left((_state, request)) => assert_eq!(request.0.into_input(), 0),
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
        Either::Right((_state, request)) => assert_eq!(request.0.into_input(), 0),
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
    assert_eq!(emitted.len(), 10);
    assert!(tiger.is_complete());

    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 107);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 98);
}
