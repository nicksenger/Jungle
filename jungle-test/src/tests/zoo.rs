use futures::channel::mpsc;
use jungle_sdk::core::Jungle as _;
use jungle_sdk::types::{
    Act, Action, ActionCompletion, ActionSet, Animal, AnimalActionSet, AnimalSet, AnimalStates,
    Ecosystem, Identity, Lens, LoopCondition, Step, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};
use jungle_sdk::{Actions, Animals, Flow, Optic};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

action!(Eat, U0, dependency = SharedState);
action!(Sleep, U1, dependency = SharedState);
action!(Forage, U2, dependency = SharedState);
action!(Drink, U3, dependency = SharedState);
action!(Hunt, U4, dependency = SharedState);
action!(Flee, U5, dependency = SharedState);

#[derive(Actions)]
struct BasicNeeds(Eat, Sleep, Forage, Drink);

#[derive(Actions)]
struct Predation(Hunt);

#[derive(Actions)]
struct Predator(BasicNeeds, Predation);

#[derive(Actions)]
struct Prey(BasicNeeds, Flee);

#[derive(serde::Serialize, serde::Deserialize)]
struct SharedState;
impl<T> From<&T> for SharedState {
    fn from(_value: &T) -> Self {
        Self
    }
}

struct UnitOkStep<A>(PhantomData<fn() -> A>);
impl<T, A> Act<T> for UnitOkStep<A>
where
    T: Animal,
    A: Action<In = ()>,
    A: Action<Out = (), Err = ()>,
{
    type Action = A;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &T::State, _input: Self::In) -> A::In {}

    fn absorb(_state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
        output.expect("workflow action should succeed");
    }
}

animal!(Gorilla, U0, GorillaJourney);
animal!(Chimpanzee, U1, ChimpanzeeJourney);
animal!(Tiger, U2, TigerJourney);
animal!(Jaguar, U3, JaguarJourney);
animal!(Anaconda, U4, AnacondaJourney);
animal!(Hippo, U5, HippoJourney);
animal!(Elephant, U6, ElephantJourney);

#[derive(Flow)]
struct GorillaJourney(
    Step<Gorilla, UnitOkStep<Eat>>,
    Step<Gorilla, UnitOkStep<Sleep>>,
    Step<Gorilla, UnitOkStep<Forage>>,
    Step<Gorilla, UnitOkStep<Drink>>,
    Step<Gorilla, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct ChimpanzeeJourney(
    Step<Chimpanzee, UnitOkStep<Eat>>,
    Step<Chimpanzee, UnitOkStep<Sleep>>,
    Step<Chimpanzee, UnitOkStep<Forage>>,
    Step<Chimpanzee, UnitOkStep<Drink>>,
    Step<Chimpanzee, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct TigerJourney(
    Step<Tiger, UnitOkStep<Eat>>,
    Step<Tiger, UnitOkStep<Sleep>>,
    Step<Tiger, UnitOkStep<Forage>>,
    Step<Tiger, UnitOkStep<Drink>>,
    Step<Tiger, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct JaguarJourney(
    Step<Jaguar, UnitOkStep<Eat>>,
    Step<Jaguar, UnitOkStep<Sleep>>,
    Step<Jaguar, UnitOkStep<Forage>>,
    Step<Jaguar, UnitOkStep<Drink>>,
    Step<Jaguar, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct AnacondaJourney(
    Step<Anaconda, UnitOkStep<Eat>>,
    Step<Anaconda, UnitOkStep<Sleep>>,
    Step<Anaconda, UnitOkStep<Forage>>,
    Step<Anaconda, UnitOkStep<Drink>>,
    Step<Anaconda, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct HippoJourney(
    Step<Hippo, UnitOkStep<Eat>>,
    Step<Hippo, UnitOkStep<Sleep>>,
    Step<Hippo, UnitOkStep<Forage>>,
    Step<Hippo, UnitOkStep<Drink>>,
    Step<Hippo, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct ElephantJourney(
    Step<Elephant, UnitOkStep<Eat>>,
    Step<Elephant, UnitOkStep<Sleep>>,
    Step<Elephant, UnitOkStep<Forage>>,
    Step<Elephant, UnitOkStep<Drink>>,
    Step<Elephant, UnitOkStep<Flee>>,
);

#[derive(Animals)]
struct Apes(Gorilla, Chimpanzee);

#[derive(Animals)]
struct Cats(Tiger, Jaguar);

#[derive(Animals)]
struct Predators(Cats, Anaconda);

#[derive(Animals)]
struct AllAnimals(Cats, Apes, Anaconda, Hippo, Elephant);

#[derive(Actions)]
struct AllActions(Predator, Prey);

struct Zoo;
impl Ecosystem for Zoo {
    type Animals = AllAnimals;
}

#[derive(Clone, Copy)]
struct RunnerDependency {
    gain: i32,
}

impl From<&RunnerZoo> for RunnerDependency {
    fn from(_value: &RunnerZoo) -> Self {
        Self { gain: 2 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct RunnerState(i32);

impl From<&RunnerZoo> for RunnerState {
    fn from(_value: &RunnerZoo) -> Self {
        Self(0)
    }
}

struct RunnerStepOneAction;
impl jungle_sdk::types::ActionMember for RunnerStepOneAction {}
impl Action for RunnerStepOneAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U14>;
    type Dependency = RunnerDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.gain))
    }
}

#[derive(Clone, Copy)]
struct RunnerStepTwoDependency {
    gain: i32,
}

impl From<&RunnerZoo> for RunnerStepTwoDependency {
    fn from(_value: &RunnerZoo) -> Self {
        Self { gain: 1 }
    }
}

struct RunnerStepTwoAction;
impl jungle_sdk::types::ActionMember for RunnerStepTwoAction {}
impl Action for RunnerStepTwoAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U15>;
    type Dependency = RunnerStepTwoDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.gain))
    }
}

struct RunnerStepOne;
impl Act<RunnerAnimal> for RunnerStepOne {
    type Action = RunnerStepOneAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &RunnerState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut RunnerState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("runner step one should succeed");
    }
}

struct RunnerStepTwo;
impl Act<RunnerAnimal> for RunnerStepTwo {
    type Action = RunnerStepTwoAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &RunnerState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut RunnerState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("runner step two should succeed");
    }
}

struct RunnerKeepGoing;
impl LoopCondition<RunnerState> for RunnerKeepGoing {
    type CarryIn = ();

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
        Step<RunnerAnimal, RunnerStepOne>,
        Step<RunnerAnimal, RunnerStepTwo>,
    >,
>;

animal!(
    RunnerAnimal,
    jungle_sdk::typosaurus::num::consts::U16,
    RunnerState,
    RunnerJourney
);

#[derive(Animals)]
struct RunnerAnimals(RunnerAnimal);

struct RunnerZoo;
impl Ecosystem for RunnerZoo {
    type Animals = RunnerAnimals;
}

#[test]
fn composite_actions() {
    type BasicList = list![Eat, Sleep, Forage, Drink];
    assert_type_eq!(ActionSet<BasicNeeds>, BasicList);

    type PredatorList = list![Eat, Sleep, Forage, Drink, Hunt];
    assert_type_eq!(ActionSet<Predator>, PredatorList);
}

#[test]
fn composite_animals() {
    type ApeList = list![Gorilla, Chimpanzee];
    assert_type_eq!(AnimalSet<Apes>, ApeList);

    type PredatorList = list![Tiger, Jaguar, Anaconda];
    assert_type_eq!(AnimalSet<Predators>, PredatorList);
}

#[test]
fn animal_action_set() {
    type ApeAnimalActions = list![Eat, Sleep, Forage, Drink, Flee];
    assert_type_eq!(AnimalActionSet<Apes>, ApeAnimalActions);

    type AllAnimalActions = list![Eat, Sleep, Forage, Drink, Hunt, Flee];
    assert_type_eq!(AnimalActionSet<AllAnimals>, AllAnimalActions);
}

#[test]
fn animal_state_set() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct ApeState;
    #[derive(serde::Serialize, serde::Deserialize)]
    struct CatState;

    #[derive(Flow)]
    struct StatefulGorillaJourney(
        Step<StatefulGorilla, UnitOkStep<Eat>>,
        Step<StatefulGorilla, UnitOkStep<Sleep>>,
        Step<StatefulGorilla, UnitOkStep<Forage>>,
        Step<StatefulGorilla, UnitOkStep<Drink>>,
        Step<StatefulGorilla, UnitOkStep<Flee>>,
    );

    #[derive(Flow)]
    struct StatefulTigerJourney(
        Step<StatefulTiger, UnitOkStep<Eat>>,
        Step<StatefulTiger, UnitOkStep<Sleep>>,
        Step<StatefulTiger, UnitOkStep<Forage>>,
        Step<StatefulTiger, UnitOkStep<Drink>>,
        Step<StatefulTiger, UnitOkStep<Hunt>>,
    );

    animal!(StatefulGorilla, U0, ApeState, StatefulGorillaJourney);
    animal!(StatefulTiger, U1, CatState, StatefulTigerJourney);

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

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CoreState {
    energy: i32,
    rounds: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ExecutorApeState {
    core: CoreState,
    bananas: i32,
    mood: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ExecutorCatState {
    core: CoreState,
    stripes: i32,
}

#[derive(Clone, Copy)]
struct EatDependency {
    base_gain: i32,
}

#[derive(Clone, Copy)]
struct HuntDependency {
    gain: i32,
}

#[derive(Clone, Copy)]
struct RoundDependency {
    tick: i32,
}

impl From<&Zoo> for EatDependency {
    fn from(_value: &Zoo) -> Self {
        Self { base_gain: 3 }
    }
}

impl From<&Zoo> for HuntDependency {
    fn from(_value: &Zoo) -> Self {
        Self { gain: 4 }
    }
}

impl From<&Zoo> for RoundDependency {
    fn from(_value: &Zoo) -> Self {
        Self { tick: 1 }
    }
}

struct EatEnergy;
impl jungle_sdk::types::ActionMember for EatEnergy {}
impl Action for EatEnergy {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U7>;
    type Dependency = EatDependency;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.base_gain))
    }
}

struct HuntEnergy;
impl jungle_sdk::types::ActionMember for HuntEnergy {}
impl Action for HuntEnergy {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U10>;
    type Dependency = HuntDependency;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.gain))
    }
}

struct RoundAdvance;
impl jungle_sdk::types::ActionMember for RoundAdvance {}
impl Action for RoundAdvance {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U13>;
    type Dependency = RoundDependency;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.tick))
    }
}

struct AddI32<Focus, A>(PhantomData<fn() -> (Focus, A)>);
impl<T, Focus, A> Act<T> for AddI32<Focus, A>
where
    T: Animal,
    Focus: jungle_sdk::types::Aspect<T::State, View = i32>,
    A: Action<In = i32, Out = i32, Err = ()>,
{
    type Action = A;
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn emit(value: &i32, _input: Self::In) -> Self::In {
        *value
    }

    fn absorb(value: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
        let delta = output.expect("add i32 step should succeed");
        *value += delta;
        *value
    }
}

type ApeRoundTask = AddI32<Lens<ExecutorApeState, list![U0, U1]>, RoundAdvance>;
type TigerHuntTask = AddI32<Lens<ExecutorCatState, list![U0, U0]>, HuntEnergy>;
type TigerEatTask = AddI32<Lens<ExecutorCatState, list![U0, U0]>, EatEnergy>;

struct ApeKeepRunning;
impl LoopCondition<ExecutorApeState> for ApeKeepRunning {
    type CarryIn = i32;

    fn should_continue(state: &ExecutorApeState) -> bool {
        state.core.rounds < 4
    }
}

struct TigerKeepRunning;
impl LoopCondition<ExecutorCatState> for TigerKeepRunning {
    type CarryIn = i32;

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

type WorkflowGorillaJourney = While<ApeKeepRunning, Step<WorkflowGorilla, ApeRoundTask>>;
type WorkflowTigerJourney = While<
    TigerKeepRunning,
    jungle_sdk::types::Conditional<
        TigerChooseHunt,
        Step<WorkflowTiger, TigerHuntTask>,
        Step<WorkflowTiger, TigerEatTask>,
    >,
>;

animal!(
    WorkflowGorilla,
    jungle_sdk::typosaurus::num::consts::U11,
    ExecutorApeState,
    WorkflowGorillaJourney
);
animal!(
    WorkflowTiger,
    jungle_sdk::typosaurus::num::consts::U12,
    ExecutorCatState,
    WorkflowTigerJourney
);

#[tokio::test]
async fn jungle_executor_runs_actions_with_ecosystem_dependency() {
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
        let completion = request.run().await.expect("gorilla action should execute");
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
        let completion = request.run().await.expect("tiger action should execute");
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
            .expect("tiger odd action should execute");
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
        let completion = request.run().await.expect("gorilla action should execute");
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
        .on_action_input({
            let input_calls = Arc::clone(&input_calls);
            move |_, _| {
                let input_calls = Arc::clone(&input_calls);
                async move {
                    input_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_action_success_output({
            let success_calls = Arc::clone(&success_calls);
            move |_, _| {
                let success_calls = Arc::clone(&success_calls);
                async move {
                    success_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_action_failure_output({
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
    use jungle_sdk::types::RunnerStep;
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
            move || {
                let poll_calls = Arc::clone(&poll_calls);
                let seed = seed.clone();
                async move {
                    let idx = poll_calls.fetch_add(1, Ordering::Relaxed);
                    if idx == 0 {
                        Ok(Some(RunnerStep::StartJourney {
                            journey_id,
                            ordinal: 16,
                            seed,
                        }))
                    } else {
                        Ok(None)
                    }
                }
            }
        })
        .on_action_input({
            let input_calls = Arc::clone(&input_calls);
            move |_, _| {
                let input_calls = Arc::clone(&input_calls);
                async move {
                    input_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_action_success_output({
            let success_calls = Arc::clone(&success_calls);
            move |_, _| {
                let success_calls = Arc::clone(&success_calls);
                async move {
                    success_calls.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        })
        .on_action_failure_output({
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
    let worker_future = worker.spawn();
    tokio::pin!(worker_future);

    tokio::select! {
        result = &mut worker_future => {
            panic!("worker should keep polling, got: {result:?}");
        }
        timed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if success_calls.load(Ordering::Relaxed) == 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }) => {
            if timed.is_err() {
                panic!(
                    "worker flow should complete: polls={}, inputs={}, successes={}, failures={}",
                    poll_calls.load(Ordering::Relaxed),
                    input_calls.load(Ordering::Relaxed),
                    success_calls.load(Ordering::Relaxed),
                    failure_calls.load(Ordering::Relaxed),
                );
            }
        }
    }

    assert_eq!(input_calls.load(Ordering::Relaxed), 2);
    assert_eq!(success_calls.load(Ordering::Relaxed), 2);
    assert_eq!(failure_calls.load(Ordering::Relaxed), 0);
    assert_eq!(flow_complete_calls.load(Ordering::Relaxed), 1);
}
