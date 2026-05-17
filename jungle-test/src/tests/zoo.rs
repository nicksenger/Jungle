use futures::channel::mpsc;
use jungle_sdk::animal;
use jungle_sdk::core::Jungle as _;
use jungle_sdk::effect;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    Act, ActionSpec, Animal, AnimalEffectSet, AnimalSet, AnimalStates, Ecosystem, EffectCompletion,
    EffectExec, EffectSchema, EffectSet, Identity, LoopCondition, StateCarrier, BoundStep, UStep, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{
    U0, U1, U10, U11, U12, U13, U14, U15, U16, U2, U3, U4, U5, U6, U7,
};
use jungle_sdk::{Animals, Effects, Optic};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

struct Eat;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Eat {
    type Id = Id<U0>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct Sleep;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Sleep {
    type Id = Id<U1>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct Forage;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Forage {
    type Id = Id<U2>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct Drink;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Drink {
    type Id = Id<U3>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct Hunt;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Hunt {
    type Id = Id<U4>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct Flee;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for Flee {
    type Id = Id<U5>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

#[derive(Effects)]
struct BasicNeeds(Eat, Sleep, Forage, Drink);

#[derive(Effects)]
struct Predation(Hunt);

#[derive(Effects)]
struct Predator(BasicNeeds, Predation);

#[derive(Effects)]
struct Prey(BasicNeeds, Flee);

#[derive(Default, Serialize, Deserialize)]
struct SharedState;
impl From<&Zoo> for SharedState {
    fn from(_value: &Zoo) -> Self {
        Self
    }
}

struct UnitOkStep<A>(PhantomData<fn() -> A>);
impl<T, A> Act<T> for UnitOkStep<A>
where
    T: Animal,
    A: EffectSchema<In = (), Out = (), Err = ()>,
{
    type Effect = A;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &T::State, _input: Self::Input) -> A::In {}

    fn absorb(_state: &mut T::State, output: EffectCompletion<A>) -> Self::Output {
        output.expect("workflow effect should succeed");
    }
}

struct UnitOkSpec<E>(PhantomData<fn() -> E>);
impl<E> ActionSpec for UnitOkSpec<E>
where
    E: EffectSchema<In = (), Out = (), Err = ()>,
{
    type Effect = E;
    type Input = ();
    type Output = ();
    type Act<A: Animal> = UnitOkStep<E>;
}

type UUnitStep<E> = UStep<UnitOkSpec<E>>;

#[derive(jungle_sdk::FlowTemplate)]
struct PreyWorkflowTemplate(
    UUnitStep<Eat>,
    UUnitStep<Sleep>,
    UUnitStep<Forage>,
    UUnitStep<Drink>,
    UUnitStep<Flee>,
);

#[derive(jungle_sdk::FlowTemplate)]
struct PredatorWorkflowTemplate(
    UUnitStep<Eat>,
    UUnitStep<Sleep>,
    UUnitStep<Forage>,
    UUnitStep<Drink>,
    UUnitStep<Hunt>,
);

#[derive(jungle_sdk::Journey)]
struct GorillaJourney(
    BoundStep<Gorilla, UnitOkStep<Eat>>,
    BoundStep<Gorilla, UnitOkStep<Sleep>>,
    BoundStep<Gorilla, UnitOkStep<Forage>>,
    BoundStep<Gorilla, UnitOkStep<Drink>>,
    BoundStep<Gorilla, UnitOkStep<Flee>>,
);

#[derive(jungle_sdk::Journey)]
struct ChimpanzeeJourney(
    BoundStep<Chimpanzee, UnitOkStep<Eat>>,
    BoundStep<Chimpanzee, UnitOkStep<Sleep>>,
    BoundStep<Chimpanzee, UnitOkStep<Forage>>,
    BoundStep<Chimpanzee, UnitOkStep<Drink>>,
    BoundStep<Chimpanzee, UnitOkStep<Flee>>,
);

#[derive(jungle_sdk::Journey)]
struct TigerJourney(
    BoundStep<Tiger, UnitOkStep<Eat>>,
    BoundStep<Tiger, UnitOkStep<Sleep>>,
    BoundStep<Tiger, UnitOkStep<Forage>>,
    BoundStep<Tiger, UnitOkStep<Drink>>,
    BoundStep<Tiger, UnitOkStep<Hunt>>,
);

#[derive(jungle_sdk::Journey)]
struct JaguarJourney(
    BoundStep<Jaguar, UnitOkStep<Eat>>,
    BoundStep<Jaguar, UnitOkStep<Sleep>>,
    BoundStep<Jaguar, UnitOkStep<Forage>>,
    BoundStep<Jaguar, UnitOkStep<Drink>>,
    BoundStep<Jaguar, UnitOkStep<Hunt>>,
);

#[derive(jungle_sdk::Journey)]
struct AnacondaJourney(
    BoundStep<Anaconda, UnitOkStep<Eat>>,
    BoundStep<Anaconda, UnitOkStep<Sleep>>,
    BoundStep<Anaconda, UnitOkStep<Forage>>,
    BoundStep<Anaconda, UnitOkStep<Drink>>,
    BoundStep<Anaconda, UnitOkStep<Hunt>>,
);

#[derive(jungle_sdk::Journey)]
struct HippoJourney(
    BoundStep<Hippo, UnitOkStep<Eat>>,
    BoundStep<Hippo, UnitOkStep<Sleep>>,
    BoundStep<Hippo, UnitOkStep<Forage>>,
    BoundStep<Hippo, UnitOkStep<Drink>>,
    BoundStep<Hippo, UnitOkStep<Flee>>,
);

#[derive(jungle_sdk::Journey)]
struct ElephantJourney(
    BoundStep<Elephant, UnitOkStep<Eat>>,
    BoundStep<Elephant, UnitOkStep<Sleep>>,
    BoundStep<Elephant, UnitOkStep<Forage>>,
    BoundStep<Elephant, UnitOkStep<Drink>>,
    BoundStep<Elephant, UnitOkStep<Flee>>,
);

struct Gorilla;

#[animal]
impl Animal for Gorilla {
    type Id = Id<U0>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = GorillaJourney;
}
struct Chimpanzee;

#[animal]
impl Animal for Chimpanzee {
    type Id = Id<U1>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = ChimpanzeeJourney;
}
struct Tiger;

#[animal]
impl Animal for Tiger {
    type Id = Id<U2>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = TigerJourney;
}
struct Jaguar;

#[animal]
impl Animal for Jaguar {
    type Id = Id<U3>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = JaguarJourney;
}
struct Anaconda;

#[animal]
impl Animal for Anaconda {
    type Id = Id<U4>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = AnacondaJourney;
}
struct Hippo;

#[animal]
impl Animal for Hippo {
    type Id = Id<U5>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = HippoJourney;
}
struct Elephant;

#[animal]
impl Animal for Elephant {
    type Id = Id<U6>;
    type Generation = U0;
    type State = SharedState;
    type Seed = SharedState;
    type Journey = ElephantJourney;
}

#[derive(Animals)]
struct Apes(Gorilla, Chimpanzee);

#[derive(Animals)]
struct Cats(Tiger, Jaguar);

#[derive(Animals)]
struct Predators(Cats, Anaconda);

#[derive(Animals)]
struct AllAnimals(Cats, Apes, Anaconda, Hippo, Elephant);

#[derive(Effects)]
struct AllEffects(Predator, Prey);

struct Zoo;
impl Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = AllAnimals;
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RunnerState(i32);

impl From<&RunnerZoo> for RunnerState {
    fn from(_value: &RunnerZoo) -> Self {
        Self(0)
    }
}

impl From<RunnerState> for () {
    fn from(_value: RunnerState) -> Self {}
}

struct RunnerStepOneEffect;
impl EffectSchema for RunnerStepOneEffect {
    type Id = Id<U14>;
    type In = ();
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for RunnerStepOneEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(2))
    }
}

struct RunnerStepTwoEffect;
impl EffectSchema for RunnerStepTwoEffect {
    type Id = Id<U15>;
    type In = ();
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for RunnerStepTwoEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

struct RunnerStepOne;
impl Act<RunnerAnimal> for RunnerStepOne {
    type Effect = RunnerStepOneEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &RunnerState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut RunnerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.0 += output.expect("runner step one should succeed");
    }
}

struct RunnerStepTwo;
impl Act<RunnerAnimal> for RunnerStepTwo {
    type Effect = RunnerStepTwoEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &RunnerState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut RunnerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.0 += output.expect("runner step two should succeed");
    }
}

struct RunnerKeepGoing;
impl LoopCondition<RunnerState> for RunnerKeepGoing {
    type Arg = ();

    fn should_continue(state: &RunnerState) -> bool {
        state.0 < 4
    }
}

struct RunnerUseStepOne;
impl jungle_sdk::types::Condition<(RunnerState, ())> for RunnerUseStepOne {
    fn choose((state, _): &(RunnerState, ())) -> bool {
        state.0 % 2 == 0
    }
}

type RunnerJourney = While<
    RunnerKeepGoing,
    jungle_sdk::types::Conditional<
        RunnerUseStepOne,
        BoundStep<RunnerAnimal, RunnerStepOne>,
        BoundStep<RunnerAnimal, RunnerStepTwo>,
    >,
>;

struct RunnerAnimal;

#[animal]
impl Animal for RunnerAnimal {
    type Id = Id<U16>;
    type Generation = U0;
    type State = RunnerState;
    type Seed = RunnerState;
    type Journey = RunnerJourney;
}

#[derive(Animals)]
struct RunnerAnimals(RunnerAnimal);

struct RunnerZoo;
impl Ecosystem for RunnerZoo {
    const NAME: &'static str = "runner-zoo";
    type Animals = RunnerAnimals;
}

#[test]
fn composite_effects() {
    type BasicList = list![Eat, Sleep, Forage, Drink];
    assert_type_eq!(EffectSet<BasicNeeds>, BasicList);

    type PredatorList = list![Eat, Sleep, Forage, Drink, Hunt];
    assert_type_eq!(EffectSet<Predator>, PredatorList);
}

#[test]
fn composite_animals() {
    type ApeList = list![Gorilla, Chimpanzee];
    assert_type_eq!(AnimalSet<Apes>, ApeList);

    type PredatorList = list![Tiger, Jaguar, Anaconda];
    assert_type_eq!(AnimalSet<Predators>, PredatorList);
}

#[test]
fn animal_effect_set() {
    type ApeAnimalEffects = list![Eat, Sleep, Forage, Drink, Flee];
    assert_type_eq!(AnimalEffectSet<Apes>, ApeAnimalEffects);

    type AllAnimalEffects = list![Eat, Sleep, Forage, Drink, Hunt, Flee];
    assert_type_eq!(AnimalEffectSet<AllAnimals>, AllAnimalEffects);
}

#[test]
fn animal_state_set() {
    #[derive(Default, Serialize, Deserialize)]
    struct ApeState;
    #[derive(Default, Serialize, Deserialize)]
    struct CatState;

    type StatefulGorillaJourney =
        <PreyWorkflowTemplate as jungle_sdk::types::BindAnimal<StatefulGorilla>>::Bound;
    type StatefulTigerJourney =
        <PredatorWorkflowTemplate as jungle_sdk::types::BindAnimal<StatefulTiger>>::Bound;

    struct StatefulGorilla;

    #[animal]
    impl Animal for StatefulGorilla {
        type Id = Id<U0>;
        type Generation = U0;
        type State = ApeState;
        type Seed = ApeState;
        type Journey = StatefulGorillaJourney;
    }
    struct StatefulTiger;

    #[animal]
    impl Animal for StatefulTiger {
        type Id = Id<U1>;
        type Generation = U0;
        type State = CatState;
        type Seed = CatState;
        type Journey = StatefulTigerJourney;
    }

    #[derive(Animals)]
    struct StatefulAnimals(StatefulGorilla, StatefulTiger);

    type StatefulAnimalStates = list![ApeState, CatState];
    assert_type_eq!(AnimalStates<StatefulAnimals>, StatefulAnimalStates);
}

#[test]
fn jungle_impl() {
    let zoo = Zoo;
    let _jungle_fut = zoo.manifest();
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CoreState {
    energy: i32,
    rounds: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ExecutorApeState {
    core: CoreState,
    bananas: i32,
    mood: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ExecutorCatState {
    core: CoreState,
    stripes: i32,
}

struct EatEnergy;
impl EffectSchema for EatEnergy {
    type Id = Id<U7>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for EatEnergy {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(3))
    }
}

struct HuntEnergy;
impl EffectSchema for HuntEnergy {
    type Id = Id<U10>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for HuntEnergy {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(4))
    }
}

struct RoundAdvance;
impl EffectSchema for RoundAdvance {
    type Id = Id<U13>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for RoundAdvance {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);
impl<T, Focus, A> Act<T> for AddI32<Focus, A>
where
    T: Animal,
    Focus: jungle_sdk::types::Aspect<T::State, View = i32>,
    A: EffectSchema<In = i32, Out = i32, Err = ()>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;

    fn emit(value: &i32, _input: Self::Input) -> Self::Input {
        *value
    }

    fn absorb(value: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let delta = output.expect("add i32 step should succeed");
        *value += delta;
        *value
    }
}

struct ApeRoundCarrier;
impl StateCarrier<ExecutorApeState> for ApeRoundCarrier {
    type View = i32;

    fn view<'a>(state: &'a mut ExecutorApeState) -> &'a mut Self::View {
        &mut state.core.rounds
    }
}

struct TigerEnergyCarrier;
impl StateCarrier<ExecutorCatState> for TigerEnergyCarrier {
    type View = i32;

    fn view<'a>(state: &'a mut ExecutorCatState) -> &'a mut Self::View {
        &mut state.core.energy
    }
}

type ApeRoundTask = AddI32<ApeRoundCarrier, RoundAdvance>;
type TigerHuntTask = AddI32<TigerEnergyCarrier, HuntEnergy>;
type TigerEatTask = AddI32<TigerEnergyCarrier, EatEnergy>;

struct ApeKeepRunning;
impl LoopCondition<ExecutorApeState> for ApeKeepRunning {
    type Arg = i32;

    fn should_continue(state: &ExecutorApeState) -> bool {
        state.core.rounds < 4
    }
}

struct TigerKeepRunning;
impl LoopCondition<ExecutorCatState> for TigerKeepRunning {
    type Arg = i32;

    fn should_continue(state: &ExecutorCatState) -> bool {
        state.core.energy < 15
    }
}

struct TigerChooseHunt;
impl jungle_sdk::types::Condition<(ExecutorCatState, i32)> for TigerChooseHunt {
    fn choose((state, _): &(ExecutorCatState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

type WorkflowGorillaJourney = While<ApeKeepRunning, BoundStep<WorkflowGorilla, ApeRoundTask>>;
type WorkflowTigerJourney = While<
    TigerKeepRunning,
    jungle_sdk::types::Conditional<
        TigerChooseHunt,
        BoundStep<WorkflowTiger, TigerHuntTask>,
        BoundStep<WorkflowTiger, TigerEatTask>,
    >,
>;

struct WorkflowGorilla;

#[animal]
impl Animal for WorkflowGorilla {
    type Id = Id<U11>;
    type Generation = U0;
    type State = ExecutorApeState;
    type Seed = ExecutorApeState;
    type Journey = WorkflowGorillaJourney;
}
struct WorkflowTiger;

#[animal]
impl Animal for WorkflowTiger {
    type Id = Id<U12>;
    type Generation = U0;
    type State = ExecutorCatState;
    type Seed = ExecutorCatState;
    type Journey = WorkflowTigerJourney;
}

#[tokio::test]
async fn jungle_executor_runs_effects_with_ecosystem_dependency() {
    use jungle_sdk::core::JungleExecutor;

    let mut gorilla = JungleExecutor::<Zoo, WorkflowGorilla>::new(
        Zoo,
        ExecutorApeState {
            core: CoreState {
                energy: 5,
                rounds: 0,
            },
            bananas: 4,
            mood: 2,
        },
    );
    let mut gorilla_requests = Vec::new();
    loop {
        let request = match gorilla.next_executable_request(1i32) {
            Ok(request) => request,
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("gorilla request should build: {err}"),
        };
        let request_input: i32 = request
            .deserialize_request()
            .expect("gorilla request should deserialize");
        gorilla_requests.push(request_input);
        let completion = request.run().await.expect("gorilla effect should execute");
        let _emitted = gorilla
            .complete_serialized(completion)
            .expect("gorilla completion should process");
    }
    assert_eq!(gorilla_requests, vec![0, 1, 2, 3]);
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 5);
    assert_eq!(gorilla_state.core.rounds, 4);
    assert_eq!(gorilla_state.bananas, 4);
    assert_eq!(gorilla_state.mood, 2);

    let mut tiger = JungleExecutor::<Zoo, WorkflowTiger>::new(
        Zoo,
        ExecutorCatState {
            core: CoreState {
                energy: 6,
                rounds: 0,
            },
            stripes: 8,
        },
    );
    let mut tiger_requests = Vec::new();
    loop {
        let request = match tiger.next_executable_request(1i32) {
            Ok(request) => request,
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("tiger request should build: {err}"),
        };
        let request_input: i32 = request
            .deserialize_request()
            .expect("tiger request should deserialize");
        tiger_requests.push(request_input);
        let completion = request.run().await.expect("tiger effect should execute");
        let _emitted = tiger
            .complete_serialized(completion)
            .expect("tiger completion should process");
    }
    assert_eq!(tiger_requests, vec![6, 10, 14]);
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 18);
    assert_eq!(tiger_state.core.rounds, 0);
    assert_eq!(tiger_state.stripes, 8);

    let mut tiger_odd = JungleExecutor::<Zoo, WorkflowTiger>::new(
        Zoo,
        ExecutorCatState {
            core: CoreState {
                energy: 6,
                rounds: 0,
            },
            stripes: 7,
        },
    );
    let mut tiger_odd_requests = Vec::new();
    loop {
        let request = match tiger_odd.next_executable_request(1i32) {
            Ok(request) => request,
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("tiger odd request should build: {err}"),
        };
        let request_input: i32 = request
            .deserialize_request()
            .expect("tiger odd request should deserialize");
        tiger_odd_requests.push(request_input);
        let completion = request
            .run()
            .await
            .expect("tiger odd effect should execute");
        let _emitted = tiger_odd
            .complete_serialized(completion)
            .expect("tiger odd completion should process");
    }
    assert_eq!(tiger_odd_requests, vec![6, 9, 12]);
    let tiger_odd_state = tiger_odd.into_state();
    assert_eq!(tiger_odd_state.core.energy, 15);
    assert_eq!(tiger_odd_state.core.rounds, 0);
    assert_eq!(tiger_odd_state.stripes, 7);
}

#[tokio::test]
async fn jungle_executor_exposes_state_during_progression() {
    use jungle_sdk::core::JungleExecutor;

    let mut gorilla = JungleExecutor::<Zoo, WorkflowGorilla>::new(
        Zoo,
        ExecutorApeState {
            core: CoreState {
                energy: 5,
                rounds: 0,
            },
            bananas: 4,
            mood: 2,
        },
    );

    loop {
        let rounds_before = gorilla.state().core.rounds;
        let request = match gorilla.next_executable_request(1i32) {
            Ok(request) => request,
            Err(jungle_sdk::types::ExecutorError::Complete) => break,
            Err(err) => panic!("gorilla request should build: {err}"),
        };
        let completion = request.run().await.expect("gorilla effect should execute");
        let _emitted = gorilla
            .complete_serialized(completion)
            .expect("gorilla completion should process");
        assert_eq!(gorilla.state().core.rounds, rounds_before + 1);
        gorilla.state_mut().bananas += 2;
    }

    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.rounds, 4);
    assert_eq!(gorilla_state.bananas, 12);
}

#[tokio::test]
async fn jungle_runner_spawns_and_completes_animal_flows() {
    use jungle_sdk::client::{MockClient, RunnerChannelTx};
    use jungle_sdk::core::JungleRunner;

    let runner = JungleRunner::new(RunnerZoo);
    let input_calls = Arc::new(AtomicUsize::new(0));
    let success_calls = Arc::new(AtomicUsize::new(0));
    let failure_calls = Arc::new(AtomicUsize::new(0));
    let client = MockClient::builder()
        .on_effect_input({
            let input_calls = Arc::clone(&input_calls);
            move |_, _| {
                let input_calls = Arc::clone(&input_calls);
                async move {
                    input_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_effect_success_output({
            let success_calls = Arc::clone(&success_calls);
            move |_, _| {
                let success_calls = Arc::clone(&success_calls);
                async move {
                    success_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_effect_failure_output({
            let failure_calls = Arc::clone(&failure_calls);
            move |_, _| {
                let failure_calls = Arc::clone(&failure_calls);
                async move {
                    failure_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .build();
    let (tx, rx): (RunnerChannelTx, _) = mpsc::channel(32);
    let resolver = tokio::spawn(async move {
        client.serve_runner_channel(rx).await;
    });

    let (first, second, third) = tokio::join!(
        runner.spawn::<RunnerAnimal>(RunnerState(0), Uuid::from_u128(1), tx.clone()),
        runner.spawn::<RunnerAnimal>(RunnerState(3), Uuid::from_u128(2), tx.clone()),
        runner.spawn::<RunnerAnimal>(RunnerState(2), Uuid::from_u128(3), tx.clone()),
    );
    drop(tx);
    resolver
        .await
        .expect("runner transport resolver should complete");

    assert_eq!(
        first.expect("first runner flow should complete"),
        RunnerState(4)
    );
    assert_eq!(
        second.expect("second runner flow should complete"),
        RunnerState(4)
    );
    assert_eq!(
        third.expect("third runner flow should complete"),
        RunnerState(4)
    );
    assert_eq!(input_calls.load(Ordering::Relaxed), 4);
    assert_eq!(success_calls.load(Ordering::Relaxed), 4);
    assert_eq!(failure_calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn jungle_worker_polls_and_completes_start_flow_work() {
    use jungle_sdk::client::MockClient;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::types::Work;
    use std::time::Duration;

    let input_calls = Arc::new(AtomicUsize::new(0));
    let success_calls = Arc::new(AtomicUsize::new(0));
    let failure_calls = Arc::new(AtomicUsize::new(0));
    let flow_complete_calls = Arc::new(AtomicUsize::new(0));
    let poll_calls = Arc::new(AtomicUsize::new(0));
    let seed = postcard::to_allocvec(&RunnerState(0)).expect("runner seed should serialize");
    let journey_id = Uuid::from_u128(101);

    let client = MockClient::builder()
        .on_poll_work({
            let poll_calls = Arc::clone(&poll_calls);
            let seed = seed.clone();
            move |_| {
                let poll_calls = Arc::clone(&poll_calls);
                let seed = seed.clone();
                async move {
                    let idx = poll_calls.fetch_add(1, Ordering::Relaxed);
                    if idx == 0 {
                        Ok(Some(Work::StartJourney {
                            journey_id,
                            animal_id: 16,
                            generation: 0,
                            seed,
                        }))
                    } else {
                        Ok(None)
                    }
                }
            }
        })
        .on_effect_input({
            let input_calls = Arc::clone(&input_calls);
            move |_, _| {
                let input_calls = Arc::clone(&input_calls);
                async move {
                    input_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_effect_success_output({
            let success_calls = Arc::clone(&success_calls);
            move |_, _| {
                let success_calls = Arc::clone(&success_calls);
                async move {
                    success_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_effect_failure_output({
            let failure_calls = Arc::clone(&failure_calls);
            move |_, _| {
                let failure_calls = Arc::clone(&failure_calls);
                async move {
                    failure_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_flow_complete({
            let flow_complete_calls = Arc::clone(&flow_complete_calls);
            move |_| {
                let flow_complete_calls = Arc::clone(&flow_complete_calls);
                async move {
                    flow_complete_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .build();

    let worker = JungleWorker::new(RunnerZoo, client);
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let timed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if success_calls.load(Ordering::Relaxed) == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    if timed.is_err() {
        panic!(
            "worker flow should complete: polls={}, inputs={}, successes={}, failures={}",
            poll_calls.load(Ordering::Relaxed),
            input_calls.load(Ordering::Relaxed),
            success_calls.load(Ordering::Relaxed),
            failure_calls.load(Ordering::Relaxed),
        );
    }
    if worker_handle.is_finished() {
        let joined = worker_handle.await;
        panic!("worker should keep polling, got: {joined:?}");
    }

    assert_eq!(input_calls.load(Ordering::Relaxed), 2);
    assert_eq!(success_calls.load(Ordering::Relaxed), 2);
    assert_eq!(failure_calls.load(Ordering::Relaxed), 0);
    assert_eq!(flow_complete_calls.load(Ordering::Relaxed), 1);

    worker_handle.abort();
    let _ = worker_handle.await;
}
