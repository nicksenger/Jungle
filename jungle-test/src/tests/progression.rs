use jungle_sdk::animal;
use jungle_sdk::types as jungle_types;
use jungle_sdk::types::Animal;
use jungle_sdk::types::{
    BoundAct, BoundFlowStep, Condition, Conditional, ContextExecutor, EffectCompletion, EffectExec,
    EffectRequest, EffectSchema, Executor, Id, Identity, ManualExecutor, Running, Waiting,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2};
use jungle_sdk::Journey;
use std::future::ready;
use std::sync::Arc;

struct SeedEffect;
impl EffectSchema for SeedEffect {
    type Id = Id<U0>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for SeedEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 2))
    }
}

struct FinishEffect;
impl EffectSchema for FinishEffect {
    type Id = Id<U1>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for FinishEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input * 3))
    }
}

struct Seed;
impl BoundAct<ProgressAnimal> for Seed {
    type Effect = SeedEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &i32, input: Self::Input) -> i32 {
        input + 1
    }

    fn absorb(state: &mut i32, output: EffectCompletion<SeedEffect>) -> Self::Output {
        let value = output.expect("seed effect should succeed");
        *state = value;
        value
    }
}

struct Finish;
impl BoundAct<ProgressAnimal> for Finish {
    type Effect = FinishEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<FinishEffect>) -> Self::Output {
        let value = output.expect("finish effect should succeed");
        *state = value;
        value
    }
}

#[derive(Journey)]
struct ProgressJourney(
    BoundFlowStep<ProgressAnimal, Seed>,
    BoundFlowStep<ProgressAnimal, Finish>,
);

struct ProgressAnimal;

#[animal]
impl Animal for ProgressAnimal {
    type Id = Id<U0>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = ProgressJourney;
}

struct ProgressContext;

type SeedStep = BoundFlowStep<ProgressAnimal, Seed>;
type FinishStep = BoundFlowStep<ProgressAnimal, Finish>;

struct StepHarness;
impl StepHarness {
    fn progress<Step>(
        state: i32,
        input: i32,
        completion: EffectCompletion<Step::Effect>,
    ) -> (i32, i32)
    where
        Step: StepExecutor,
    {
        let (state, request) = <Step as Running>::run((state, input));
        let _prepared = request.into_input();
        <Step as Waiting>::accept((state, completion))
    }
}

trait StepExecutor:
    Running<In = (i32, i32), Out = (i32, EffectRequest<Self::Effect>)>
    + Waiting<In = (i32, EffectCompletion<Self::Effect>), Out = (i32, i32)>
{
    type Effect: EffectSchema<In = i32, Out = i32, Err = ()>;
}

impl<A> StepExecutor for BoundFlowStep<ProgressAnimal, A>
where
    A: BoundAct<ProgressAnimal, Aspect = Identity, Input = i32, Output = i32>,
    <A as BoundAct<ProgressAnimal>>::Effect:
        EffectSchema<In = i32, Out = i32, Err = ()> + EffectExec<()>,
{
    type Effect = <A as BoundAct<ProgressAnimal>>::Effect;
}

#[test]
fn executor_progresses_simple_journey_steps() {
    let (state_after_seed, emitted_seed) = StepHarness::progress::<SeedStep>(0, 5, Ok(8));
    assert_eq!(state_after_seed, 8);
    assert_eq!(emitted_seed, 8);

    let (state_after_finish, emitted_finish) =
        StepHarness::progress::<FinishStep>(state_after_seed, 4, Ok(36));
    assert_eq!(state_after_finish, 36);
    assert_eq!(emitted_finish, 36);
}

#[test]
fn executor_next_advances_with_serialized_completions() {
    let mut executor = ManualExecutor::<ProgressAnimal>::new(0);

    let emitted_seed: i32 = executor.next_typed(5, Ok::<i32, ()>(8)).expect("seed step");
    assert_eq!(emitted_seed, 8);

    let emitted_finish: i32 = executor
        .next_typed(4, Ok::<i32, ()>(36))
        .expect("finish step");
    assert_eq!(emitted_finish, 36);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

#[test]
fn executor_advance_to_end_runs_remaining_flow() {
    let mut executor = ManualExecutor::<ProgressAnimal>::new(0);
    let emitted: Vec<i32> = executor
        .advance_to_end_typed(vec![(5, Ok::<i32, ()>(8)), (4, Ok::<i32, ()>(36))])
        .expect("flow should advance");

    assert_eq!(emitted, vec![8, 36]);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

#[test]
fn executor_threads_previous_emitted_output_into_next_input() {
    let mut executor = Executor::<ProgressAnimal>::new(0);

    let request_seed: i32 = executor.next_request().expect("seed request");
    assert_eq!(request_seed, 1);
    let emitted_seed: i32 = executor
        .complete(Ok::<i32, ()>(8))
        .expect("seed completion");
    assert_eq!(emitted_seed, 8);

    let request_finish: i32 = executor.next_request().expect("finish request");
    assert_eq!(request_finish, 16);
    let emitted_finish: i32 = executor
        .complete(Ok::<i32, ()>(36))
        .expect("finish completion");
    assert_eq!(emitted_finish, 36);

    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

#[test]
fn context_executor_progresses_multi_step_derived_journey() {
    let mut executor =
        ContextExecutor::<ProgressContext, ProgressAnimal>::new(Arc::new(ProgressContext), 0);

    let request_seed: i32 = executor.next_request().expect("seed request");
    assert_eq!(request_seed, 1);
    let emitted_seed: i32 = executor
        .complete(Ok::<i32, ()>(8))
        .expect("seed completion");
    assert_eq!(emitted_seed, 8);

    let request_finish: i32 = executor.next_request().expect("finish request");
    assert_eq!(request_finish, 16);
    let emitted_finish: i32 = executor
        .complete(Ok::<i32, ()>(36))
        .expect("finish completion");
    assert_eq!(emitted_finish, 36);

    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

struct BranchEffect;
impl EffectSchema for BranchEffect {
    type Id = Id<U2>;
    type In = i32;
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for BranchEffect {
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}

struct BranchStepA;
impl BoundAct<BranchAnimal> for BranchStepA {
    type Effect = BranchEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &i32, _input: Self::Input) -> i32 {
        *state
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        *state = output.expect("branch step A should succeed");
    }
}

struct BranchStepB;
impl BoundAct<BranchAnimal> for BranchStepB {
    type Effect = BranchEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &i32, _input: Self::Input) -> i32 {
        *state
    }

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        *state = output.expect("branch step B should succeed");
    }
}

struct UseDerivedBranch;
impl Condition<(i32, ())> for UseDerivedBranch {
    fn choose((state, _): &(i32, ())) -> bool {
        *state >= 0
    }
}

#[derive(Journey)]
struct DerivedBranchFlow(
    BoundFlowStep<BranchAnimal, BranchStepA>,
    BoundFlowStep<BranchAnimal, BranchStepB>,
);

type BranchConditionalFlow =
    Conditional<UseDerivedBranch, DerivedBranchFlow, BoundFlowStep<BranchAnimal, BranchStepB>>;

#[derive(Journey)]
struct BranchJourney(BranchConditionalFlow);

struct BranchAnimal;

#[animal]
impl Animal for BranchAnimal {
    type Id = Id<U1>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = BranchJourney;
}

struct BranchContext;

#[test]
fn context_executor_accepts_conditional_with_derived_multistep_branch() {
    fn assert_context_flow<F>()
    where
        F: jungle_types::BuildFlowWithContext<
            (Arc<BranchContext>, jungle_types::DynFlow<i32>),
            Output = (Arc<BranchContext>, jungle_types::DynFlow<i32>),
        >,
    {
    }

    assert_context_flow::<DerivedBranchFlow>();
    assert_context_flow::<BranchJourney>();
}
