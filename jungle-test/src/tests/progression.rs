use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    Action, ActionCompletion, ActionRequest, ActionStep, AspectStep, CreatureActionSet, Executor,
    Id, Identity, ManualExecutor, Running, Waiting,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1};
use jungle_sdk::{Creatures, Instinct};
use std::future::ready;

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
impl AspectStep<ProgressCreature, SeedAction> for Seed {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(_state: &i32, input: Self::In) -> i32 {
        input + 1
    }

    fn apply(state: &mut i32, output: ActionCompletion<SeedAction>) -> Self::Out {
        let value = output.expect("seed action should succeed");
        *state = value;
        value
    }
}

struct Finish;
impl AspectStep<ProgressCreature, FinishAction> for Finish {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state + input
    }

    fn apply(state: &mut i32, output: ActionCompletion<FinishAction>) -> Self::Out {
        let value = output.expect("finish action should succeed");
        *state = value;
        value
    }
}

#[derive(Instinct)]
struct ProgressInstinct(
    ActionStep<ProgressCreature, SeedAction, Seed>,
    ActionStep<ProgressCreature, FinishAction, Finish>,
);

animal!(ProgressCreature, U0, i32, ProgressInstinct);

#[derive(Creatures)]
struct ProgressCreatures(ProgressCreature);

type SeedStep = ActionStep<ProgressCreature, SeedAction, Seed>;
type FinishStep = ActionStep<ProgressCreature, FinishAction, Finish>;

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

impl<A, Step> StepExecutor for ActionStep<ProgressCreature, A, Step>
where
    A: Action<Dependency = (), In = i32, Out = i32, Err = ()>,
    Step: AspectStep<ProgressCreature, A, Aspect = Identity, In = i32, Out = i32>,
{
    type Action = A;
}

#[test]
fn workflow_action_set_is_extracted_from_instinct_composite() {
    type Expected = list![SeedAction, FinishAction];
    type Extracted = CreatureActionSet<ProgressCreatures>;
    assert_type_eq!(Extracted, Expected);
}

#[test]
fn executor_progresses_simple_instinct_steps() {
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
    let mut executor = ManualExecutor::<ProgressCreature>::new(0);

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
    let mut executor = ManualExecutor::<ProgressCreature>::new(0);
    let emitted: Vec<i32> = executor
        .advance_to_end_typed(vec![(5, Ok::<i32, ()>(8)), (4, Ok::<i32, ()>(36))])
        .expect("flow should advance");

    assert_eq!(emitted, vec![8, 36]);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

#[test]
fn executor_threads_previous_emitted_output_into_next_input() {
    let mut executor = Executor::<ProgressCreature>::new(0);

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
