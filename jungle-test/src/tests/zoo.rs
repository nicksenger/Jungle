use futures::{channel::mpsc, StreamExt};
use jungle_sdk::core::Jungle as _;
use jungle_sdk::types::{
    Action, ActionCompletion, ActionSet, ClientIn, Creature, CreatureActionSet, CreatureSet,
    CreatureStates, Ecosystem, Identity, Impulse, Lens, LoopCondition, Task, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};
use jungle_sdk::{Actions, Creatures, Flow, Optic};
use std::marker::PhantomData;
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

struct SharedState;
impl<T> From<&T> for SharedState {
    fn from(_value: &T) -> Self {
        Self
    }
}

struct UnitOkStep<A>(PhantomData<fn() -> A>);
impl<T, A> Task<T> for UnitOkStep<A>
where
    T: Creature,
    A: Action<In = ()>,
    A: Action<Out = (), Err = ()>,
{
    type Action = A;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &T::State, _input: Self::In) -> A::In {}

    fn process(_state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
        output.expect("workflow action should succeed");
    }
}

animal!(Gorilla, U0, GorillaInstinct);
animal!(Chimpanzee, U1, ChimpanzeeInstinct);
animal!(Tiger, U2, TigerInstinct);
animal!(Jaguar, U3, JaguarInstinct);
animal!(Anaconda, U4, AnacondaInstinct);
animal!(Hippo, U5, HippoInstinct);
animal!(Elephant, U6, ElephantInstinct);

#[derive(Flow)]
struct GorillaInstinct(
    Impulse<Gorilla, UnitOkStep<Eat>>,
    Impulse<Gorilla, UnitOkStep<Sleep>>,
    Impulse<Gorilla, UnitOkStep<Forage>>,
    Impulse<Gorilla, UnitOkStep<Drink>>,
    Impulse<Gorilla, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct ChimpanzeeInstinct(
    Impulse<Chimpanzee, UnitOkStep<Eat>>,
    Impulse<Chimpanzee, UnitOkStep<Sleep>>,
    Impulse<Chimpanzee, UnitOkStep<Forage>>,
    Impulse<Chimpanzee, UnitOkStep<Drink>>,
    Impulse<Chimpanzee, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct TigerInstinct(
    Impulse<Tiger, UnitOkStep<Eat>>,
    Impulse<Tiger, UnitOkStep<Sleep>>,
    Impulse<Tiger, UnitOkStep<Forage>>,
    Impulse<Tiger, UnitOkStep<Drink>>,
    Impulse<Tiger, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct JaguarInstinct(
    Impulse<Jaguar, UnitOkStep<Eat>>,
    Impulse<Jaguar, UnitOkStep<Sleep>>,
    Impulse<Jaguar, UnitOkStep<Forage>>,
    Impulse<Jaguar, UnitOkStep<Drink>>,
    Impulse<Jaguar, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct AnacondaInstinct(
    Impulse<Anaconda, UnitOkStep<Eat>>,
    Impulse<Anaconda, UnitOkStep<Sleep>>,
    Impulse<Anaconda, UnitOkStep<Forage>>,
    Impulse<Anaconda, UnitOkStep<Drink>>,
    Impulse<Anaconda, UnitOkStep<Hunt>>,
);

#[derive(Flow)]
struct HippoInstinct(
    Impulse<Hippo, UnitOkStep<Eat>>,
    Impulse<Hippo, UnitOkStep<Sleep>>,
    Impulse<Hippo, UnitOkStep<Forage>>,
    Impulse<Hippo, UnitOkStep<Drink>>,
    Impulse<Hippo, UnitOkStep<Flee>>,
);

#[derive(Flow)]
struct ElephantInstinct(
    Impulse<Elephant, UnitOkStep<Eat>>,
    Impulse<Elephant, UnitOkStep<Sleep>>,
    Impulse<Elephant, UnitOkStep<Forage>>,
    Impulse<Elephant, UnitOkStep<Drink>>,
    Impulse<Elephant, UnitOkStep<Flee>>,
);

#[derive(Creatures)]
struct Apes(Gorilla, Chimpanzee);

#[derive(Creatures)]
struct Cats(Tiger, Jaguar);

#[derive(Creatures)]
struct Predators(Cats, Anaconda);

#[derive(Creatures)]
struct AllCreatures(Cats, Apes, Anaconda, Hippo, Elephant);

#[derive(Actions)]
struct AllActions(Predator, Prey);

struct Zoo;
impl Ecosystem for Zoo {
    type Creatures = AllCreatures;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
impl Task<RunnerCreature> for RunnerStepOne {
    type Action = RunnerStepOneAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &RunnerState, _input: Self::In) -> Self::In {}

    fn process(state: &mut RunnerState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("runner step one should succeed");
    }
}

struct RunnerStepTwo;
impl Task<RunnerCreature> for RunnerStepTwo {
    type Action = RunnerStepTwoAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &RunnerState, _input: Self::In) -> Self::In {}

    fn process(state: &mut RunnerState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("runner step two should succeed");
    }
}

struct RunnerKeepGoing;
impl LoopCondition<RunnerState> for RunnerKeepGoing {
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

type RunnerInstinct = While<
    RunnerKeepGoing,
    jungle_sdk::types::Conditional<
        RunnerUseStepOne,
        Impulse<RunnerCreature, RunnerStepOne>,
        Impulse<RunnerCreature, RunnerStepTwo>,
    >,
>;

animal!(
    RunnerCreature,
    jungle_sdk::typosaurus::num::consts::U16,
    RunnerState,
    RunnerInstinct
);

struct RunnerZoo;

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
    assert_type_eq!(CreatureSet<Apes>, ApeList);

    type PredatorList = list![Tiger, Jaguar, Anaconda];
    assert_type_eq!(CreatureSet<Predators>, PredatorList);
}

#[test]
fn animal_action_set() {
    type ApeCreatureActions = list![Eat, Sleep, Forage, Drink, Flee];
    assert_type_eq!(CreatureActionSet<Apes>, ApeCreatureActions);

    type AllCreatureActions = list![Eat, Sleep, Forage, Drink, Hunt, Flee];
    assert_type_eq!(CreatureActionSet<AllCreatures>, AllCreatureActions);
}

#[test]
fn animal_state_set() {
    struct ApeState;
    struct CatState;

    #[derive(Flow)]
    struct StatefulGorillaInstinct(
        Impulse<StatefulGorilla, UnitOkStep<Eat>>,
        Impulse<StatefulGorilla, UnitOkStep<Sleep>>,
        Impulse<StatefulGorilla, UnitOkStep<Forage>>,
        Impulse<StatefulGorilla, UnitOkStep<Drink>>,
        Impulse<StatefulGorilla, UnitOkStep<Flee>>,
    );

    #[derive(Flow)]
    struct StatefulTigerInstinct(
        Impulse<StatefulTiger, UnitOkStep<Eat>>,
        Impulse<StatefulTiger, UnitOkStep<Sleep>>,
        Impulse<StatefulTiger, UnitOkStep<Forage>>,
        Impulse<StatefulTiger, UnitOkStep<Drink>>,
        Impulse<StatefulTiger, UnitOkStep<Hunt>>,
    );

    animal!(StatefulGorilla, U0, ApeState, StatefulGorillaInstinct);
    animal!(StatefulTiger, U1, CatState, StatefulTigerInstinct);

    #[derive(Creatures)]
    struct StatefulCreatures(StatefulGorilla, StatefulTiger);

    type StatefulCreatureStates = list![ApeState, CatState];
    assert_type_eq!(CreatureStates<StatefulCreatures>, StatefulCreatureStates);
}

#[test]
fn jungle_impl() {
    let zoo = Zoo;
    let _jungle_fut = zoo.manifest();
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
struct CoreState {
    energy: i32,
    rounds: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
struct ExecutorApeState {
    core: CoreState,
    bananas: i32,
    mood: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
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
impl<T, Focus, A> Task<T> for AddI32<Focus, A>
where
    T: Creature,
    Focus: jungle_sdk::types::Aspect<T::State, View = i32>,
    A: Action<In = i32, Out = i32, Err = ()>,
{
    type Action = A;
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(value: &i32, _input: Self::In) -> Self::In {
        *value
    }

    fn process(value: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Out {
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
    fn should_continue(state: &ExecutorApeState) -> bool {
        state.core.rounds < 4
    }
}

struct TigerKeepRunning;
impl LoopCondition<ExecutorCatState> for TigerKeepRunning {
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

type WorkflowGorillaInstinct = While<ApeKeepRunning, Impulse<WorkflowGorilla, ApeRoundTask>>;
type WorkflowTigerInstinct = While<
    TigerKeepRunning,
    jungle_sdk::types::Conditional<
        TigerChooseHunt,
        Impulse<WorkflowTiger, TigerHuntTask>,
        Impulse<WorkflowTiger, TigerEatTask>,
    >,
>;

animal!(
    WorkflowGorilla,
    jungle_sdk::typosaurus::num::consts::U11,
    ExecutorApeState,
    WorkflowGorillaInstinct
);
animal!(
    WorkflowTiger,
    jungle_sdk::typosaurus::num::consts::U12,
    ExecutorCatState,
    WorkflowTigerInstinct
);

#[tokio::test]
async fn jungle_executor_runs_actions_with_ecosystem_dependency() {
    use jungle_sdk::core::JungleExecutor;

    let zoo = Zoo;
    let mut gorilla = JungleExecutor::<Zoo, WorkflowGorilla>::new(
        &zoo,
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
    while !gorilla.is_complete() {
        let request = gorilla
            .next_executable_request(1i32)
            .expect("gorilla request should build");
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
        &zoo,
        ExecutorCatState {
            core: CoreState {
                energy: 6,
                rounds: 0,
            },
            stripes: 8,
        },
    );
    let mut tiger_requests = Vec::new();
    while !tiger.is_complete() {
        let request = tiger
            .next_executable_request(1i32)
            .expect("tiger request should build");
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
        &zoo,
        ExecutorCatState {
            core: CoreState {
                energy: 6,
                rounds: 0,
            },
            stripes: 7,
        },
    );
    let mut tiger_odd_requests = Vec::new();
    while !tiger_odd.is_complete() {
        let request = tiger_odd
            .next_executable_request(1i32)
            .expect("tiger odd request should build");
        let request_input: i32 = request
            .deserialize_request()
            .expect("tiger odd request should deserialize");
        tiger_odd_requests.push(request_input);
        let completion = request.run().await.expect("tiger odd action should execute");
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

    let zoo = Zoo;
    let mut gorilla = JungleExecutor::<Zoo, WorkflowGorilla>::new(
        &zoo,
        ExecutorApeState {
            core: CoreState {
                energy: 5,
                rounds: 0,
            },
            bananas: 4,
            mood: 2,
        },
    );

    while !gorilla.is_complete() {
        let rounds_before = gorilla.state().core.rounds;
        let request = gorilla
            .next_executable_request(1i32)
            .expect("gorilla request should build");
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
async fn jungle_runner_spawns_and_completes_creature_flows() {
    use jungle_sdk::core::JungleRunner;

    let runner = JungleRunner::new(RunnerZoo);
    let (tx, mut rx) = mpsc::channel::<(ClientIn, futures::channel::oneshot::Sender<()>)>(32);
    let resolver = tokio::spawn(async move {
        while let Some((message, done)) = rx.next().await {
            match message {
                ClientIn::ActionInput { .. } | ClientIn::ActionOutput { .. } => {
                    let _ = done.send(());
                }
            }
        }
    });

    let (first, second, third) = tokio::join!(
        runner.spawn::<RunnerCreature>(RunnerState(0), Uuid::from_u128(1), tx.clone()),
        runner.spawn::<RunnerCreature>(RunnerState(3), Uuid::from_u128(2), tx.clone()),
        runner.spawn::<RunnerCreature>(RunnerState(2), Uuid::from_u128(3), tx.clone()),
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
}
