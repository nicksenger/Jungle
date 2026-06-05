use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::{Animals, JungleClient, Optic};
use num::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub struct TemplateAddEffect;
#[jungle::effect(id = 40)]
impl<J> Effect<J> for TemplateAddEffect {
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

pub struct TemplateCommitEffect;
#[jungle::effect(id = 41)]
impl<J> Effect<J> for TemplateCommitEffect {
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

struct JoinConcurrentRuntime {
    barrier: Mutex<Arc<tokio::sync::Barrier>>,
    active: AtomicUsize,
    max_active: AtomicUsize,
}

impl JoinConcurrentRuntime {
    fn reset(&self, parties: usize) {
        self.active.store(0, Ordering::SeqCst);
        self.max_active.store(0, Ordering::SeqCst);
        *self
            .barrier
            .lock()
            .expect("join concurrent barrier lock should not be poisoned") =
            Arc::new(tokio::sync::Barrier::new(parties));
    }
}

fn join_concurrent_runtime() -> Arc<JoinConcurrentRuntime> {
    static RUNTIME: OnceLock<Arc<JoinConcurrentRuntime>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Arc::new(JoinConcurrentRuntime {
                barrier: Mutex::new(Arc::new(tokio::sync::Barrier::new(2))),
                active: AtomicUsize::new(0),
                max_active: AtomicUsize::new(0),
            })
        })
        .clone()
}

struct JoinConcurrentGuard(Arc<JoinConcurrentRuntime>);

impl Drop for JoinConcurrentGuard {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct TemplateConcurrentJoinEffect;
#[jungle::effect(id = 142)]
impl<J> Effect<J> for TemplateConcurrentJoinEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let runtime = join_concurrent_runtime();
            let active = runtime.active.fetch_add(1, Ordering::SeqCst) + 1;
            runtime.max_active.fetch_max(active, Ordering::SeqCst);
            let _guard = JoinConcurrentGuard(runtime.clone());
            let barrier = runtime
                .barrier
                .lock()
                .expect("join concurrent barrier lock should not be poisoned")
                .clone();
            barrier.wait().await;
            Ok(input)
        }
    }
}

pub struct AddOneSpec;
pub struct CommitSpec;

#[jungle::action(bind = GenericAddOne<A>)]
impl Action for AddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = GenericCommit<A>)]
impl Action for CommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
struct TestFlow(Step<AddOneSpec>, Step<CommitSpec>);

struct CounterAddOneSpec;
#[jungle::action(bind = CounterAddOne<A>)]
impl Action for CounterAddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

struct CounterCommitSpec;
#[jungle::action(bind = CounterCommit<A>)]
impl Action for CounterCommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
struct CounterFlowTemplate(Step<CounterAddOneSpec>, Step<CounterCommitSpec>);

struct LedgerAddOneSpec;
#[jungle::action(bind = LedgerAddOne<A>)]
impl Action for LedgerAddOneSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

struct LedgerCommitSpec;
#[jungle::action(bind = LedgerCommit<A>)]
impl Action for LedgerCommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
struct LedgerFlowTemplate(Step<LedgerAddOneSpec>, Step<LedgerCommitSpec>);

struct CounterAnimal;
#[jungle::animal(id = 42, generation = 0)]
impl Animal for CounterAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = CounterFlowTemplate;
}

impl LateBoundPolicy for CounterAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LedgerAnimal;
#[jungle::animal(id = 43, generation = 0)]
impl Animal for LedgerAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = LedgerFlowTemplate;
}

impl LateBoundPolicy for LedgerAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct CounterAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for CounterAddOne<A>
where
    A: Animal<State = i32>,
{
    const NAME: &'static str = "CounterAddOne";
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 1
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let value = output.map_err(|_err| Failure::from("counter add step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_1)
    }
}

struct LedgerAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for LedgerAddOne<A>
where
    A: Animal<State = i32>,
{
    const NAME: &'static str = "LedgerAddOne";
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 10
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            let value = output.map_err(|_err| Failure::from("ledger add step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_2)
    }
}

struct CounterCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for CounterCommit<A>
where
    A: Animal<State = i32>,
{
    const NAME: &'static str = "CounterCommit";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            let value =
                output.map_err(|_err| Failure::from("counter commit step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_3)
    }
}

struct LedgerCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for LedgerCommit<A>
where
    A: Animal<State = i32>,
{
    const NAME: &'static str = "LedgerCommit";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state - input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            let value =
                output.map_err(|_err| Failure::from("ledger commit step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_4)
    }
}

pub struct GenericAddOne<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for GenericAddOne<A>
where
    A: Animal<State = i32> + LateBoundPolicy,
{
    const NAME: &'static str = "GenericAddOne";
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + A::ADD_INPUT_DELTA
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
            let value = output.map_err(|_err| Failure::from("generic add step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_5)
    }
}

pub struct GenericCommit<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for GenericCommit<A>
where
    A: Animal<State = i32> + LateBoundPolicy,
{
    const NAME: &'static str = "GenericCommit";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(state: &i32, input: Self::Input) -> i32 {
        if A::COMMIT_SUBTRACT {
            *state - input
        } else {
            *state + input
        }
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_6 = {
            let value =
                output.map_err(|_err| Failure::from("generic commit step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_6)
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
    assert_eq!(counter_request_2, 10);
    let counter_emitted_2: i32 = counter
        .complete_typed::<i32, (), i32>(Ok(10))
        .expect("counter second completion");
    assert_eq!(counter_emitted_2, 10);
    assert_eq!(counter.into_state(), 10);

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
    assert_eq!(ledger_request_2, 0);
    let ledger_emitted_2: i32 = ledger
        .complete_typed::<i32, (), i32>(Ok(0))
        .expect("ledger second completion");
    assert_eq!(ledger_emitted_2, 0);
    assert_eq!(ledger.into_state(), 0);
}

#[test]
fn template_binding_preserves_step_shape_after_binding() {
    type CounterBound = <TestFlow as BindAnimal<CounterAnimal>>::Bound;
    type LedgerBound = <TestFlow as BindAnimal<LedgerAnimal>>::Bound;
    type ExpectedCounter = jungle_sdk::typosaurus::collections::list::List<(
        BoundFlowStep<CounterAnimal, GenericAddOne<CounterAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            BoundFlowStep<CounterAnimal, GenericCommit<CounterAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;
    type ExpectedLedger = jungle_sdk::typosaurus::collections::list::List<(
        BoundFlowStep<LedgerAnimal, GenericAddOne<LedgerAnimal>>,
        jungle_sdk::typosaurus::collections::list::List<(
            BoundFlowStep<LedgerAnimal, GenericCommit<LedgerAnimal>>,
            jungle_sdk::typosaurus::collections::list::Empty,
        )>,
    )>;

    let _counter_step_1: BoundFlowStep<CounterAnimal, CounterAddOne<CounterAnimal>> =
        BoundFlowStep::new();
    let _counter_step_2: BoundFlowStep<CounterAnimal, CounterCommit<CounterAnimal>> =
        BoundFlowStep::new();
    let _ledger_step_1: BoundFlowStep<LedgerAnimal, LedgerAddOne<LedgerAnimal>> =
        BoundFlowStep::new();
    let _ledger_step_2: BoundFlowStep<LedgerAnimal, LedgerCommit<LedgerAnimal>> =
        BoundFlowStep::new();

    assert_type_eq!(CounterBound, ExpectedCounter);
    assert_type_eq!(LedgerBound, ExpectedLedger);
}

struct BoundTemplateAnimal;
#[jungle::animal(id = 44, generation = 0)]
impl Animal for BoundTemplateAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = TestFlow;
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
    assert_eq!(req_2, 12);
    let out_2: i32 = executor
        .complete_typed::<i32, (), i32>(Ok(12))
        .expect("second bound completion");
    assert_eq!(out_2, 12);
    assert_eq!(executor.into_state(), 12);
}

struct LocalTemplateAlphaAnimal;
#[jungle::animal(id = 45, generation = 0)]
impl Animal for LocalTemplateAlphaAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = TestFlow;
}

impl LateBoundPolicy for LocalTemplateAlphaAnimal {
    const ADD_INPUT_DELTA: i32 = 1;
    const COMMIT_SUBTRACT: bool = false;
}

struct LocalTemplateBetaAnimal;
#[jungle::animal(id = 46, generation = 0)]
impl Animal for LocalTemplateBetaAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = TestFlow;
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
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-local-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LocalTemplateZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .spawn::<LocalTemplateAlphaAnimal>(&3_i32)
        .await
        .expect("alpha journey should start")
        .journey_id;
    let beta_id = client
        .spawn::<LocalTemplateBetaAnimal>(&3_i32)
        .await
        .expect("beta journey should start")
        .journey_id;

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

impl RequiresContextBump for () {
    fn context_bump(&self) -> i32 {
        0
    }
}

pub struct ContextBoundEffect;
#[jungle::effect(id = 53)]
impl<J> Effect<J> for ContextBoundEffect
where
    J: RequiresContextBump,
{
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
#[jungle::action(bind = ContextBoundAct<A>)]
impl Action for ContextBoundSpec {
    type Effect = ContextBoundEffect;
    type Input = i32;
    type Output = i32;
}

struct ContextBoundAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for ContextBoundAct<A>
where
    A: Animal<State = i32>,
{
    const NAME: &'static str = "ContextBoundAct";
    type Effect = ContextBoundEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_7 = {
            let value =
                output.map_err(|_err| Failure::from("context-bound step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_7)
    }
}

#[derive(Flow)]
struct ContextBoundFlow(Step<ContextBoundSpec>);

struct LocalTemplateContextAnimal;
#[jungle::animal(id = 54, generation = 0)]
impl Animal for LocalTemplateContextAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = ContextBoundFlow;
}

#[derive(Animals)]
struct LocalTemplateContextAnimals(LocalTemplateContextAnimal);

struct LocalTemplateContextZoo;
impl Ecosystem for LocalTemplateContextZoo {
    const NAME: &'static str = "late-bound-context-bound-template-zoo";
    type Animals = LocalTemplateContextAnimals;
}

impl RequiresContextBump for &LocalTemplateContextZoo {
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
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-context-bound-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LocalTemplateContextZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<LocalTemplateContextAnimal>(&4_i32)
        .await
        .expect("journey should start")
        .journey_id;

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

async fn await_completion(client: &jungle_sdk::FusedClient, journey_id: uuid::Uuid) {
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
    use jungle_sdk::prelude::*;

    // Fragment A: no Animal type appears here.
    #[derive(Flow)]
    pub struct IntakeStage(Step<AddOneSpec>);

    // Fragment B: no Animal type appears here.
    #[derive(Flow)]
    pub struct CommitStage(Step<CommitSpec>);

    // Final composition of independent unbound fragments, still no Animal type.
    #[derive(Flow)]
    pub struct ComposedPipeline(IntakeStage, CommitStage);
}

struct ComposedAlphaAnimal;
#[jungle::animal(id = 47, generation = 0)]
impl Animal for ComposedAlphaAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = composed_templates::ComposedPipeline;
}

impl LateBoundPolicy for ComposedAlphaAnimal {
    const ADD_INPUT_DELTA: i32 = 2;
    const COMMIT_SUBTRACT: bool = false;
}

struct ComposedBetaAnimal;
#[jungle::animal(id = 48, generation = 0)]
impl Animal for ComposedBetaAnimal {
    type State = i32;
    type Seed = i32;
    type Flow = composed_templates::ComposedPipeline;
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
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-composed-template-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(ComposedTemplateZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .spawn::<ComposedAlphaAnimal>(&3_i32)
        .await
        .expect("alpha journey should start")
        .journey_id;
    let beta_id = client
        .spawn::<ComposedBetaAnimal>(&3_i32)
        .await
        .expect("beta journey should start")
        .journey_id;

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
impl<A> BoundAction<A> for LensReadSpareAct<A>
where
    A: Animal<State = LensRootState>,
{
    const NAME: &'static str = "LensReadSpareAct";
    type Effect = TemplateAddEffect;
    type Aspect = LensRootSpareCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_8 = {
            let out = output.map_err(|_err| Failure::from("lens spare step should succeed"))?;
            *view = out;
            out
        };
        Ok(__absorb_out_8)
    }
}

struct LensReadLeafAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for LensReadLeafAct<A>
where
    A: Animal<State = LensRootState>,
{
    const NAME: &'static str = "LensReadLeafAct";
    type Effect = TemplateAddEffect;
    type Aspect = LensRootLeafValueCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_9 = {
            let out = output.map_err(|_err| Failure::from("lens leaf step should succeed"))?;
            *view = out;
            out
        };
        Ok(__absorb_out_9)
    }
}

struct LensCommitAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for LensCommitAct<A>
where
    A: Animal<State = LensRootState>,
{
    const NAME: &'static str = "LensCommitAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &LensRootState, input: Self::Input) -> i32 {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut LensRootState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_10 = {
            let out = output.map_err(|_err| Failure::from("lens commit should succeed"))?;
            state.committed = out;
            out
        };
        Ok(__absorb_out_10)
    }
}

#[jungle::action(bind = LensReadSpareAct<A>)]
impl Action for LensReadSpareSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = LensReadLeafAct<A>)]
impl Action for LensReadLeafSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = LensCommitAct<A>)]
impl Action for LensCommitSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[derive(Flow)]
struct LensFlow(Step<LensReadSpareSpec>, Step<LensCommitSpec>);

struct SeenStep<T>(core::marker::PhantomData<T>);
struct LensTraversal;
impl<S> TraverseStep<jungle_sdk::types::Step<S>> for LensTraversal
where
    S: Action,
{
    type Output = SeenStep<jungle_sdk::types::Step<S>>;
}

struct LensReplacer;
impl ReplaceStep<jungle_sdk::types::Step<LensReadSpareSpec>> for LensReplacer {
    type Output = jungle_sdk::types::Step<LensReadLeafSpec>;
}
impl ReplaceStep<jungle_sdk::types::Step<LensReadLeafSpec>> for LensReplacer {
    type Output = jungle_sdk::types::Step<LensReadSpareSpec>;
}
impl ReplaceStep<jungle_sdk::types::Step<LensCommitSpec>> for LensReplacer {
    type Output = jungle_sdk::types::Step<LensCommitSpec>;
}

struct LensAlphaAnimal;
#[jungle::animal(observe, id = 52, generation = 0)]
impl Animal for LensAlphaAnimal {
    type State = LensRootState;
    type Seed = i32;
    type Flow = LensFlow;
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

#[derive(Flow)]
#[jungle(focus = LensBranch)]
struct ScopedLensFlow(LensFlow);

#[derive(Flow)]
#[jungle(focus = LensBranch)]
struct ScopedLensMultiField(LensFlow, LensFlow);

struct GenericFocus<T>(core::marker::PhantomData<fn() -> T>);

#[derive(Flow)]
#[jungle(focus = GenericFocus<LensBranch>)]
struct GenericScopedLensFlow(LensFlow);

#[test]
fn template_binding_unbound_flow_supports_traverse_and_replace_with_lens_specs() {
    type Traversed = jungle_sdk::types::Traversed<LensFlow, LensTraversal>;
    type ExpectedTraversed = jungle_sdk::typosaurus::list![
        SeenStep<jungle_sdk::types::Step<LensReadSpareSpec>>,
        SeenStep<jungle_sdk::types::Step<LensCommitSpec>>
    ];
    assert_type_eq!(Traversed, ExpectedTraversed);

    type Replaced = jungle_sdk::types::Replace<LensFlow, LensReplacer>;
    type ExpectedReplaced = jungle_sdk::typosaurus::list![
        jungle_sdk::types::Step<LensReadLeafSpec>,
        jungle_sdk::types::Step<LensCommitSpec>
    ];
    assert_type_eq!(Replaced, ExpectedReplaced);

    type ScopedTraverse = <ScopedLensFlow as TraverseFlow>::Output;
    type ScopedExpected = Scoped<
        LensBranch,
        jungle_sdk::typosaurus::list![
            jungle_sdk::types::Step<LensReadSpareSpec>,
            jungle_sdk::types::Step<LensCommitSpec>
        ],
    >;
    assert_type_eq!(ScopedTraverse, ScopedExpected);

    type ScopedMultiTraverse = <ScopedLensMultiField as TraverseFlow>::Output;
    type ScopedMultiExpected = Scoped<
        LensBranch,
        jungle_sdk::typosaurus::list![
            jungle_sdk::types::Step<LensReadSpareSpec>,
            jungle_sdk::types::Step<LensCommitSpec>,
            jungle_sdk::types::Step<LensReadSpareSpec>,
            jungle_sdk::types::Step<LensCommitSpec>
        ],
    >;
    assert_type_eq!(ScopedMultiTraverse, ScopedMultiExpected);

    type GenericScopedTraverse = <GenericScopedLensFlow as TraverseFlow>::Output;
    type GenericScopedExpected = Scoped<
        GenericFocus<LensBranch>,
        jungle_sdk::typosaurus::list![
            jungle_sdk::types::Step<LensReadSpareSpec>,
            jungle_sdk::types::Step<LensCommitSpec>
        ],
    >;
    assert_type_eq!(GenericScopedTraverse, GenericScopedExpected);

    type GenericScopedView = <GenericScopedLensFlow as FlowScope>::View;
    type GenericScopedViewExpected = FlowView<GenericFocus<LensBranch>>;
    assert_type_eq!(GenericScopedView, GenericScopedViewExpected);
}

#[tokio::test]
async fn template_binding_unbound_lens_template_runs_end_to_end() {
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-lens-replace-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(LensZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<LensAlphaAnimal>(&30_i32)
        .await
        .expect("journey should start")
        .journey_id;

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
    #[jungle(focus)]
    leaf: NestedLensLeaf,
    spare: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedLensRootState {
    #[jungle(focus)]
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
struct NestedAutoBranchSpec;

type NestedBranchSpareCarrier = Lens<NestedLensBranch, U1>;
type NestedLeafValueCarrier = Lens<NestedLensLeaf, U0>;
type NestedLeafNoiseCarrier = Lens<NestedLensLeaf, U1>;

struct NestedBranchSpareAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for NestedBranchSpareAct<A>
where
    A: Animal<State = NestedLensBranch>,
{
    const NAME: &'static str = "NestedBranchSpareAct";
    type Effect = TemplateAddEffect;
    type Aspect = NestedBranchSpareCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_11 = {
            let out =
                output.map_err(|_err| Failure::from("nested branch spare step should succeed"))?;
            *view = out;
            out
        };
        Ok(__absorb_out_11)
    }
}

struct NestedLeafValueAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for NestedLeafValueAct<A>
where
    A: Animal<State = NestedLensLeaf>,
{
    const NAME: &'static str = "NestedLeafValueAct";
    type Effect = TemplateAddEffect;
    type Aspect = NestedLeafValueCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_12 = {
            let out =
                output.map_err(|_err| Failure::from("nested leaf value step should succeed"))?;
            *view = out;
            out
        };
        Ok(__absorb_out_12)
    }
}

struct NestedLeafNoiseAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for NestedLeafNoiseAct<A>
where
    A: Animal<State = NestedLensLeaf>,
{
    const NAME: &'static str = "NestedLeafNoiseAct";
    type Effect = TemplateAddEffect;
    type Aspect = NestedLeafNoiseCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_13 = {
            let out =
                output.map_err(|_err| Failure::from("nested leaf noise step should succeed"))?;
            *view = out;
            out
        };
        Ok(__absorb_out_13)
    }
}

#[jungle::action(bind = NestedBranchSpareAct<A>)]
impl Action for NestedBranchSpareSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = NestedLeafValueAct<A>)]
impl Action for NestedLeafValueSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = NestedLeafNoiseAct<A>)]
impl Action for NestedLeafNoiseSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[allow(private_interfaces)]
#[jungle::action]
impl Action for NestedAutoBranchSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &NestedLensBranch, input: Self::Input) -> i32 {
        state.spare + input
    }

    fn absorb(
        state: &mut NestedLensBranch,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_14 = {
            let out =
                output.map_err(|_err| Failure::from("nested auto branch step should succeed"))?;
            state.spare = out;
            out
        };
        Ok(__absorb_out_14)
    }
}

#[derive(Flow)]
#[jungle(focus = NestedLensLeaf)]
struct NestedLeafScopedFlow(Step<NestedLeafValueSpec>, Step<NestedLeafNoiseSpec>);

#[derive(Flow)]
#[jungle(focus = NestedLensBranch)]
struct NestedBranchScopedFlow(
    Step<NestedBranchSpareSpec>,
    NestedLeafScopedFlow,
    Step<NestedBranchSpareSpec>,
);

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GenericBranchFocus<T> {
    #[jungle(focus)]
    branch: T,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GenericNestedRootState {
    #[jungle(focus)]
    wrapped: GenericBranchFocus<NestedLensBranch>,
}

impl From<i32> for GenericNestedRootState {
    fn from(seed: i32) -> Self {
        Self {
            wrapped: GenericBranchFocus {
                branch: NestedLensBranch {
                    leaf: NestedLensLeaf::default(),
                    spare: seed,
                },
            },
        }
    }
}

#[derive(Flow)]
#[jungle(focus = NestedLensBranch)]
struct GenericConcreteNestedFlow(Step<NestedAutoBranchSpec>);

#[derive(Flow)]
#[jungle(focus = GenericBranchFocus<NestedLensBranch>)]
struct GenericFocusedOuterFlow(GenericConcreteNestedFlow);

struct GenericNestedScopeAnimal;
#[jungle::animal(observe, id = 56, generation = 0)]
impl Animal for GenericNestedScopeAnimal {
    type State = GenericNestedRootState;
    type Seed = i32;
    type Flow = GenericFocusedOuterFlow;
}

impl Observe for GenericNestedScopeAnimal {
    type Appearance = GenericNestedRootState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct GenericNestedScopeAnimals(GenericNestedScopeAnimal);

struct GenericNestedScopeZoo;
impl Ecosystem for GenericNestedScopeZoo {
    const NAME: &'static str = "late-bound-generic-nested-scope-zoo";
    type Animals = GenericNestedScopeAnimals;
}

struct NestedScopeAnimal;
#[jungle::animal(observe, id = 53, generation = 0)]
impl Animal for NestedScopeAnimal {
    type State = NestedLensRootState;
    type Seed = i32;
    type Flow = NestedBranchScopedFlow;
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
    type LeafScopedTraverse = <NestedLeafScopedFlow as TraverseFlow>::Output;
    type LeafScopedExpected = Scoped<NestedLensLeaf, <NestedLeafScopedFlow as ReplaceFlow>::Output>;
    assert_type_eq!(LeafScopedTraverse, LeafScopedExpected);

    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-nested-view-scoped-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(NestedScopeZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<NestedScopeAnimal>(&3_i32)
        .await
        .expect("journey should start")
        .journey_id;

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

#[test]
fn template_binding_generic_focus_supports_nested_concrete_focus() {
    type NestedTraverse = <GenericConcreteNestedFlow as TraverseFlow>::Output;
    type NestedExpected = Scoped<
        NestedLensBranch,
        jungle_sdk::typosaurus::list![jungle_sdk::types::Step<NestedAutoBranchSpec>],
    >;
    assert_type_eq!(NestedTraverse, NestedExpected);

    type OuterTraverse = <GenericFocusedOuterFlow as TraverseFlow>::Output;
    type OuterExpected = Scoped<
        GenericBranchFocus<NestedLensBranch>,
        jungle_sdk::typosaurus::list![Scoped<
            NestedLensBranch,
            jungle_sdk::typosaurus::list![jungle_sdk::types::Step<NestedAutoBranchSpec>],
        >],
    >;
    assert_type_eq!(OuterTraverse, OuterExpected);

    let mut exec = ManualExecutor::<GenericNestedScopeAnimal>::new(GenericNestedRootState::from(3));
    let emitted: i32 = exec
        .next_typed(2, Ok::<i32, ()>(6))
        .expect("generic focused nested flow should complete");
    assert_eq!(emitted, 6);

    let state = exec.into_state();
    assert_eq!(state.wrapped.branch.spare, 6);
}

#[tokio::test]
async fn template_binding_generic_focus_nested_concrete_focus_runs_end_to_end_local() {
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-generic-nested-scope-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(GenericNestedScopeZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<GenericNestedScopeAnimal>(&3_i32)
        .await
        .expect("journey should start")
        .journey_id;

    await_completion(&client, journey_id).await;

    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey history should be available");
    let effect_inputs = decode_effect_inputs(&history);
    assert_eq!(effect_inputs, vec![3]);
    assert_eq!(effect_inputs.len(), 1);

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: GenericNestedRootState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");
    assert_eq!(appearance.wrapped.branch.spare, 4);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Loop2Container<St> {
    counter: usize,
    #[jungle(focus)]
    st: St,
}

impl<St> ViewProject<Loop2Container<St>> for Loop2Container<St> {
    fn project_view(state: &mut Self) -> &mut Loop2Container<St> {
        state
    }
}

struct Loop2SetCounterTo2Spec<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for Loop2SetCounterTo2Spec<St> {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &Loop2Container<St>, input: Self::Input) -> i32 {
        input
    }

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_15 = {
            state.counter = 2;
            output.map_err(|_err| Failure::from("loop2 set-counter step should succeed"))?
        };
        Ok(__absorb_out_15)
    }
}

struct Loop2DecrementCounterSpec<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for Loop2DecrementCounterSpec<St> {
    type Effect = TemplateCommitEffect;
    type Input = Either<i32, i32>;
    type Output = (bool, i32);

    fn emit(_state: &Loop2Container<St>, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_16 = {
            let value =
                output.map_err(|_err| Failure::from("loop2 decrement step should succeed"))?;
            state.counter = state.counter.saturating_sub(1);
            (state.counter > 0, value)
        };
        Ok(__absorb_out_16)
    }
}

struct Loop2CounterGt0;
impl<St> Predicate<(&Loop2Container<St>, &i32)> for Loop2CounterGt0 {
    fn eval((state, _): &(&Loop2Container<St>, &i32)) -> bool {
        state.counter > 0
    }
}

struct Loop2CounterIsEven;
impl<St> Predicate<(Loop2Container<St>, i32)> for Loop2CounterIsEven {
    fn eval((state, _): &(Loop2Container<St>, i32)) -> bool {
        state.counter % 2 == 0
    }
}

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
struct Loop2Body<St, L: TraverseFlow, R: TraverseFlow>(
    Conditional<FocusedCondition<Loop2CounterIsEven, Loop2Container<St>>, L, R>,
    Step<Loop2DecrementCounterSpec<St>>,
);

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
struct Loop2<St, L: TraverseFlow, R: TraverseFlow>(
    Step<Loop2SetCounterTo2Spec<St>>,
    While<FocusedLoopCondition<Loop2CounterGt0, Loop2Container<St>>, Loop2Body<St, L, R>>,
);

struct Loop2LeftSpec;
#[jungle::action]
impl Action for Loop2LeftSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 10
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_17 = {
            let value = output.map_err(|_err| Failure::from("loop2 left step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_17)
    }
}

struct Loop2RightSpec;
#[jungle::action]
impl Action for Loop2RightSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 100
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_18 = {
            let value = output.map_err(|_err| Failure::from("loop2 right step should succeed"))?;
            *state = value;
            value
        };
        Ok(__absorb_out_18)
    }
}

#[derive(Flow)]
#[jungle(focus = i32)]
struct Loop2LeftFlow(Step<Loop2LeftSpec>);

#[derive(Flow)]
#[jungle(focus = i32)]
struct Loop2RightFlow(Step<Loop2RightSpec>);

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Loop2HostState {
    #[jungle(focus)]
    loop2: Loop2Container<i32>,
    marker: i32,
}

impl From<i32> for Loop2HostState {
    fn from(seed: i32) -> Self {
        Self {
            loop2: Loop2Container {
                counter: 0,
                st: seed,
            },
            marker: -1,
        }
    }
}

type Loop2Journey = Loop2<i32, Loop2LeftFlow, Loop2RightFlow>;

struct Loop2CompositeAnimal;
#[jungle::animal(observe, id = 57, generation = 0)]
impl Animal for Loop2CompositeAnimal {
    type State = Loop2HostState;
    type Seed = i32;
    type Flow = Loop2Journey;
}

impl Observe for Loop2CompositeAnimal {
    type Appearance = Loop2HostState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct Loop2Animals(Loop2CompositeAnimal);

struct Loop2Zoo;
impl Ecosystem for Loop2Zoo {
    const NAME: &'static str = "late-bound-loop2-zoo";
    type Animals = Loop2Animals;
}

#[test]
fn template_binding_higher_order_generic_loop2_container_is_supported() {
    type Loop2View = <Loop2<i32, Loop2LeftFlow, Loop2RightFlow> as FlowScope>::View;
    type Loop2ViewExpected = FlowView<Loop2Container<i32>>;
    assert_type_eq!(Loop2View, Loop2ViewExpected);

    let mut exec = ManualExecutor::<Loop2CompositeAnimal>::new(Loop2HostState::from(5));

    let set_counter: i32 = exec
        .next_typed(1, Ok::<i32, ()>(1))
        .expect("loop2 set-counter step should complete");
    assert_eq!(set_counter, 1);
    assert_eq!(exec.state().loop2.counter, 2);

    let left_out: Either<i32, i32> = exec
        .next_typed(set_counter, Ok::<i32, ()>(11))
        .expect("loop2 left arm should run first");
    assert_eq!(left_out, Either::Left(11));
    assert_eq!(exec.state().loop2.st, 11);

    let after_left: (bool, i32) = exec
        .next_typed(left_out, Ok::<i32, ()>(11))
        .expect("loop2 first decrement should run");
    assert_eq!(after_left, (true, 11));
    assert_eq!(exec.state().loop2.counter, 1);

    let right_out: Either<i32, i32> = exec
        .next_typed(after_left, Ok::<i32, ()>(111))
        .expect("loop2 right arm should run second");
    assert_eq!(right_out, Either::Right(111));
    assert_eq!(exec.state().loop2.st, 111);

    let after_right: (bool, i32) = exec
        .next_typed(right_out, Ok::<i32, ()>(111))
        .expect("loop2 second decrement should run");
    assert_eq!(after_right, (false, 111));
    assert_eq!(exec.state().loop2.counter, 0);
    assert_eq!(exec.state().loop2.st, 111);
    assert_eq!(exec.state().marker, -1);
    let final_probe = exec.next_request(
        postcard::to_allocvec(&after_right).expect("loop2 final probe input should serialize"),
    );
    assert!(matches!(final_probe, Err(ExecutorError::Complete)));
    assert!(exec.is_complete());
}

#[tokio::test]
async fn template_binding_higher_order_generic_loop2_container_runs_end_to_end_local() {
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-loop2-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(Loop2Zoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<Loop2CompositeAnimal>(&5_i32)
        .await
        .expect("journey should start")
        .journey_id;

    await_completion(&client, journey_id).await;

    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey history should be available");
    let effect_inputs = decode_effect_inputs(&history);
    assert_eq!(effect_inputs, vec![5, 15, 15, 115, 115]);
    assert_eq!(effect_inputs.len(), 5);

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: Loop2HostState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");
    assert_eq!(appearance.loop2.st, 115);
    assert_eq!(appearance.loop2.counter, 0);
    assert_eq!(appearance.marker, 0);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NoEffectLoop2TraceState {
    left_hits: u8,
    right_hits: u8,
    order: u8,
}

struct NoEffectLoop2SetCounter<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for NoEffectLoop2SetCounter<St> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, _input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_19 = {
            state.counter = 2;
        };
        Ok(__absorb_out_19)
    }
}

struct NoEffectLoop2DecCounter<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for NoEffectLoop2DecCounter<St> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, _input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_20 = {
            output
                .map_err(|_err| Failure::from("no-effect loop2 decrement step should succeed"))?;
            state.counter = state.counter.saturating_sub(1);
        };
        Ok(__absorb_out_20)
    }
}

struct NoEffectFlattenEither<T, S>(core::marker::PhantomData<fn() -> (T, S)>);
#[jungle::action]
impl<T, S> Action for NoEffectFlattenEither<T, S> {
    type Effect = NoEffect;
    type Input = Either<T, T>;
    type Output = T;
    type Carry = Either<T, T>;

    fn emit(_state: &S, input: Self::Input) -> ((), Either<T, T>) {
        ((), input)
    }

    fn absorb(
        _state: &mut S,
        output: EffectCompletion<Self::Effect>,
        carry: Either<T, T>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_21 = {
            output.map_err(|_err| Failure::from("no-effect loop2 flatten step should succeed"))?;
            match carry {
                Either::Left(value) | Either::Right(value) => value,
            }
        };
        Ok(__absorb_out_21)
    }
}

struct NoEffectLoop2CounterGt0;
impl<St> Predicate<(&Loop2Container<St>, &())> for NoEffectLoop2CounterGt0 {
    fn eval((state, _): &(&Loop2Container<St>, &())) -> bool {
        state.counter > 0
    }
}

struct NoEffectLoop2CounterIsEven;
impl<St> Predicate<(Loop2Container<St>, ())> for NoEffectLoop2CounterIsEven {
    fn eval((state, _): &(Loop2Container<St>, ())) -> bool {
        state.counter % 2 == 0
    }
}

struct NoEffectLoop2LeftSpec;
#[jungle::action]
impl Action for NoEffectLoop2LeftSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &NoEffectLoop2TraceState, _input: Self::Input) {}

    fn absorb(
        state: &mut NoEffectLoop2TraceState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_22 = {
            output.map_err(|_err| Failure::from("no-effect loop2 left arm should succeed"))?;
            state.left_hits = state.left_hits.saturating_add(1);
            state.order = state.order.saturating_mul(10).saturating_add(1);
        };
        Ok(__absorb_out_22)
    }
}

struct NoEffectLoop2RightSpec;
#[jungle::action]
impl Action for NoEffectLoop2RightSpec {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &NoEffectLoop2TraceState, _input: Self::Input) {}

    fn absorb(
        state: &mut NoEffectLoop2TraceState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_23 = {
            output.map_err(|_err| Failure::from("no-effect loop2 right arm should succeed"))?;
            state.right_hits = state.right_hits.saturating_add(1);
            state.order = state.order.saturating_mul(10).saturating_add(2);
        };
        Ok(__absorb_out_23)
    }
}

#[derive(Flow)]
#[jungle(focus = NoEffectLoop2TraceState)]
struct NoEffectLoop2LeftFlow(Step<NoEffectLoop2LeftSpec>);

#[derive(Flow)]
#[jungle(focus = NoEffectLoop2TraceState)]
struct NoEffectLoop2RightFlow(Step<NoEffectLoop2RightSpec>);

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
struct NoEffectLoop2Body<St, L: TraverseFlow, R: TraverseFlow>(
    Conditional<FocusedCondition<NoEffectLoop2CounterIsEven, Loop2Container<St>>, L, R>,
    Step<NoEffectFlattenEither<(), Loop2Container<St>>>,
    Step<NoEffectLoop2DecCounter<St>>,
);

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
struct NoEffectLoop2<St, L: TraverseFlow, R: TraverseFlow>(
    Step<NoEffectLoop2SetCounter<St>>,
    While<
        FocusedLoopCondition<NoEffectLoop2CounterGt0, Loop2Container<St>>,
        NoEffectLoop2Body<St, L, R>,
    >,
);

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NoEffectLoop2HarnessState {
    #[jungle(focus)]
    loop2: Loop2Container<NoEffectLoop2TraceState>,
}

impl From<NoEffectLoop2TraceState> for NoEffectLoop2HarnessState {
    fn from(seed: NoEffectLoop2TraceState) -> Self {
        Self {
            loop2: Loop2Container {
                counter: 0,
                st: seed,
            },
        }
    }
}

type NoEffectLoop2Journey =
    NoEffectLoop2<NoEffectLoop2TraceState, NoEffectLoop2LeftFlow, NoEffectLoop2RightFlow>;

struct NoEffectLoop2HarnessAnimal;
#[jungle::animal(id = 58, generation = 0)]
impl Animal for NoEffectLoop2HarnessAnimal {
    type State = NoEffectLoop2HarnessState;
    type Seed = NoEffectLoop2TraceState;
    type Flow = NoEffectLoop2Journey;
}

#[test]
fn template_binding_no_effect_loop2_repro_completes_during_executor_init() {
    let mut exec = ManualExecutor::<NoEffectLoop2HarnessAnimal>::new(
        NoEffectLoop2HarnessState::from(NoEffectLoop2TraceState::default()),
    );

    assert!(!exec.is_complete());
    let step: Result<(), ExecutorError> = exec.next_typed((), Ok::<(), ()>(()));
    assert!(matches!(step, Err(ExecutorError::Complete)));
    assert!(exec.is_complete());

    let state = exec.into_state();
    assert_eq!(state.loop2.counter, 0);
    assert_eq!(state.loop2.st.left_hits, 1);
    assert_eq!(state.loop2.st.right_hits, 1);
    assert_eq!(state.loop2.st.order, 12);
}

#[derive(Flow)]
struct InheritedAutoFocusLeafFlow(Step<NestedAutoBranchSpec>);

#[derive(Flow)]
struct InheritedAutoFocusMiddleFlow(InheritedAutoFocusLeafFlow);

#[derive(Flow)]
#[jungle(focus = NestedLensBranch)]
struct InheritedAutoFocusRootFlow(InheritedAutoFocusMiddleFlow);

struct InheritedAutoFocusAnimal;
#[jungle::animal(observe, id = 54, generation = 0)]
impl Animal for InheritedAutoFocusAnimal {
    type State = NestedLensRootState;
    type Seed = i32;
    type Flow = InheritedAutoFocusRootFlow;
}

impl Observe for InheritedAutoFocusAnimal {
    type Appearance = NestedLensRootState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct InheritedAutoFocusAnimals(InheritedAutoFocusAnimal);

struct InheritedAutoFocusZoo;
impl Ecosystem for InheritedAutoFocusZoo {
    const NAME: &'static str = "late-bound-inherited-auto-focus-zoo";
    type Animals = InheritedAutoFocusAnimals;
}

#[tokio::test]
async fn template_binding_focus_is_inherited_through_unfocused_nested_flows_for_auto_act() {
    let mut exec = ManualExecutor::<InheritedAutoFocusAnimal>::new(NestedLensRootState::default());
    let req: i32 = exec
        .next_request_typed::<_, i32>(3)
        .expect("request should deserialize");
    assert_eq!(req, 3);
    let emitted: i32 = exec
        .complete_typed::<i32, (), i32>(Ok(4))
        .expect("completion should deserialize");
    assert_eq!(emitted, 4);
    assert!(
        exec.is_complete(),
        "single-step focused flow should complete"
    );
    let state = exec.into_state();
    assert_eq!(state.branch.spare, 4);

    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-inherited-auto-focus-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(InheritedAutoFocusZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<InheritedAutoFocusAnimal>(&3_i32)
        .await
        .expect("journey should start")
        .journey_id;

    await_completion(&client, journey_id).await;

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: NestedLensRootState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");

    assert_eq!(appearance.branch.spare, 4);

    worker_handle.abort();
    let _ = worker_handle.await;
}

pub struct InheritedControlCondition;
impl Predicate<(NestedLensRootState, i32)> for InheritedControlCondition {
    fn eval((_state, _): &(NestedLensRootState, i32)) -> bool {
        true
    }
}

#[derive(Flow)]
struct InheritedAutoFocusMiddleConditionalFlow(
    Conditional<InheritedControlCondition, InheritedAutoFocusLeafFlow, InheritedAutoFocusLeafFlow>,
);

#[derive(Flow)]
#[jungle(focus = NestedLensBranch)]
struct InheritedAutoFocusConditionalRootFlow(InheritedAutoFocusMiddleConditionalFlow);

struct InheritedAutoFocusConditionalAnimal;
#[jungle::animal(observe, id = 55, generation = 0)]
impl Animal for InheritedAutoFocusConditionalAnimal {
    type State = NestedLensRootState;
    type Seed = i32;
    type Flow = InheritedAutoFocusConditionalRootFlow;
}

impl Observe for InheritedAutoFocusConditionalAnimal {
    type Appearance = NestedLensRootState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct InheritedAutoFocusConditionalAnimals(InheritedAutoFocusConditionalAnimal);

struct InheritedAutoFocusConditionalZoo;
impl Ecosystem for InheritedAutoFocusConditionalZoo {
    const NAME: &'static str = "late-bound-inherited-auto-focus-conditional-zoo";
    type Animals = InheritedAutoFocusConditionalAnimals;
}

#[tokio::test]
async fn template_binding_focus_inheritance_does_not_duplicate_conditional_branch_progression() {
    let mut exec =
        ManualExecutor::<InheritedAutoFocusConditionalAnimal>::new(NestedLensRootState::default());
    let req: i32 = exec
        .next_request_typed::<_, i32>(3)
        .expect("request should deserialize");
    assert_eq!(req, 3);
    let emitted: Either<i32, i32> = exec
        .complete_typed::<i32, (), Either<i32, i32>>(Ok(4))
        .expect("completion should deserialize");
    assert_eq!(emitted, Either::Left(4));
    assert!(
        exec.is_complete(),
        "conditional wrapper should progress exactly one chosen branch"
    );
    let state = exec.into_state();
    assert_eq!(state.branch.spare, 4);

    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-inherited-auto-focus-conditional-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker =
        jungle_sdk::core::JungleWorker::new(InheritedAutoFocusConditionalZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<InheritedAutoFocusConditionalAnimal>(&3_i32)
        .await
        .expect("journey should start")
        .journey_id;

    await_completion(&client, journey_id).await;

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: NestedLensRootState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");

    assert_eq!(appearance.branch.spare, 4);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JoinFocusedLeftState {
    value: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JoinFocusedRightState {
    value: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct JoinFocusedHostState {
    #[jungle(focus)]
    left: JoinFocusedLeftState,
    #[jungle(focus)]
    right: JoinFocusedRightState,
    marker: i32,
}

struct JoinFocusedLeftSpec;
#[jungle::action]
impl Action for JoinFocusedLeftSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &JoinFocusedLeftState, input: Self::Input) -> i32 {
        state.value + input
    }

    fn absorb(
        state: &mut JoinFocusedLeftState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("focused left join step should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

struct JoinFocusedRightSpec;
#[jungle::action]
impl Action for JoinFocusedRightSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &JoinFocusedRightState, input: Self::Input) -> i32 {
        state.value + input * 10
    }

    fn absorb(
        state: &mut JoinFocusedRightState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("focused right join step should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

#[derive(Flow)]
#[jungle(focus = JoinFocusedLeftState)]
struct JoinFocusedLeftFlow(Step<JoinFocusedLeftSpec>);

#[derive(Flow)]
#[jungle(focus = JoinFocusedRightState)]
struct JoinFocusedRightFlow(Step<JoinFocusedRightSpec>);

#[derive(Flow)]
struct JoinFocusedTemplate(Join<JoinFocusedLeftFlow, JoinFocusedRightFlow>);

struct JoinFocusedAnimal;
#[jungle::animal(id = 88, generation = 0)]
impl Animal for JoinFocusedAnimal {
    type State = JoinFocusedHostState;
    type Seed = i32;
    type Flow = JoinFocusedTemplate;
}

struct JoinFocusedConcurrentLeftSpec;
#[jungle::action]
impl Action for JoinFocusedConcurrentLeftSpec {
    type Effect = TemplateConcurrentJoinEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &JoinFocusedLeftState, input: Self::Input) -> i32 {
        state.value + input
    }

    fn absorb(
        state: &mut JoinFocusedLeftState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("focused concurrent left should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

struct JoinFocusedConcurrentRightSpec;
#[jungle::action]
impl Action for JoinFocusedConcurrentRightSpec {
    type Effect = TemplateConcurrentJoinEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &JoinFocusedRightState, input: Self::Input) -> i32 {
        state.value + input * 10
    }

    fn absorb(
        state: &mut JoinFocusedRightState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out =
            output.map_err(|_err| Failure::from("focused concurrent right should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

#[derive(Flow)]
#[jungle(focus = JoinFocusedLeftState)]
struct JoinFocusedConcurrentLeftFlow(Step<JoinFocusedConcurrentLeftSpec>);

#[derive(Flow)]
#[jungle(focus = JoinFocusedRightState)]
struct JoinFocusedConcurrentRightFlow(Step<JoinFocusedConcurrentRightSpec>);

#[derive(Flow)]
struct JoinFocusedConcurrentTemplate(
    Join<JoinFocusedConcurrentLeftFlow, JoinFocusedConcurrentRightFlow>,
);

struct JoinFocusedConcurrentAnimal;
#[jungle::animal(id = 89, generation = 0)]
impl Animal for JoinFocusedConcurrentAnimal {
    type State = JoinFocusedHostState;
    type Seed = i32;
    type Flow = JoinFocusedConcurrentTemplate;
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedJoinFocusedLeftState {
    value: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedJoinFocusedMiddleState {
    value: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedJoinFocusedRightState {
    value: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedJoinFocusedHostState {
    #[jungle(focus)]
    left: NestedJoinFocusedLeftState,
    #[jungle(focus)]
    middle: NestedJoinFocusedMiddleState,
    #[jungle(focus)]
    right: NestedJoinFocusedRightState,
    marker: i32,
}

struct NestedJoinFocusedLeftSpec;
#[jungle::action]
impl Action for NestedJoinFocusedLeftSpec {
    type Effect = TemplateConcurrentJoinEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &NestedJoinFocusedLeftState, input: Self::Input) -> i32 {
        state.value + input
    }

    fn absorb(
        state: &mut NestedJoinFocusedLeftState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("nested focused left should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

struct NestedJoinFocusedMiddleSpec;
#[jungle::action]
impl Action for NestedJoinFocusedMiddleSpec {
    type Effect = TemplateConcurrentJoinEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &NestedJoinFocusedMiddleState, input: Self::Input) -> i32 {
        state.value + input * 10
    }

    fn absorb(
        state: &mut NestedJoinFocusedMiddleState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("nested focused middle should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

struct NestedJoinFocusedRightSpec;
#[jungle::action]
impl Action for NestedJoinFocusedRightSpec {
    type Effect = TemplateConcurrentJoinEffect;
    type Input = i32;
    type Output = i32;

    fn emit(state: &NestedJoinFocusedRightState, input: Self::Input) -> i32 {
        state.value + input * 100
    }

    fn absorb(
        state: &mut NestedJoinFocusedRightState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("nested focused right should succeed"))?;
        state.value = out;
        Ok(out)
    }
}

#[derive(Flow)]
#[jungle(focus = NestedJoinFocusedLeftState)]
struct NestedJoinFocusedLeftFlow(Step<NestedJoinFocusedLeftSpec>);

#[derive(Flow)]
#[jungle(focus = NestedJoinFocusedMiddleState)]
struct NestedJoinFocusedMiddleFlow(Step<NestedJoinFocusedMiddleSpec>);

#[derive(Flow)]
#[jungle(focus = NestedJoinFocusedRightState)]
struct NestedJoinFocusedRightFlow(Step<NestedJoinFocusedRightSpec>);

#[derive(Flow)]
struct NestedJoinFocusedPair(Join<NestedJoinFocusedLeftFlow, NestedJoinFocusedMiddleFlow>);

#[derive(Flow)]
struct NestedJoinFocusedTemplate(Join<NestedJoinFocusedPair, NestedJoinFocusedRightFlow>);

struct NestedJoinFocusedAnimal;
#[jungle::animal(id = 90, generation = 0)]
impl Animal for NestedJoinFocusedAnimal {
    type State = NestedJoinFocusedHostState;
    type Seed = i32;
    type Flow = NestedJoinFocusedTemplate;
}

#[test]
fn template_binding_join_merges_distinct_focused_branch_states() {
    let mut executor = Executor::<JoinFocusedAnimal>::new(JoinFocusedHostState {
        left: JoinFocusedLeftState { value: 1 },
        right: JoinFocusedRightState { value: 2 },
        marker: 99,
    });

    let emitted = futures::executor::block_on(executor.advance_to_end_with(3))
        .expect("focused join flow should complete");
    let final_emitted: (i32, i32) =
        postcard::from_bytes(emitted.last().expect("join should emit one final value"))
            .expect("join final emitted tuple should deserialize");

    assert_eq!(final_emitted, (4, 32));
    assert_eq!(
        executor.state(),
        &JoinFocusedHostState {
            left: JoinFocusedLeftState { value: 4 },
            right: JoinFocusedRightState { value: 32 },
            marker: 99,
        }
    );
}

#[tokio::test]
async fn template_binding_focused_join_subflows_run_concurrently() {
    let runtime = join_concurrent_runtime();
    runtime.reset(2);

    let mut executor = Executor::<JoinFocusedConcurrentAnimal>::new(JoinFocusedHostState {
        left: JoinFocusedLeftState { value: 1 },
        right: JoinFocusedRightState { value: 2 },
        marker: 99,
    });

    let request = executor
        .next_executable_request(3)
        .expect("focused join should produce an executable request");
    let completion = tokio::time::timeout(Duration::from_millis(250), request.run())
        .await
        .expect("focused join branches should rendezvous without serial deadlock")
        .expect("focused join effect runner should succeed");
    let emitted = executor
        .complete_serialized(completion)
        .expect("focused join completion should apply cleanly");
    let final_emitted: (i32, i32) =
        postcard::from_bytes(&emitted).expect("join final emitted tuple should deserialize");

    assert_eq!(runtime.max_active.load(Ordering::SeqCst), 2);
    assert_eq!(final_emitted, (4, 32));
    assert_eq!(
        executor.state(),
        &JoinFocusedHostState {
            left: JoinFocusedLeftState { value: 4 },
            right: JoinFocusedRightState { value: 32 },
            marker: 99,
        }
    );
}

#[tokio::test]
async fn template_binding_nested_focused_join_subflows_run_concurrently() {
    let runtime = join_concurrent_runtime();
    runtime.reset(3);

    let mut executor = Executor::<NestedJoinFocusedAnimal>::new(NestedJoinFocusedHostState {
        left: NestedJoinFocusedLeftState { value: 1 },
        middle: NestedJoinFocusedMiddleState { value: 2 },
        right: NestedJoinFocusedRightState { value: 3 },
        marker: 99,
    });

    let request = executor
        .next_executable_request(3)
        .expect("nested focused join should produce an executable request");
    let completion = tokio::time::timeout(Duration::from_millis(250), request.run())
        .await
        .expect("nested focused join branches should rendezvous without serial deadlock")
        .expect("nested focused join effect runner should succeed");
    let emitted = executor
        .complete_serialized(completion)
        .expect("nested focused join completion should apply cleanly");
    let final_emitted: ((i32, i32), i32) =
        postcard::from_bytes(&emitted).expect("nested focused join output should deserialize");

    assert_eq!(runtime.max_active.load(Ordering::SeqCst), 3);
    assert_eq!(final_emitted, ((4, 32), 303));
    assert_eq!(
        executor.state(),
        &NestedJoinFocusedHostState {
            left: NestedJoinFocusedLeftState { value: 4 },
            middle: NestedJoinFocusedMiddleState { value: 32 },
            right: NestedJoinFocusedRightState { value: 303 },
            marker: 99,
        }
    );
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalJoinMergeState {
    marker: i32,
    left_join_hits: u8,
    right_join_hits: u8,
    join_merge_hits: u8,
    terminal_merge_hits: u8,
}

pub struct PreferLeftWhenMarkerNonNegative;
impl Predicate<(ConditionalJoinMergeState, ())> for PreferLeftWhenMarkerNonNegative {
    fn eval((state, _): &(ConditionalJoinMergeState, ())) -> bool {
        state.marker != -1
    }
}

pub struct LeftJoinFirstSpec;
#[jungle::action]
impl Action for LeftJoinFirstSpec {
    type Effect = TemplateCommitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_24 = {
            output.map_err(|_err| Failure::from("left join first should succeed"))?;
            state.left_join_hits = state.left_join_hits.saturating_add(1);
        };
        Ok(__absorb_out_24)
    }
}

pub struct LeftJoinSecondSpec;
#[jungle::action]
impl Action for LeftJoinSecondSpec {
    type Effect = TemplateCommitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_25 = {
            output.map_err(|_err| Failure::from("left join second should succeed"))?;
            state.left_join_hits = state.left_join_hits.saturating_add(1);
        };
        Ok(__absorb_out_25)
    }
}

pub struct RightJoinFirstSpec;
#[jungle::action]
impl Action for RightJoinFirstSpec {
    type Effect = TemplateCommitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_26 = {
            output.map_err(|_err| Failure::from("right join first should succeed"))?;
            state.right_join_hits = state.right_join_hits.saturating_add(1);
        };
        Ok(__absorb_out_26)
    }
}

pub struct RightJoinSecondSpec;
#[jungle::action]
impl Action for RightJoinSecondSpec {
    type Effect = TemplateCommitEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_27 = {
            output.map_err(|_err| Failure::from("right join second should succeed"))?;
            state.right_join_hits = state.right_join_hits.saturating_add(1);
        };
        Ok(__absorb_out_27)
    }
}

pub struct MergeJoinedUnitSpec;
#[jungle::action]
impl Action for MergeJoinedUnitSpec {
    type Effect = TemplateCommitEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_28 = {
            output.map_err(|_err| Failure::from("merge joined unit should succeed"))?;
            state.join_merge_hits = state.join_merge_hits.saturating_add(1);
        };
        Ok(__absorb_out_28)
    }
}

pub struct MergeConditionalUnitSpec;
#[jungle::action]
impl Action for MergeConditionalUnitSpec {
    type Effect = TemplateCommitEffect;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &ConditionalJoinMergeState, _input: Self::Input) -> i32 {
        0
    }

    fn absorb(
        state: &mut ConditionalJoinMergeState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_29 = {
            output.map_err(|_err| Failure::from("merge conditional unit should succeed"))?;
            state.terminal_merge_hits = state.terminal_merge_hits.saturating_add(1);
        };
        Ok(__absorb_out_29)
    }
}

#[derive(Flow)]
struct LeftJoinBranch(
    Join<Step<LeftJoinFirstSpec>, Step<LeftJoinSecondSpec>>,
    Step<MergeJoinedUnitSpec>,
);

#[derive(Flow)]
struct RightJoinBranch(
    Join<Step<RightJoinFirstSpec>, Step<RightJoinSecondSpec>>,
    Step<MergeJoinedUnitSpec>,
);

#[derive(Flow)]
struct ConditionalJoinMergeFlow(
    Conditional<PreferLeftWhenMarkerNonNegative, LeftJoinBranch, RightJoinBranch>,
    Step<MergeConditionalUnitSpec>,
);

struct ConditionalJoinMergeAnimal;
#[jungle::animal(observe, id = 56, generation = 0)]
impl Animal for ConditionalJoinMergeAnimal {
    type State = ConditionalJoinMergeState;
    type Seed = ConditionalJoinMergeState;
    type Flow = ConditionalJoinMergeFlow;
}

impl Observe for ConditionalJoinMergeAnimal {
    type Appearance = ConditionalJoinMergeState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct ConditionalJoinMergeAnimals(ConditionalJoinMergeAnimal);

struct ConditionalJoinMergeZoo;
impl Ecosystem for ConditionalJoinMergeZoo {
    const NAME: &'static str = "conditional-join-merge-local-client-zoo";
    type Animals = ConditionalJoinMergeAnimals;
}

impl From<ConditionalJoinMergeState> for () {
    fn from(_value: ConditionalJoinMergeState) -> Self {}
}

#[tokio::test]
async fn conditional_then_join_branches_then_merge_flattens_unit_output_end_to_end() {
    let client = jungle_sdk::FusedClient::builder()
        .namespace("conditional-join-merge-local-client-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(ConditionalJoinMergeZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<ConditionalJoinMergeAnimal>(&ConditionalJoinMergeState {
            marker: 1,
            ..ConditionalJoinMergeState::default()
        })
        .await
        .expect("journey should start")
        .journey_id;

    await_completion(&client, journey_id).await;

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("appearance request should succeed")
        .expect("appearance should exist");
    let appearance: ConditionalJoinMergeState =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");

    assert_eq!(appearance.left_join_hits + appearance.right_join_hits, 2);
    assert_eq!(appearance.join_merge_hits, 1);
    assert_eq!(appearance.terminal_merge_hits, 1);

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
#[jungle::effect(id = 49)]
impl<J> Effect<J> for ComplexTimedEffect {
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    #[allow(clippy::manual_async_fn)]
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
impl Predicate<(&ComplexAlphaState, &i32)> for KeepLoopingShared {
    fn eval((state, _): &(&ComplexAlphaState, &i32)) -> bool {
        state.loops < 2
    }
}

impl Predicate<(&ComplexBetaState, &i32)> for KeepLoopingShared {
    fn eval((state, _): &(&ComplexBetaState, &i32)) -> bool {
        state.iterations < 2
    }
}

pub struct ChooseUniqueAlpha;
impl Predicate<(ComplexAlphaState, i32)> for ChooseUniqueAlpha {
    fn eval((_state, _): &(ComplexAlphaState, i32)) -> bool {
        true
    }
}

impl Predicate<(ComplexBetaState, i32)> for ChooseUniqueAlpha {
    fn eval((_state, _): &(ComplexBetaState, i32)) -> bool {
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
pub struct UniqueToCarrySpec;

pub struct JoinLeftAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for JoinLeftAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "JoinLeftAct";
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::fast_ms(), input + 1)
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        _state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_30 =
            output.map_err(|_err| Failure::from("join-left effect should succeed"))?;
        Ok(__absorb_out_30)
    }
}

pub struct JoinRightAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for JoinRightAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "JoinRightAct";
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::slow_ms(), input + 2)
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        _state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_31 =
            output.map_err(|_err| Failure::from("join-right effect should succeed"))?;
        Ok(__absorb_out_31)
    }
}

pub struct JoinToCarryAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for JoinToCarryAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "JoinToCarryAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = (i32, i32);
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input.0 + input.1
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_32 = {
            let value = output.map_err(|_err| Failure::from("join-to-carry should succeed"))?;
            A::set_join_sum(state, value);
            value
        };
        Ok(__absorb_out_32)
    }
}

pub struct SelectFastAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for SelectFastAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "SelectFastAct";
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::fast_ms(), input + 3)
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        _state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_33 =
            output.map_err(|_err| Failure::from("select-fast effect should succeed"))?;
        Ok(__absorb_out_33)
    }
}

pub struct SelectSlowAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for SelectSlowAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "SelectSlowAct";
    type Effect = ComplexTimedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> (u64, i32) {
        (A::slow_ms(), input + 4)
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        _state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_34 =
            output.map_err(|_err| Failure::from("select-slow effect should succeed"))?;
        Ok(__absorb_out_34)
    }
}

pub struct SelectToCarryAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for SelectToCarryAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "SelectToCarryAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = Either<i32, i32>;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_35 = {
            let value = output.map_err(|_err| Failure::from("select-to-carry should succeed"))?;
            A::set_select_winner(state, value);
            value
        };
        Ok(__absorb_out_35)
    }
}

pub struct LoopAdvanceAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for LoopAdvanceAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "LoopAdvanceAct";
    type Effect = TemplateAddEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_36 = {
            let value = output.map_err(|_err| Failure::from("loop-advance should succeed"))?;
            A::set_shared_work(state, value);
            A::inc_loop(state);
            value
        };
        Ok(__absorb_out_36)
    }
}

pub struct UniqueAlphaAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for UniqueAlphaAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "UniqueAlphaAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input + 100
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_37 = {
            let value = output.map_err(|_err| Failure::from("unique-alpha should succeed"))?;
            A::set_unique_alpha(state, value);
            value
        };
        Ok(__absorb_out_37)
    }
}

pub struct UniqueBetaAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for UniqueBetaAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "UniqueBetaAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input - 100
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_38 = {
            let value = output.map_err(|_err| Failure::from("unique-beta should succeed"))?;
            A::set_unique_beta(state, value);
            value
        };
        Ok(__absorb_out_38)
    }
}

pub struct FinalizeAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for FinalizeAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "FinalizeAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_39 = {
            let value = output.map_err(|_err| Failure::from("finalize should succeed"))?;
            A::set_final(state, value);
            value
        };
        Ok(__absorb_out_39)
    }
}

pub struct UniqueToCarryAct<A>(core::marker::PhantomData<fn() -> A>);
impl<A> BoundAction<A> for UniqueToCarryAct<A>
where
    A: Animal + ComplexFlowBinding,
{
    const NAME: &'static str = "UniqueToCarryAct";
    type Effect = TemplateCommitEffect;
    type Aspect = Identity;
    type Input = Either<i32, i32>;
    type Output = i32;
    type Carry = ();

    fn emit(_state: &A::State, input: Self::Input) -> i32 {
        match input {
            Either::Left(value) | Either::Right(value) => value,
        }
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<A>>::Aspect as StateCarrier<A::State>>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<A>>::emit(view, input), ())
    }

    fn absorb(
        _state: &mut A::State,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_40 =
            output.map_err(|_err| Failure::from("unique-to-carry should succeed"))?;
        Ok(__absorb_out_40)
    }
}

#[jungle::action(bind = JoinLeftAct<A>)]
impl Action for JoinLeftSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = JoinRightAct<A>)]
impl Action for JoinRightSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = JoinToCarryAct<A>)]
impl Action for JoinToCarrySpec {
    type Effect = TemplateCommitEffect;
    type Input = (i32, i32);
    type Output = i32;
}

#[jungle::action(bind = SelectFastAct<A>)]
impl Action for SelectFastSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = SelectSlowAct<A>)]
impl Action for SelectSlowSpec {
    type Effect = ComplexTimedEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = SelectToCarryAct<A>)]
impl Action for SelectToCarrySpec {
    type Effect = TemplateCommitEffect;
    type Input = Either<i32, i32>;
    type Output = i32;
}

#[jungle::action(bind = LoopAdvanceAct<A>)]
impl Action for LoopAdvanceSpec {
    type Effect = TemplateAddEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = UniqueAlphaAct<A>)]
impl Action for UniqueAlphaSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = UniqueBetaAct<A>)]
impl Action for UniqueBetaSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = FinalizeAct<A>)]
impl Action for FinalizeSpec {
    type Effect = TemplateCommitEffect;
    type Input = i32;
    type Output = i32;
}

#[jungle::action(bind = UniqueToCarryAct<A>)]
impl Action for UniqueToCarrySpec {
    type Effect = TemplateCommitEffect;
    type Input = Either<i32, i32>;
    type Output = i32;
}

#[derive(Flow)]
struct SharedJoinBranch(
    Join<Step<JoinLeftSpec>, Step<JoinRightSpec>>,
    Step<JoinToCarrySpec>,
);

#[derive(Flow)]
struct SharedSelectBranch(
    Select<Step<SelectFastSpec>, Step<SelectSlowSpec>>,
    Step<SelectToCarrySpec>,
);

#[derive(Flow)]
struct SharedLoopBody(Step<LoopAdvanceSpec>);

#[derive(Flow)]
struct SharedComposedSegment(
    While<KeepLoopingShared, SharedLoopBody>,
    SharedJoinBranch,
    SharedSelectBranch,
);

#[derive(Flow)]
struct LongSharedSegment(Transparent<SharedMeta, SharedComposedSegment>);

#[derive(Flow)]
struct UniqueSegment(Conditional<ChooseUniqueAlpha, Step<UniqueAlphaSpec>, Step<UniqueBetaSpec>>);

#[derive(Flow)]
struct LongMixedFlow(
    LongSharedSegment,
    UniqueSegment,
    Step<UniqueToCarrySpec>,
    Step<FinalizeSpec>,
);

struct ComplexAlphaAnimal;
#[jungle::animal(observe, id = 50, generation = 0)]
impl Animal for ComplexAlphaAnimal {
    type State = ComplexAlphaState;
    type Seed = i32;
    type Flow = LongMixedFlow;
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
#[jungle::animal(observe, id = 51, generation = 0)]
impl Animal for ComplexBetaAnimal {
    type State = ComplexBetaState;
    type Seed = i32;
    type Flow = LongMixedFlow;
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
    let client = jungle_sdk::FusedClient::builder()
        .namespace("late-bound-complex-mixed-zoo")
        .build()
        .await
        .expect("local client should build");

    let worker = jungle_sdk::core::JungleWorker::new(ComplexMixedZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let alpha_id = client
        .spawn::<ComplexAlphaAnimal>(&5_i32)
        .await
        .expect("alpha journey should start")
        .journey_id;
    let beta_id = client
        .spawn::<ComplexBetaAnimal>(&5_i32)
        .await
        .expect("beta journey should start")
        .journey_id;

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
