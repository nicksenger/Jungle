use futures::channel::mpsc;
use jungle_sdk::core::Jungle as _;
use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::{Animals, Effects, Optic};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct Eat;

#[jungle::effect(id = 0)]
impl<J> Effect<J> for Eat {
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
pub struct Sleep;

#[jungle::effect(id = 1)]
impl<J> Effect<J> for Sleep {
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
pub struct Forage;

#[jungle::effect(id = 2)]
impl<J> Effect<J> for Forage {
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
pub struct Drink;

#[jungle::effect(id = 3)]
impl<J> Effect<J> for Drink {
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
pub struct Hunt;

#[jungle::effect(id = 4)]
impl<J> Effect<J> for Hunt {
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
pub struct Flee;

#[jungle::effect(id = 5)]
impl<J> Effect<J> for Flee {
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
pub struct BasicNeeds(Eat, Sleep, Forage, Drink);

#[derive(Effects)]
pub struct Predation(Hunt);

#[derive(Effects)]
pub struct Predator(BasicNeeds, Predation);

#[derive(Effects)]
pub struct Prey(BasicNeeds, Flee);

#[derive(Default, Serialize, Deserialize)]
pub struct SharedState;
impl From<&Zoo> for SharedState {
    fn from(_value: &Zoo) -> Self {
        Self
    }
}

pub struct UnitOkStep<A>(PhantomData<fn() -> A>);
impl<T, A> BoundAction<T> for UnitOkStep<A>
where
    T: Animal,
    A: EffectSchema<In = (), Out = (), Err = ()>,
{
    type Effect = A;
    type Aspect = Identity;
    type Input = ();
    type Output = ();
    type Carry = ();

    fn emit(_state: &T::State, _input: Self::Input) -> A::In {}

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
    }

    fn absorb(_state: &mut T::State, output: EffectCompletion<A>) -> Self::Output {
        output.expect("workflow effect should succeed");
    }
}

pub struct UnitOkSpec<E>(PhantomData<fn() -> E>);
impl<E> Action for UnitOkSpec<E>
where
    E: EffectSchema<In = (), Out = (), Err = ()>,
{
    type Effect = E;
    type Input = ();
    type Output = ();
    type Carry = ();
    type Bind<A: Animal> = UnitOkStep<E>;
}

type UUnitStep<E> = Step<UnitOkSpec<E>>;

#[derive(Flow)]
pub struct PreyWorkflowTemplate(
    UUnitStep<Eat>,
    UUnitStep<Sleep>,
    UUnitStep<Forage>,
    UUnitStep<Drink>,
    UUnitStep<Flee>,
);

#[derive(Flow)]
pub struct PredatorWorkflowTemplate(
    UUnitStep<Eat>,
    UUnitStep<Sleep>,
    UUnitStep<Forage>,
    UUnitStep<Drink>,
    UUnitStep<Hunt>,
);

pub struct Gorilla;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for Gorilla {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PreyWorkflowTemplate;
}
pub struct Chimpanzee;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for Chimpanzee {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PreyWorkflowTemplate;
}
pub struct Tiger;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for Tiger {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PredatorWorkflowTemplate;
}
pub struct Jaguar;

#[jungle::animal(id = 3, generation = 0)]
impl Animal for Jaguar {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PredatorWorkflowTemplate;
}
pub struct Anaconda;

#[jungle::animal(id = 4, generation = 0)]
impl Animal for Anaconda {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PredatorWorkflowTemplate;
}
pub struct Hippo;

#[jungle::animal(id = 5, generation = 0)]
impl Animal for Hippo {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PreyWorkflowTemplate;
}
pub struct Elephant;

#[jungle::animal(id = 6, generation = 0)]
impl Animal for Elephant {
    type State = SharedState;
    type Seed = SharedState;
    type Journey = PreyWorkflowTemplate;
}

#[derive(Animals)]
pub struct Apes(Gorilla, Chimpanzee);

#[derive(Animals)]
pub struct Cats(Tiger, Jaguar);

#[derive(Animals)]
pub struct Predators(Cats, Anaconda);

#[derive(Animals)]
pub struct AllAnimals(Cats, Apes, Anaconda, Hippo, Elephant);

#[derive(Effects)]
pub struct AllEffects(Predator, Prey);

pub struct Zoo;
impl Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = AllAnimals;
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerState(i32);

impl From<&RunnerZoo> for RunnerState {
    fn from(_value: &RunnerZoo) -> Self {
        Self(0)
    }
}

impl From<RunnerState> for () {
    fn from(_value: RunnerState) -> Self {}
}

pub struct RunnerStepOneEffect;
#[jungle::effect(id = 14)]
impl<J> Effect<J> for RunnerStepOneEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(2))
    }
}

pub struct RunnerStepTwoEffect;
#[jungle::effect(id = 15)]
impl<J> Effect<J> for RunnerStepTwoEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

pub struct SlowRunnerEffect;
#[jungle::effect(id = 99)]
impl<J> Effect<J> for SlowRunnerEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            Ok(1)
        }
    }
}

pub struct RunnerKeepGoing;
impl LoopCondition<RunnerState> for RunnerKeepGoing {
    type Arg = ();

    fn should_continue(state: &RunnerState) -> bool {
        state.0 < 4
    }
}

pub struct RunnerUseStepOne;
impl jungle_sdk::types::Predicate<(RunnerState, ())> for RunnerUseStepOne {
    fn eval((state, _): &(RunnerState, ())) -> bool {
        state.0 % 2 == 0
    }
}

pub struct RunnerStepOneSpec;
#[jungle::action]
impl Action for RunnerStepOneSpec {
    type Effect = RunnerStepOneEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RunnerState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut RunnerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.0 += output.expect("runner step one should succeed");
    }
}

pub struct RunnerStepTwoSpec;
#[jungle::action]
impl Action for RunnerStepTwoSpec {
    type Effect = RunnerStepTwoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RunnerState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut RunnerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.0 += output.expect("runner step two should succeed");
    }
}

pub struct SlowRunnerStepSpec;
#[jungle::action]
impl Action for SlowRunnerStepSpec {
    type Effect = SlowRunnerEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RunnerState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut RunnerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.0 += output.expect("slow runner step should succeed");
    }
}

#[derive(Flow)]
pub struct RunnerJourneyTemplate(
    While<
        RunnerKeepGoing,
        jungle_sdk::types::Conditional<
            RunnerUseStepOne,
            Step<RunnerStepOneSpec>,
            Step<RunnerStepTwoSpec>,
        >,
    >,
);

#[derive(Flow)]
pub struct SlowRunnerJourneyTemplate(Step<SlowRunnerStepSpec>);

pub struct RunnerAnimal;

#[jungle::animal(id = 16, generation = 0)]
impl Animal for RunnerAnimal {
    type State = RunnerState;
    type Seed = RunnerState;
    type Journey = RunnerJourneyTemplate;
}

pub struct SlowRunnerAnimal;

#[jungle::animal(id = 17, generation = 0)]
impl Animal for SlowRunnerAnimal {
    type State = RunnerState;
    type Seed = RunnerState;
    type Journey = SlowRunnerJourneyTemplate;
}

#[derive(Animals)]
pub struct RunnerAnimals(RunnerAnimal);

pub struct RunnerZoo;
impl Ecosystem for RunnerZoo {
    const NAME: &'static str = "runner-zoo";
    type Animals = RunnerAnimals;
}

#[derive(Animals)]
pub struct ConcurrentRunnerAnimals(SlowRunnerAnimal);

pub struct ConcurrentRunnerZoo;
impl Ecosystem for ConcurrentRunnerZoo {
    const NAME: &'static str = "concurrent-runner-zoo";
    type Animals = ConcurrentRunnerAnimals;
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
fn animal_state_set() {
    #[derive(Default, Serialize, Deserialize)]
    struct ApeState;
    #[derive(Default, Serialize, Deserialize)]
    struct CatState;

    type StatefulGorillaJourney = PreyWorkflowTemplate;
    type StatefulTigerJourney = PredatorWorkflowTemplate;

    struct StatefulGorilla;

    #[jungle::animal(id = 0, generation = 0)]
    impl Animal for StatefulGorilla {
        type State = ApeState;
        type Seed = ApeState;
        type Journey = StatefulGorillaJourney;
    }
    struct StatefulTiger;

    #[jungle::animal(id = 1, generation = 0)]
    impl Animal for StatefulTiger {
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
pub struct CoreState {
    energy: i32,
    rounds: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorApeState {
    core: CoreState,
    bananas: i32,
    mood: i32,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorCatState {
    core: CoreState,
    stripes: i32,
}

pub struct EatEnergy;
#[jungle::effect(id = 7)]
impl<J> Effect<J> for EatEnergy {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(3))
    }
}

pub struct HuntEnergy;
#[jungle::effect(id = 10)]
impl<J> Effect<J> for HuntEnergy {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(4))
    }
}

pub struct RoundAdvance;
#[jungle::effect(id = 13)]
impl<J> Effect<J> for RoundAdvance {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

pub struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);
impl<T, Focus, A> BoundAction<T> for AddI32<Focus, A>
where
    T: Animal,
    Focus: jungle_sdk::types::Aspect<T::State, Focus = i32>,
    A: EffectSchema<In = i32, Out = i32, Err = ()>,
{
    type Effect = A;
    type Aspect = Focus;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(value: &i32, _input: Self::Input) -> Self::Input {
        *value
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<T>>::Aspect as StateCarrier<T::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<T>>::emit(view, input), ())
    }

    fn absorb(value: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let delta = output.expect("add i32 step should succeed");
        *value += delta;
        *value
    }
}

pub struct ApeRoundCarrier;
impl StateCarrier<ExecutorApeState> for ApeRoundCarrier {
    type Focus = i32;

    fn focus(state: &mut ExecutorApeState) -> &mut Self::Focus {
        &mut state.core.rounds
    }
}

pub struct TigerEnergyCarrier;
impl StateCarrier<ExecutorCatState> for TigerEnergyCarrier {
    type Focus = i32;

    fn focus(state: &mut ExecutorCatState) -> &mut Self::Focus {
        &mut state.core.energy
    }
}

type ApeRoundTask = AddI32<ApeRoundCarrier, RoundAdvance>;
type TigerHuntTask = AddI32<TigerEnergyCarrier, HuntEnergy>;
type TigerEatTask = AddI32<TigerEnergyCarrier, EatEnergy>;

pub struct ApeRoundTaskSpec;
#[jungle::action(bind = ApeRoundTask)]
impl Action for ApeRoundTaskSpec {
    type Effect = RoundAdvance;
    type Input = i32;
    type Output = i32;
}

pub struct TigerHuntTaskSpec;
#[jungle::action(bind = TigerHuntTask)]
impl Action for TigerHuntTaskSpec {
    type Effect = HuntEnergy;
    type Input = i32;
    type Output = i32;
}

pub struct TigerEatTaskSpec;
#[jungle::action(bind = TigerEatTask)]
impl Action for TigerEatTaskSpec {
    type Effect = EatEnergy;
    type Input = i32;
    type Output = i32;
}

pub struct ApeKeepRunning;
impl LoopCondition<ExecutorApeState> for ApeKeepRunning {
    type Arg = i32;

    fn should_continue(state: &ExecutorApeState) -> bool {
        state.core.rounds < 4
    }
}

pub struct TigerKeepRunning;
impl LoopCondition<ExecutorCatState> for TigerKeepRunning {
    type Arg = i32;

    fn should_continue(state: &ExecutorCatState) -> bool {
        state.core.energy < 15
    }
}

pub struct TigerChooseHunt;
impl jungle_sdk::types::Predicate<(ExecutorCatState, i32)> for TigerChooseHunt {
    fn eval((state, _): &(ExecutorCatState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

#[derive(Flow)]
pub struct WorkflowGorillaJourneyTemplate(While<ApeKeepRunning, Step<ApeRoundTaskSpec>>);

#[derive(Flow)]
pub struct WorkflowTigerJourneyTemplate(
    While<
        TigerKeepRunning,
        jungle_sdk::types::Conditional<
            TigerChooseHunt,
            Step<TigerHuntTaskSpec>,
            Step<TigerEatTaskSpec>,
        >,
    >,
);

pub struct WorkflowGorilla;

#[jungle::animal(id = 11, generation = 0)]
impl Animal for WorkflowGorilla {
    type State = ExecutorApeState;
    type Seed = ExecutorApeState;
    type Journey = WorkflowGorillaJourneyTemplate;
}
pub struct WorkflowTiger;

#[jungle::animal(id = 12, generation = 0)]
impl Animal for WorkflowTiger {
    type State = ExecutorCatState;
    type Seed = ExecutorCatState;
    type Journey = WorkflowTigerJourneyTemplate;
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

#[tokio::test]
async fn jungle_worker_can_run_multiple_journeys_in_parallel_when_configured() {
    use jungle_sdk::client::MockClient;
    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::types::Work;
    use std::time::Duration;

    let history_calls = Arc::new(AtomicUsize::new(0));
    let flow_complete_calls = Arc::new(AtomicUsize::new(0));
    let history_calls_at_first_complete = Arc::new(AtomicUsize::new(usize::MAX));
    let poll_idx = Arc::new(AtomicUsize::new(0));
    let seed = postcard::to_allocvec(&RunnerState(0)).expect("runner seed should serialize");
    let first_journey_id = Uuid::from_u128(201);
    let second_journey_id = Uuid::from_u128(202);

    let client = MockClient::builder()
        .on_poll_work({
            let poll_idx = Arc::clone(&poll_idx);
            let seed = seed.clone();
            move |_| {
                let poll_idx = Arc::clone(&poll_idx);
                let seed = seed.clone();
                async move {
                    match poll_idx.fetch_add(1, Ordering::Relaxed) {
                        0 => Ok(Some(Work::StartJourney {
                            journey_id: first_journey_id,
                            animal_id: 17,
                            generation: 0,
                            seed: seed.clone(),
                        })),
                        1 => Ok(Some(Work::StartJourney {
                            journey_id: second_journey_id,
                            animal_id: 17,
                            generation: 0,
                            seed: seed.clone(),
                        })),
                        _ => Ok(None),
                    }
                }
            }
        })
        .on_journey_history({
            let history_calls = Arc::clone(&history_calls);
            move |_| {
                let history_calls = Arc::clone(&history_calls);
                async move {
                    history_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(Vec::new())
                }
            }
        })
        .on_flow_complete({
            let flow_complete_calls = Arc::clone(&flow_complete_calls);
            let history_calls = Arc::clone(&history_calls);
            let history_calls_at_first_complete = Arc::clone(&history_calls_at_first_complete);
            move |_| {
                let flow_complete_calls = Arc::clone(&flow_complete_calls);
                let history_calls = Arc::clone(&history_calls);
                let history_calls_at_first_complete = Arc::clone(&history_calls_at_first_complete);
                async move {
                    let prior = flow_complete_calls.fetch_add(1, Ordering::Relaxed);
                    if prior == 0 {
                        let _ = history_calls_at_first_complete.compare_exchange(
                            usize::MAX,
                            history_calls.load(Ordering::Relaxed),
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        );
                    }
                    Ok(())
                }
            }
        })
        .build();

    let worker = JungleWorker::new(ConcurrentRunnerZoo, client).with_max_in_flight_journeys(2);
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let finished = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if flow_complete_calls.load(Ordering::Relaxed) >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    if finished.is_err() {
        panic!(
            "worker did not complete both journeys: history_calls={}, flow_complete_calls={}",
            history_calls.load(Ordering::Relaxed),
            flow_complete_calls.load(Ordering::Relaxed),
        );
    }

    assert_eq!(flow_complete_calls.load(Ordering::Relaxed), 2);
    assert_eq!(history_calls.load(Ordering::Relaxed), 2);
    assert_eq!(
        history_calls_at_first_complete.load(Ordering::Relaxed),
        2,
        "second journey should be claimed before first completion when parallelism > 1",
    );

    worker_handle.abort();
    let _ = worker_handle.await;
}
