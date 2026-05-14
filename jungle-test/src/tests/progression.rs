use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    Action, ActionCompletion, ActionRequest, AnimalActionSet, Condition, Conditional,
    ContextExecutor, Executor, Id, Identity, ManualExecutor, Pulse, Running, Step, Waiting,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2};
use jungle_sdk::{Animals, Journey};
use std::future::ready;
use std::sync::Arc;

struct SeedAction;
impl jungle_types::ActionMember for SeedAction {}
impl Action for SeedAction {
    type Id = Id<U0>;
    type Dependency = ();
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 2))
    }
}

struct FinishAction;
impl jungle_types::ActionMember for FinishAction {}
impl Action for FinishAction {
    type Id = Id<U1>;
    type Dependency = ();
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input * 3))
    }
}

struct Seed;
impl Pulse<ProgressAnimal> for Seed {
    type Action = SeedAction;
    type Aspect = Identity;
    type CarryIn = i32;
    type CarryOut = i32;

    fn emit(_state: &i32, input: Self::CarryIn) -> i32 {
        input + 1
    }

    fn absorb(state: &mut i32, output: ActionCompletion<SeedAction>) -> Self::CarryOut {
        let value = output.expect("seed action should succeed");
        *state = value;
        value
    }
}

struct Finish;
impl Pulse<ProgressAnimal> for Finish {
    type Action = FinishAction;
    type Aspect = Identity;
    type CarryIn = i32;
    type CarryOut = i32;

    fn emit(state: &i32, input: Self::CarryIn) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: ActionCompletion<FinishAction>) -> Self::CarryOut {
        let value = output.expect("finish action should succeed");
        *state = value;
        value
    }
}

#[derive(Journey)]
struct ProgressJourney(Step<ProgressAnimal, Seed>, Step<ProgressAnimal, Finish>);

animal!(ProgressAnimal, U0, i32, ProgressJourney);

#[derive(Animals)]
struct ProgressAnimals(ProgressAnimal);

struct ProgressContext;
impl From<&ProgressContext> for () {
    fn from(_value: &ProgressContext) -> Self {}
}

type SeedStep = Step<ProgressAnimal, Seed>;
type FinishStep = Step<ProgressAnimal, Finish>;

struct StepHarness;
impl StepHarness {
    fn progress<Step>(
        state: i32,
        input: i32,
        completion: ActionCompletion<Step::Action>,
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
    Running<In = (i32, i32), Out = (i32, ActionRequest<Self::Action>)>
    + Waiting<In = (i32, ActionCompletion<Self::Action>), Out = (i32, i32)>
{
    type Action: Action<Dependency = (), In = i32, Out = i32, Err = ()>;
}

impl<A> StepExecutor for Step<ProgressAnimal, A>
where
    A: Pulse<ProgressAnimal, Aspect = Identity, CarryIn = i32, CarryOut = i32>,
    <A as Pulse<ProgressAnimal>>::Action: Action<Dependency = (), In = i32, Out = i32, Err = ()>,
{
    type Action = <A as Pulse<ProgressAnimal>>::Action;
}

#[test]
fn workflow_action_set_is_extracted_from_journey_composite() {
    type Expected = list![SeedAction, FinishAction];
    type Extracted = AnimalActionSet<ProgressAnimals>;
    assert_type_eq!(Extracted, Expected);
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

struct BranchAction;
impl jungle_types::ActionMember for BranchAction {}
impl Action for BranchAction {
    type Id = Id<U2>;
    type Dependency = ();
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}

struct BranchStepA;
impl Pulse<BranchAnimal> for BranchStepA {
    type Action = BranchAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &i32, _input: Self::CarryIn) -> i32 {
        *state
    }

    fn absorb(state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        *state = output.expect("branch step A should succeed");
    }
}

struct BranchStepB;
impl Pulse<BranchAnimal> for BranchStepB {
    type Action = BranchAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &i32, _input: Self::CarryIn) -> i32 {
        *state
    }

    fn absorb(state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
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
struct DerivedBranchFlow(Step<BranchAnimal, BranchStepA>, Step<BranchAnimal, BranchStepB>);

type BranchConditionalFlow =
    Conditional<UseDerivedBranch, DerivedBranchFlow, Step<BranchAnimal, BranchStepB>>;

#[derive(Journey)]
struct BranchJourney(BranchConditionalFlow);

animal!(BranchAnimal, U1, i32, BranchJourney);

struct BranchContext;
impl From<&BranchContext> for () {
    fn from(_value: &BranchContext) -> Self {}
}

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
