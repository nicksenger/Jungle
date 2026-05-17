use jungle_sdk::animal;
use jungle_sdk::effect;
use jungle_sdk::types::Animal;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    Act, ActionSpec, BindAnimal, Condition, Conditional, Ecosystem, EffectCompletion, EffectExec,
    EffectSchema, Either, Identity, Join, JourneyStatus, Lens, LoopCondition, ManualExecutor,
    NodeMetadata, Observe, ReplaceFlow, ReplaceStep, RunnerOut, Scoped, Select, Step, Transparent,
    TraverseFlow, TraverseStep, UStep, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::*;
use jungle_sdk::typosaurus::num::consts::{
    U0, U40, U41, U42, U43, U44, U45, U46, U47, U48, U49, U53,
};
use jungle_sdk::{Animals, JungleClient, Optic};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct TemplateAddEffect;
impl EffectSchema for TemplateAddEffect {
    type Id = Id<U40>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for TemplateAddEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}

pub struct TemplateCommitEffect;
impl EffectSchema for TemplateCommitEffect {
    type Id = Id<U41>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for TemplateCommitEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct AddOneSpec;
pub struct CommitSpec;

impl ActionSpec for AddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = GenericAddOne<A>;
}

impl ActionSpec for CommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = GenericCommit<A>;
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
#[animal]
impl Animal for CounterAnimal {
    type Id = Id<U42>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = CounterJourney;
}

impl LateBoundPolicy for CounterAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LedgerAnimal;
#[animal]
impl Animal for LedgerAnimal {
    type Id = Id<U43>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = LedgerJourney;
}

impl LateBoundPolicy for LedgerAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct CounterAddOne;
impl Act<CounterAnimal> for CounterAddOne {
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
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
    type Aspect = Identity;
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
    type Aspect = Identity;
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
    type Aspect = Identity;
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

pub struct GenericAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericAddOne<A>
where
    A: Animal<State = i32> + LateBoundPolicy,
{
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
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

pub struct GenericCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for GenericCommit<A>
where
    A: Animal<State = i32> + LateBoundPolicy,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
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
#[animal]
impl Animal for BoundTemplateAnimal {
    type Id = Id<U44>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<BoundTemplateAnimal>>::Bound;
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
#[animal]
impl Animal for LocalTemplateAlphaAnimal {
    type Id = Id<U45>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<LocalTemplateAlphaAnimal>>::Bound;
}

impl LateBoundPolicy for LocalTemplateAlphaAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LocalTemplateBetaAnimal;
#[animal]
impl Animal for LocalTemplateBetaAnimal {
    type Id = Id<U46>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <TemplateFlow as BindAnimal<LocalTemplateBetaAnimal>>::Bound;
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

trait RequiresContextBump {
    fn context_bump(&self) -> i32;
}

pub struct ContextBoundTemplateEffect;
#[effect]
impl<J> jungle_sdk::types::Effect<J> for ContextBoundTemplateEffect
where
    J: RequiresContextBump,
{
    type Id = Id<U53>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + jungle.context_bump()))
    }
}

struct ContextBoundSpec;
impl ActionSpec for ContextBoundSpec {
    type Effect = ContextBoundTemplateEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = ContextBoundAct<A>;
}

struct ContextBoundAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for ContextBoundAct<A>
where
    A: Animal<State = i32>,
{
    type Effect = ContextBoundTemplateEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("context-bound step should succeed");
        *state = value;
        value
    }
}

#[derive(jungle_sdk::FlowTemplate)]
struct ContextBoundTemplateFlow(UStep<ContextBoundSpec>);

struct LocalTemplateContextAnimal;
#[animal]
impl Animal for LocalTemplateContextAnimal {
    type Id = Id<U54>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <ContextBoundTemplateFlow as BindAnimal<LocalTemplateContextAnimal>>::Bound;
}

#[derive(Animals)]
struct LocalTemplateContextAnimals(LocalTemplateContextAnimal);

struct LocalTemplateContextZoo;
impl Ecosystem for LocalTemplateContextZoo {
    const NAME: &'static str = "late-bound-context-bound-template-zoo";
    type Animals = LocalTemplateContextAnimals;
}

impl<'a> RequiresContextBump for &'a LocalTemplateContextZoo {
    fn context_bump(&self) -> i32 {
        11
    }
}

impl RequiresContextBump for LocalTemplateContextZoo {
    fn context_bump(&self) -> i32 {
        11
    }
}

impl RequiresContextBump for std::sync::Arc<LocalTemplateContextZoo> {
    fn context_bump(&self) -> i32 {
        11
    }
}

#[tokio::test]
async fn template_binding_unbound_effect_with_context_bound_runs_end_to_end_with_local_client() {
    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-context-bound-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LocalTemplateContextZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .start_journey::<LocalTemplateContextAnimal>(
            postcard::to_allocvec(&4_i32).expect("seed should serialize"),
        )
        .await
        .expect("journey should start");

    await_completion(&client, journey_id).await;

    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey history should be available");
    let effect_inputs = decode_effect_inputs(&history);
    let effect_outputs = decode_effect_success_outputs(&history);

    assert_eq!(effect_inputs, vec![4]);
    // Output proves `Effect<J>` executed against `&LocalTemplateContextZoo` and used
    // the extra `J: RequiresContextBump` bound at runtime.
    assert_eq!(effect_outputs, vec![15]);

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
            match status {
                JourneyStatus::Completed => break,
                JourneyStatus::Dead | JourneyStatus::Stopped => {
                    let history = client
                        .journey_history(journey_id)
                        .await
                        .expect("journey history should be available for terminal status");
                    panic!("journey reached terminal non-complete status {status:?}: {history:?}");
                }
                JourneyStatus::Created | JourneyStatus::Alive => {}
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    if completion.is_err() {
        let status = client
            .journey_details(journey_id)
            .await
            .expect("journey_details should succeed after timeout");
        let history = client
            .journey_history(journey_id)
            .await
            .expect("journey history should be available after timeout");
        panic!("journey did not complete before timeout; status={status:?}, history={history:?}");
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

fn decode_effect_success_outputs(history: &[RunnerOut]) -> Vec<i32> {
    history
        .iter()
        .filter_map(|entry| match entry {
            RunnerOut::EffectSuccessOutput { data, .. } => Some(
                postcard::from_bytes::<i32>(data)
                    .expect("effect output payload should deserialize to i32"),
            ),
            _ => None,
        })
        .collect()
}

mod composed_templates {
    use super::{AddOneSpec, CommitSpec};
    use jungle_sdk::types::UStep;

    // Fragment A: no Animal type appears here.
    #[derive(jungle_sdk::FlowTemplate)]
    pub struct IntakeStage(UStep<AddOneSpec>);

    // Fragment B: no Animal type appears here.
    #[derive(jungle_sdk::FlowTemplate)]
    pub struct CommitStage(UStep<CommitSpec>);

    // Final composition of independent unbound fragments, still no Animal type.
    #[derive(jungle_sdk::FlowTemplate)]
    pub struct ComposedPipeline(IntakeStage, CommitStage);
}

struct ComposedAlphaAnimal;
#[animal]
impl Animal for ComposedAlphaAnimal {
    type Id = Id<U47>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <composed_templates::ComposedPipeline as BindAnimal<ComposedAlphaAnimal>>::Bound;
}

impl LateBoundPolicy for ComposedAlphaAnimal {
    const ADD_INPUT_DELTA: i32 = 2;
    const COMMIT_SUBTRACT: bool = false;
}

struct ComposedBetaAnimal;
#[animal]
impl Animal for ComposedBetaAnimal {
    type Id = Id<U48>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <composed_templates::ComposedPipeline as BindAnimal<ComposedBetaAnimal>>::Bound;
}

impl LateBoundPolicy for ComposedBetaAnimal {
    const ADD_INPUT_DELTA: i32 = 20;
    const COMMIT_SUBTRACT: bool = true;
}

#[derive(Animals)]
struct ComposedTemplateAnimals(ComposedAlphaAnimal, ComposedBetaAnimal);

struct ComposedTemplateZoo;
impl Ecosystem for ComposedTemplateZoo {
    const NAME: &'static str = "late-bound-composed-template-zoo";
    type Animals = ComposedTemplateAnimals;
}

#[tokio::test]
async fn template_binding_composes_unbound_fragments_then_binds_once_per_animal() {
    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-composed-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(ComposedTemplateZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .start_journey::<ComposedAlphaAnimal>(
            postcard::to_allocvec(&3_i32).expect("alpha seed should serialize"),
        )
        .await
        .expect("alpha journey should start");
    let beta_id = client
        .start_journey::<ComposedBetaAnimal>(
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

    // The flow came from composition of unbound fragments, then was bound at the edge once per animal.
    assert_eq!(alpha_inputs, vec![5, 12]);
    assert_eq!(beta_inputs, vec![23, 0]);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LensLeaf {
    value: i32,
    noise: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LensBranch {
    leaf: LensLeaf,
    spare: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LensRootState {
    branch: LensBranch,
    committed: i32,
}

impl From<i32> for LensRootState {
    fn from(seed: i32) -> Self {
        Self {
            branch: LensBranch {
                leaf: LensLeaf { value: 4, noise: 9 },
                spare: seed,
            },
            committed: 0,
        }
    }
}

struct LensReadSpareSpec;
struct LensReadLeafSpec;
struct LensCommitSpec;

type LensRootSpareCarrier = Lens<LensRootState, jungle_sdk::typosaurus::list![U0, U1]>;
type LensRootLeafValueCarrier = Lens<LensRootState, jungle_sdk::typosaurus::list![U0, U0, U0]>;

struct LensReadSpareAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for LensReadSpareAct<A>
where
    A: Animal<State = LensRootState>,
{
    type Effect = TemplateAddEffect;
    type Aspect = LensRootSpareCarrier;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("lens spare step should succeed");
        *view = out;
        out
    }
}

struct LensReadLeafAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for LensReadLeafAct<A>
where
    A: Animal<State = LensRootState>,
{
    type Effect = TemplateAddEffect;
    type Aspect = LensRootLeafValueCarrier;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("lens leaf step should succeed");
        *view = out;
        out
    }
}

struct LensCommitAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for LensCommitAct<A>
where
    A: Animal<State = LensRootState>,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &LensRootState, input: Self::Input) -> i32 {
        input
    }

    fn absorb(state: &mut LensRootState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("lens commit should succeed");
        state.committed = out;
        out
    }
}

impl ActionSpec for LensReadSpareSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = LensReadSpareAct<A>;
}

impl ActionSpec for LensReadLeafSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = LensReadLeafAct<A>;
}

impl ActionSpec for LensCommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = LensCommitAct<A>;
}

#[derive(jungle_sdk::FlowTemplate)]
struct LensTemplate(UStep<LensReadSpareSpec>, UStep<LensCommitSpec>);

struct SeenStep<T>(core::marker::PhantomData<T>);
struct LensTraversal;
impl<S> TraverseStep<jungle_sdk::types::StepSpec<S>> for LensTraversal
where
    S: ActionSpec,
{
    type Output = SeenStep<jungle_sdk::types::StepSpec<S>>;
}

struct LensReplacer;
impl ReplaceStep<jungle_sdk::types::StepSpec<LensReadSpareSpec>> for LensReplacer {
    type Output = jungle_sdk::types::StepSpec<LensReadLeafSpec>;
}
impl ReplaceStep<jungle_sdk::types::StepSpec<LensReadLeafSpec>> for LensReplacer {
    type Output = jungle_sdk::types::StepSpec<LensReadSpareSpec>;
}
impl ReplaceStep<jungle_sdk::types::StepSpec<LensCommitSpec>> for LensReplacer {
    type Output = jungle_sdk::types::StepSpec<LensCommitSpec>;
}

struct LensAlphaAnimal;
#[animal(observe)]
impl Animal for LensAlphaAnimal {
    type Id = Id<U52>;
    type Generation = U0;
    type State = LensRootState;
    type Seed = i32;
    type Journey = <LensTemplate as BindAnimal<LensAlphaAnimal>>::Bound;
}

impl Observe for LensAlphaAnimal {
    type Appearance = LensRootState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct LensAnimals(LensAlphaAnimal);

struct LensZoo;
impl Ecosystem for LensZoo {
    const NAME: &'static str = "late-bound-lens-replace-zoo";
    type Animals = LensAnimals;
}

#[derive(jungle_sdk::FlowTemplate)]
#[jungle(view = LensBranch)]
struct ScopedLensTemplate(LensTemplate);

#[derive(jungle_sdk::FlowTemplate)]
#[jungle(view = LensBranch)]
struct ScopedLensMultiField(LensTemplate, LensTemplate);

#[test]
fn template_binding_unbound_flow_supports_traverse_and_replace_with_lens_specs() {
    type Traversed = jungle_sdk::types::Traversed<LensTemplate, LensTraversal>;
    type ExpectedTraversed = jungle_sdk::typosaurus::list![
        SeenStep<jungle_sdk::types::StepSpec<LensReadSpareSpec>>,
        SeenStep<jungle_sdk::types::StepSpec<LensCommitSpec>>
    ];
    assert_type_eq!(Traversed, ExpectedTraversed);

    type Replaced = jungle_sdk::types::Replace<LensTemplate, LensReplacer>;
    type ExpectedReplaced = jungle_sdk::typosaurus::list![
        jungle_sdk::types::StepSpec<LensReadLeafSpec>,
        jungle_sdk::types::StepSpec<LensCommitSpec>
    ];
    assert_type_eq!(Replaced, ExpectedReplaced);

    type ScopedTraverse = <ScopedLensTemplate as TraverseFlow>::Output;
    type ScopedExpected = Scoped<LensBranch, <ScopedLensTemplate as ReplaceFlow>::Output>;
    assert_type_eq!(ScopedTraverse, ScopedExpected);

    type ScopedMultiTraverse = <ScopedLensMultiField as TraverseFlow>::Output;
    type ScopedMultiExpected = Scoped<LensBranch, <ScopedLensMultiField as ReplaceFlow>::Output>;
    assert_type_eq!(ScopedMultiTraverse, ScopedMultiExpected);
}

#[tokio::test]
async fn template_binding_unbound_lens_template_runs_end_to_end() {
    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-lens-replace-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LensZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .start_journey::<LensAlphaAnimal>(
            postcard::to_allocvec(&30_i32).expect("seed should serialize"),
        )
        .await
        .expect("journey should start");

    await_completion(&client, journey_id).await;

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: LensRootState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");

    // Unbound template is bound once and executes a lens-based action over `branch.spare`.
    // With seed=30 as flow input, spare path is updated (0 + 30 + effect +1 => 31).
    assert_eq!(appearance.branch.leaf.value, 0);
    assert_eq!(appearance.branch.leaf.noise, 0);
    assert_eq!(appearance.branch.spare, 31);
    assert_eq!(appearance.committed, 31);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedLensLeaf {
    value: i32,
    noise: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedLensBranch {
    #[view]
    leaf: NestedLensLeaf,
    spare: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedLensRootState {
    #[view]
    branch: NestedLensBranch,
    committed: i32,
}

impl From<i32> for NestedLensRootState {
    fn from(seed: i32) -> Self {
        Self {
            branch: NestedLensBranch {
                leaf: NestedLensLeaf::default(),
                spare: seed,
            },
            committed: 0,
        }
    }
}

struct NestedBranchSpareSpec;
struct NestedLeafValueSpec;
struct NestedLeafNoiseSpec;

type NestedBranchSpareCarrier = Lens<NestedLensBranch, U1>;
type NestedLeafValueCarrier = Lens<NestedLensLeaf, U0>;
type NestedLeafNoiseCarrier = Lens<NestedLensLeaf, U1>;

struct NestedBranchSpareAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for NestedBranchSpareAct<A>
where
    A: Animal<State = NestedLensBranch>,
{
    type Effect = TemplateAddEffect;
    type Aspect = NestedBranchSpareCarrier;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("nested branch spare step should succeed");
        *view = out;
        out
    }
}

struct NestedLeafValueAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for NestedLeafValueAct<A>
where
    A: Animal<State = NestedLensLeaf>,
{
    type Effect = TemplateAddEffect;
    type Aspect = NestedLeafValueCarrier;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("nested leaf value step should succeed");
        *view = out;
        out
    }
}

struct NestedLeafNoiseAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for NestedLeafNoiseAct<A>
where
    A: Animal<State = NestedLensLeaf>,
{
    type Effect = TemplateAddEffect;
    type Aspect = NestedLeafNoiseCarrier;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("nested leaf noise step should succeed");
        *view = out;
        out
    }
}

impl ActionSpec for NestedBranchSpareSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = NestedBranchSpareAct<A>;
}

impl ActionSpec for NestedLeafValueSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = NestedLeafValueAct<A>;
}

impl ActionSpec for NestedLeafNoiseSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = NestedLeafNoiseAct<A>;
}

#[derive(jungle_sdk::FlowTemplate)]
#[jungle(view = NestedLensLeaf)]
struct NestedLeafScopedTemplate(UStep<NestedLeafValueSpec>, UStep<NestedLeafNoiseSpec>);

#[derive(jungle_sdk::FlowTemplate)]
#[jungle(view = NestedLensBranch)]
struct NestedBranchScopedTemplate(
    UStep<NestedBranchSpareSpec>,
    NestedLeafScopedTemplate,
    UStep<NestedBranchSpareSpec>,
);

struct NestedScopeAnimal;
#[animal(observe)]
impl Animal for NestedScopeAnimal {
    type Id = Id<U53>;
    type Generation = U0;
    type State = NestedLensRootState;
    type Seed = i32;
    type Journey = <NestedBranchScopedTemplate as BindAnimal<NestedScopeAnimal>>::Bound;
}

impl Observe for NestedScopeAnimal {
    type Appearance = NestedLensRootState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct NestedScopeAnimals(NestedScopeAnimal);

struct NestedScopeZoo;
impl Ecosystem for NestedScopeZoo {
    const NAME: &'static str = "late-bound-nested-view-scoped-zoo";
    type Animals = NestedScopeAnimals;
}

#[tokio::test]
async fn template_binding_nested_view_scopes_with_multiple_steps_run_end_to_end() {
    type LeafScopedTraverse = <NestedLeafScopedTemplate as TraverseFlow>::Output;
    type LeafScopedExpected =
        Scoped<NestedLensLeaf, <NestedLeafScopedTemplate as ReplaceFlow>::Output>;
    assert_type_eq!(LeafScopedTraverse, LeafScopedExpected);

    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-nested-view-scoped-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(NestedScopeZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .start_journey::<NestedScopeAnimal>(
            postcard::to_allocvec(&3_i32).expect("seed should serialize"),
        )
        .await
        .expect("journey should start");

    await_completion(&client, journey_id).await;

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: NestedLensRootState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");

    // Step chain with nested scopes (seed=3):
    // branch-level and leaf-level scoped steps both execute end-to-end.
    // Current scoped carry semantics produce:
    // leaf value => 5, leaf noise => 6, branch spare => 11.
    assert_eq!(appearance.branch.spare, 11);
    assert_eq!(appearance.branch.leaf.value, 5);
    assert_eq!(appearance.branch.leaf.noise, 6);
    assert_eq!(appearance.committed, 0);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ComplexAlphaState {
    loops: u8,
    shared_work: i32,
    join_sum: i32,
    select_winner: i32,
    unique_alpha: i32,
    final_value: i32,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ComplexBetaState {
    iterations: u8,
    core: i32,
    joined: i32,
    raced: i32,
    unique_beta: i32,
    done: i32,
    beta_flag: bool,
}

pub struct ComplexTimedEffect;
impl EffectSchema for ComplexTimedEffect {
    type Id = Id<U49>;
    type In = (u64, i32);
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for ComplexTimedEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

trait ComplexFlowBinding: Animal {
    fn inc_loop(state: &mut Self::State);
    fn set_shared_work(state: &mut Self::State, value: i32);
    fn set_join_sum(state: &mut Self::State, value: i32);
    fn set_select_winner(state: &mut Self::State, value: i32);
    fn set_unique_alpha(state: &mut Self::State, value: i32);
    fn set_unique_beta(state: &mut Self::State, value: i32);
    fn set_final(state: &mut Self::State, value: i32);
    fn fast_ms() -> u64;
    fn slow_ms() -> u64;
}

pub struct SharedMeta;
impl NodeMetadata for SharedMeta {
    const METADATA: &'static str = "segment:shared/transparent";
}

pub struct KeepLoopingShared;
impl LoopCondition<ComplexAlphaState> for KeepLoopingShared {
    type Arg = i32;

    fn should_continue(state: &ComplexAlphaState) -> bool {
        state.loops < 2
    }
}

impl LoopCondition<ComplexBetaState> for KeepLoopingShared {
    type Arg = i32;

    fn should_continue(state: &ComplexBetaState) -> bool {
        state.iterations < 2
    }
}

pub struct ChooseUniqueAlpha;
impl Condition<(ComplexAlphaState, i32)> for ChooseUniqueAlpha {
    fn choose((_state, _): &(ComplexAlphaState, i32)) -> bool {
        true
    }
}

impl Condition<(ComplexBetaState, i32)> for ChooseUniqueAlpha {
    fn choose((_state, _): &(ComplexBetaState, i32)) -> bool {
        false
    }
}

pub struct JoinLeftSpec;
pub struct JoinRightSpec;
pub struct JoinToCarrySpec;
pub struct SelectFastSpec;
pub struct SelectSlowSpec;
pub struct SelectToCarrySpec;
pub struct LoopAdvanceSpec;
pub struct UniqueAlphaSpec;
pub struct UniqueBetaSpec;
pub struct FinalizeSpec;

pub struct JoinLeftAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for JoinLeftAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::fast_ms(), input + 1)
    }

    fn absorb(_state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join-left effect should succeed")
    }
}

pub struct JoinRightAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for JoinRightAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::slow_ms(), input + 2)
    }

    fn absorb(_state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join-right effect should succeed")
    }
}

pub struct JoinToCarryAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for JoinToCarryAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = (i32, i32);
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input.0 + input.1
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("join-to-carry should succeed");
        A::set_join_sum(state, value);
        value
    }
}

pub struct SelectFastAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for SelectFastAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::fast_ms(), input + 3)
    }

    fn absorb(_state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("select-fast effect should succeed")
    }
}

pub struct SelectSlowAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for SelectSlowAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::slow_ms(), input + 4)
    }

    fn absorb(_state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("select-slow effect should succeed")
    }
}

pub struct SelectToCarryAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for SelectToCarryAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("select-to-carry should succeed");
        A::set_select_winner(state, value);
        value
    }
}

pub struct LoopAdvanceAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for LoopAdvanceAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("loop-advance should succeed");
        A::set_shared_work(state, value);
        A::inc_loop(state);
        value
    }
}

pub struct UniqueAlphaAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for UniqueAlphaAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input + 100
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("unique-alpha should succeed");
        A::set_unique_alpha(state, value);
        value
    }
}

pub struct UniqueBetaAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for UniqueBetaAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input - 100
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("unique-beta should succeed");
        A::set_unique_beta(state, value);
        value
    }
}

pub struct FinalizeAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> Act<A> for FinalizeAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input
    }

    fn absorb(state: &mut A::State, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let value = output.expect("finalize should succeed");
        A::set_final(state, value);
        value
    }
}

impl ActionSpec for JoinLeftSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = JoinLeftAct<A>;
}

impl ActionSpec for JoinRightSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = JoinRightAct<A>;
}

impl ActionSpec for JoinToCarrySpec {
    type Effect = TemplateCommitEffect;
    type Input = (i32, i32);
    type Output = i32;
    type Act<A: Animal> = JoinToCarryAct<A>;
}

impl ActionSpec for SelectFastSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = SelectFastAct<A>;
}

impl ActionSpec for SelectSlowSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = SelectSlowAct<A>;
}

impl ActionSpec for SelectToCarrySpec {
    type Effect = TemplateCommitEffect;
    type Input = Either<i32, i32>;
    type Output = i32;
    type Act<A: Animal> = SelectToCarryAct<A>;
}

impl ActionSpec for LoopAdvanceSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = LoopAdvanceAct<A>;
}

impl ActionSpec for UniqueAlphaSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = UniqueAlphaAct<A>;
}

impl ActionSpec for UniqueBetaSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = UniqueBetaAct<A>;
}

impl ActionSpec for FinalizeSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
    type Act<A: Animal> = FinalizeAct<A>;
}

#[derive(jungle_sdk::FlowTemplate)]
struct SharedJoinBranch(
    Join<UStep<JoinLeftSpec>, UStep<JoinRightSpec>>,
    UStep<JoinToCarrySpec>,
);

#[derive(jungle_sdk::FlowTemplate)]
struct SharedSelectBranch(
    Select<UStep<SelectFastSpec>, UStep<SelectSlowSpec>>,
    UStep<SelectToCarrySpec>,
);

#[derive(jungle_sdk::FlowTemplate)]
struct SharedLoopBody(UStep<LoopAdvanceSpec>);

#[derive(jungle_sdk::FlowTemplate)]
struct SharedComposedSegment(
    While<KeepLoopingShared, SharedLoopBody>,
    SharedJoinBranch,
    SharedSelectBranch,
);

#[derive(jungle_sdk::FlowTemplate)]
struct LongSharedSegment(Transparent<SharedMeta, SharedComposedSegment>);

#[derive(jungle_sdk::FlowTemplate)]
struct UniqueSegment(Conditional<ChooseUniqueAlpha, UStep<UniqueAlphaSpec>, UStep<UniqueBetaSpec>>);

#[derive(jungle_sdk::FlowTemplate)]
struct LongMixedTemplate(LongSharedSegment, UniqueSegment, UStep<FinalizeSpec>);

struct ComplexAlphaAnimal;
#[animal(observe)]
impl Animal for ComplexAlphaAnimal {
    type Id = Id<U50>;
    type Generation = U0;
    type State = ComplexAlphaState;
    type Seed = i32;
    type Journey = <LongMixedTemplate as BindAnimal<ComplexAlphaAnimal>>::Bound;
}

impl ComplexFlowBinding for ComplexAlphaAnimal {
    fn inc_loop(state: &mut Self::State) {
        state.loops = state.loops.saturating_add(1);
    }

    fn set_shared_work(state: &mut Self::State, value: i32) {
        state.shared_work = value;
    }

    fn set_join_sum(state: &mut Self::State, value: i32) {
        state.join_sum = value;
    }

    fn set_select_winner(state: &mut Self::State, value: i32) {
        state.select_winner = value;
    }

    fn set_unique_alpha(state: &mut Self::State, value: i32) {
        state.unique_alpha = value;
    }

    fn set_unique_beta(_state: &mut Self::State, _value: i32) {}

    fn set_final(state: &mut Self::State, value: i32) {
        state.final_value = value;
    }

    fn fast_ms() -> u64 {
        1
    }

    fn slow_ms() -> u64 {
        20
    }
}

impl Observe for ComplexAlphaAnimal {
    type Appearance = ComplexAlphaState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

struct ComplexBetaAnimal;
#[animal(observe)]
impl Animal for ComplexBetaAnimal {
    type Id = Id<U51>;
    type Generation = U0;
    type State = ComplexBetaState;
    type Seed = i32;
    type Journey = <LongMixedTemplate as BindAnimal<ComplexBetaAnimal>>::Bound;
}

impl ComplexFlowBinding for ComplexBetaAnimal {
    fn inc_loop(state: &mut Self::State) {
        state.iterations = state.iterations.saturating_add(1);
    }

    fn set_shared_work(state: &mut Self::State, value: i32) {
        state.core = value;
    }

    fn set_join_sum(state: &mut Self::State, value: i32) {
        state.joined = value;
    }

    fn set_select_winner(state: &mut Self::State, value: i32) {
        state.raced = value;
    }

    fn set_unique_alpha(_state: &mut Self::State, _value: i32) {}

    fn set_unique_beta(state: &mut Self::State, value: i32) {
        state.unique_beta = value;
        state.beta_flag = true;
    }

    fn set_final(state: &mut Self::State, value: i32) {
        state.done = value;
    }

    fn fast_ms() -> u64 {
        1
    }

    fn slow_ms() -> u64 {
        20
    }
}

impl Observe for ComplexBetaAnimal {
    type Appearance = ComplexBetaState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct ComplexMixedAnimals(ComplexAlphaAnimal, ComplexBetaAnimal);

struct ComplexMixedZoo;
impl Ecosystem for ComplexMixedZoo {
    const NAME: &'static str = "late-bound-complex-mixed-zoo";
    type Animals = ComplexMixedAnimals;
}

#[tokio::test]
async fn template_binding_long_shared_and_unique_segments_with_different_animal_states_e2e() {
    let client = jungle_sdk::LocalClient::builder()
        .namespace("late-bound-complex-mixed-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(ComplexMixedZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .start_journey::<ComplexAlphaAnimal>(
            postcard::to_allocvec(&5_i32).expect("alpha seed should serialize"),
        )
        .await
        .expect("alpha journey should start");
    let beta_id = client
        .start_journey::<ComplexBetaAnimal>(
            postcard::to_allocvec(&5_i32).expect("beta seed should serialize"),
        )
        .await
        .expect("beta journey should start");

    await_completion(&client, alpha_id).await;
    await_completion(&client, beta_id).await;

    let alpha_appearance_bytes = client
        .animal_appearance(alpha_id)
        .await
        .expect("alpha appearance request should succeed")
        .expect("alpha appearance should exist");
    let beta_appearance_bytes = client
        .animal_appearance(beta_id)
        .await
        .expect("beta appearance request should succeed")
        .expect("beta appearance should exist");
    let alpha: ComplexAlphaState =
        postcard::from_bytes(&alpha_appearance_bytes).expect("alpha appearance should deserialize");
    let beta: ComplexBetaState =
        postcard::from_bytes(&beta_appearance_bytes).expect("beta appearance should deserialize");

    // Shared long segment assertions (Transparent + While + Join + Select).
    assert_eq!(alpha.loops, 2);
    assert_eq!(alpha.join_sum, 17);
    assert_eq!(alpha.select_winner, 20);
    assert_eq!(alpha.shared_work, 7);

    assert_eq!(beta.iterations, 2);
    assert_eq!(beta.joined, 17);
    assert_eq!(beta.raced, 20);
    assert_eq!(beta.core, 7);

    // Unique per-animal segment assertions.
    assert_eq!(alpha.unique_alpha, 120);
    assert_eq!(alpha.final_value, 120);
    assert_eq!(beta.unique_beta, -80);
    assert_eq!(beta.done, -80);
    assert!(beta.beta_flag);

    worker_handle.abort();
    let _ = worker_handle.await;
}
