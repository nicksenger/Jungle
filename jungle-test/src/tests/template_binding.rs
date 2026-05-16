use jungle_sdk::types::{
    Act, ActionSpec, BindAnimal, Ecosystem, Effect, EffectCompletion, Identity, JourneyStatus,
    ManualExecutor, RunnerOut, Step, UStep,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::{U0, U40, U41, U42, U43, U44, U45, U46};
use jungle_sdk::{Animals, JungleClient};
use std::time::Duration;

struct TemplateAddEffect;
impl<J> Effect<J> for TemplateAddEffect {
    type Id = jungle_sdk::types::Id<U40>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}

struct TemplateCommitEffect;
impl<J> Effect<J> for TemplateCommitEffect {
    type Id = jungle_sdk::types::Id<U41>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

struct AddOneSpec;
struct CommitSpec;

impl ActionSpec for AddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: jungle_sdk::types::Animal> = GenericAddOne<A>;
}

impl ActionSpec for CommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: jungle_sdk::types::Animal> = GenericCommit<A>;
}

#[derive(jungle_sdk::FlowTemplate)]
struct TemplateFlow(UStep<AddOneSpec>, UStep<CommitSpec>);

#[derive(jungle_sdk::Journey)]
struct CounterJourney(
    Step<CounterAnimal, CounterAddOne>,
    Step<CounterAnimal, CounterCommit>,
);

#[derive(jungle_sdk::Journey)]
struct LedgerJourney(
    Step<LedgerAnimal, LedgerAddOne>,
    Step<LedgerAnimal, LedgerCommit>,
);

struct CounterAnimal;
impl jungle_sdk::types::Animal for CounterAnimal {
    type Id = jungle_sdk::types::Id<U42>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = CounterJourney;
}

impl jungle_sdk::types::Observable for CounterAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for CounterAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for CounterAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U42, CounterAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for CounterAnimal {
    type Id = U42;
}

impl LateBoundPolicy for CounterAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LedgerAnimal;
impl jungle_sdk::types::Animal for LedgerAnimal {
    type Id = jungle_sdk::types::Id<U43>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = LedgerJourney;
}

impl jungle_sdk::types::Observable for LedgerAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for LedgerAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for LedgerAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U43, LedgerAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for LedgerAnimal {
    type Id = U43;
}

impl LateBoundPolicy for LedgerAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct CounterAddOne;
impl Act<CounterAnimal> for CounterAddOne {
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 1
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("counter add step should succeed");
        *state = value;
        value
    }
}

struct LedgerAddOne;
impl Act<LedgerAnimal> for LedgerAddOne {
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 10
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("ledger add step should succeed");
        *state = value;
        value
    }
}

struct CounterCommit;
impl Act<CounterAnimal> for CounterCommit {
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("counter commit step should succeed");
        *state = value;
        value
    }
}

struct LedgerCommit;
impl Act<LedgerAnimal> for LedgerCommit {
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state - input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("ledger commit step should succeed");
        *state = value;
        value
    }
}

struct GenericAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericAddOne<A>
where
    A: jungle_sdk::types::Animal<State = i32> + LateBoundPolicy,
{
    type Effect = TemplateAddEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + A::ADD_INPUT_DELTA
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("generic add step should succeed");
        *state = value;
        value
    }
}

struct GenericCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericCommit<A>
where
    A: jungle_sdk::types::Animal<State = i32> + LateBoundPolicy,
{
    type Effect = TemplateCommitEffect;
    type StateAspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        if A::COMMIT_SUBTRACT {
            *state - input
        } else {
            *state + input
        }
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("generic commit step should succeed");
        *state = value;
        value
    }
}

#[test]
fn template_binding_executes_with_animal_specific_actions() {
    let mut counter = ManualExecutor::<CounterAnimal>::new(0);
    let counter_request_1: i32 = counter
        .next_request_typed::<_, i32>(3)
        .expect("counter first request");
    assert_eq!(counter_request_1, 4);
    let counter_emitted_1: i32 = counter
        .complete_typed::<i32, (), i32>(Ok(5))
        .expect("counter first completion");
    assert_eq!(counter_emitted_1, 5);

    let counter_request_2: i32 = counter
        .next_request_typed::<_, i32>(2)
        .expect("counter second request");
    assert_eq!(counter_request_2, 7);
    let counter_emitted_2: i32 = counter
        .complete_typed::<i32, (), i32>(Ok(7))
        .expect("counter second completion");
    assert_eq!(counter_emitted_2, 7);
    assert_eq!(counter.into_state(), 7);

    let mut ledger = ManualExecutor::<LedgerAnimal>::new(0);
    let ledger_request_1: i32 = ledger
        .next_request_typed::<_, i32>(3)
        .expect("ledger first request");
    assert_eq!(ledger_request_1, 13);
    let ledger_emitted_1: i32 = ledger
        .complete_typed::<i32, (), i32>(Ok(20))
        .expect("ledger first completion");
    assert_eq!(ledger_emitted_1, 20);

    let ledger_request_2: i32 = ledger
        .next_request_typed::<_, i32>(2)
        .expect("ledger second request");
    assert_eq!(ledger_request_2, 18);
    let ledger_emitted_2: i32 = ledger
        .complete_typed::<i32, (), i32>(Ok(18))
        .expect("ledger second completion");
    assert_eq!(ledger_emitted_2, 18);
    assert_eq!(ledger.into_state(), 18);
}

#[test]
fn template_binding_preserves_step_shape_after_binding() {
    type CounterBound = <TemplateFlow as BindAnimal<CounterAnimal>>::Bound;
    type LedgerBound = <TemplateFlow as BindAnimal<LedgerAnimal>>::Bound;
    type ExpectedCounter = jungle_sdk::typosaurus::collections::list::List<(
        Step<CounterAnimal, GenericAddOne<CounterAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            Step<CounterAnimal, GenericCommit<CounterAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;
    type ExpectedLedger = jungle_sdk::typosaurus::collections::list::List<(
        Step<LedgerAnimal, GenericAddOne<LedgerAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            Step<LedgerAnimal, GenericCommit<LedgerAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;

    let _counter_step_1: Step<CounterAnimal, CounterAddOne> = Step::new();
    let _counter_step_2: Step<CounterAnimal, CounterCommit> = Step::new();
    let _ledger_step_1: Step<LedgerAnimal, LedgerAddOne> = Step::new();
    let _ledger_step_2: Step<LedgerAnimal, LedgerCommit> = Step::new();

    assert_type_eq!(CounterBound, ExpectedCounter);
    assert_type_eq!(LedgerBound, ExpectedLedger);
}

struct BoundTemplateAnimal;
impl jungle_sdk::types::Animal for BoundTemplateAnimal {
    type Id = jungle_sdk::types::Id<U44>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<BoundTemplateAnimal>>::Bound;
}

impl jungle_sdk::types::Observable for BoundTemplateAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for BoundTemplateAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

trait LateBoundPolicy {
    const ADD_INPUT_DELTA: i32;
    const COMMIT_SUBTRACT: bool;
}

impl LateBoundPolicy for BoundTemplateAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

#[test]
fn template_binding_bound_journey_is_executor_ready() {
    let mut executor = ManualExecutor::<BoundTemplateAnimal>::new(0);

    let req_1: i32 = executor
        .next_request_typed::<_, i32>(3)
        .expect("first bound request");
    assert_eq!(req_1, 4);
    let out_1: i32 = executor
        .complete_typed::<i32, (), i32>(Ok(6))
        .expect("first bound completion");
    assert_eq!(out_1, 6);

    let req_2: i32 = executor
        .next_request_typed::<_, i32>(2)
        .expect("second bound request");
    assert_eq!(req_2, 8);
    let out_2: i32 = executor
        .complete_typed::<i32, (), i32>(Ok(8))
        .expect("second bound completion");
    assert_eq!(out_2, 8);
    assert_eq!(executor.into_state(), 8);
}

struct LocalTemplateAlphaAnimal;
impl jungle_sdk::types::Animal for LocalTemplateAlphaAnimal {
    type Id = jungle_sdk::types::Id<U45>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<LocalTemplateAlphaAnimal>>::Bound;
}

impl jungle_sdk::types::Observable for LocalTemplateAlphaAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for LocalTemplateAlphaAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for LocalTemplateAlphaAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U45, LocalTemplateAlphaAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for LocalTemplateAlphaAnimal {
    type Id = U45;
}

impl LateBoundPolicy for LocalTemplateAlphaAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LocalTemplateBetaAnimal;
impl jungle_sdk::types::Animal for LocalTemplateBetaAnimal {
    type Id = jungle_sdk::types::Id<U46>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<LocalTemplateBetaAnimal>>::Bound;
}

impl jungle_sdk::types::Observable for LocalTemplateBetaAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for LocalTemplateBetaAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for LocalTemplateBetaAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U46, LocalTemplateBetaAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for LocalTemplateBetaAnimal {
    type Id = U46;
}

impl LateBoundPolicy for LocalTemplateBetaAnimal {
    const ADD_INPUT_DELTA: i32 = 10;
    const COMMIT_SUBTRACT: bool = true;
}

#[derive(Animals)]
struct LocalTemplateAnimals(LocalTemplateAlphaAnimal, LocalTemplateBetaAnimal);

struct LocalTemplateZoo;
impl Ecosystem for LocalTemplateZoo {
    const NAME: &'static str = "late-bound-local-template-zoo";
    type Animals = LocalTemplateAnimals;
}

#[tokio::test]
async fn template_binding_local_client_reuses_one_template_for_two_animals_end_to_end() {
    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-local-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LocalTemplateZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .start_journey::<LocalTemplateAlphaAnimal>(
            postcard::to_allocvec(&3_i32).expect("alpha seed should serialize"),
        )
        .await
        .expect("alpha journey should start");
    let beta_id = client
        .start_journey::<LocalTemplateBetaAnimal>(
            postcard::to_allocvec(&3_i32).expect("beta seed should serialize"),
        )
        .await
        .expect("beta journey should start");

    await_completion(&client, alpha_id).await;
    await_completion(&client, beta_id).await;

    let alpha_history = client
        .journey_history(alpha_id)
        .await
        .expect("alpha history should be available");
    let beta_history = client
        .journey_history(beta_id)
        .await
        .expect("beta history should be available");

    let alpha_inputs = decode_effect_inputs(&alpha_history);
    let beta_inputs = decode_effect_inputs(&beta_history);

    // Same bound template topology (2 steps), different behavior from animal-specific late binding.
    assert_eq!(alpha_inputs, vec![4, 10]);
    assert_eq!(beta_inputs, vec![13, 0]);

    worker_handle.abort();
    let _ = worker_handle.await;
}

async fn await_completion(client: &jungle_sdk::LocalClient, journey_id: uuid::Uuid) {
    let completion = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == JourneyStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    if completion.is_err() {
        panic!("journey did not complete before timeout");
    }
}

fn decode_effect_inputs(history: &[RunnerOut]) -> Vec<i32> {
    history
        .iter()
        .filter_map(|entry| match entry {
            RunnerOut::EffectInput { data, .. } => Some(
                postcard::from_bytes::<i32>(data)
                    .expect("effect input payload should deserialize to i32"),
            ),
            _ => None,
        })
        .collect()
}
