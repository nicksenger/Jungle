use inception::Inception;
use jungle_types::{
    Action, ActionCompletion, ActionInputMapper, ActionOutputMapper, ActionRequest, ActionStep,
    AnimalActionSet, Awaiting, Id, Ident, JungleAnimals, JungleWorkflowActions, Yielding,
};
use std::marker::PhantomData;
use typosaurus::assert_type_eq;
use typosaurus::list;
use typosaurus::num::consts::{U0, U1};

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
        std::future::ready(Ok(input + 2))
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
        std::future::ready(Ok(input * 3))
    }
}

struct PrepareSeed;
impl ActionInputMapper<ProgressAnimal, SeedAction> for PrepareSeed {
    type In = i32;

    fn map_input(_state: &i32, input: Self::In) -> i32 {
        input + 1
    }
}

struct ApplySeed;
impl ActionOutputMapper<ProgressAnimal, SeedAction> for ApplySeed {
    type Out = i32;

    fn map_output(state: &mut i32, output: ActionCompletion<SeedAction>) -> Self::Out {
        let value = output.expect("seed action should succeed");
        *state = value;
        value
    }
}

struct PrepareFinish;
impl ActionInputMapper<ProgressAnimal, FinishAction> for PrepareFinish {
    type In = i32;

    fn map_input(state: &i32, input: Self::In) -> i32 {
        *state + input
    }
}

struct ApplyFinish;
impl ActionOutputMapper<ProgressAnimal, FinishAction> for ApplyFinish {
    type Out = i32;

    fn map_output(state: &mut i32, output: ActionCompletion<FinishAction>) -> Self::Out {
        let value = output.expect("finish action should succeed");
        *state = value;
        value
    }
}

#[derive(Inception)]
#[inception(properties = [JungleWorkflowActions])]
struct ProgressInstinct(
    ActionStep<ProgressAnimal, SeedAction, PrepareSeed, ApplySeed>,
    ActionStep<ProgressAnimal, FinishAction, PrepareFinish, ApplyFinish>,
);

animal!(ProgressAnimal, U0, i32, ProgressInstinct);

#[derive(Inception)]
#[inception(properties = [Ident, JungleAnimals])]
struct ProgressAnimals(ProgressAnimal);

type SeedStep = ActionStep<ProgressAnimal, SeedAction, PrepareSeed, ApplySeed>;
type FinishStep = ActionStep<ProgressAnimal, FinishAction, PrepareFinish, ApplyFinish>;

struct Executor;
impl Executor {
    fn progress<Step>(state: i32, input: i32, completion: ActionCompletion<Step::Action>) -> (i32, i32)
    where
        Step: StepExecutor,
    {
        let (state, request) = <Step as Yielding>::run((state, input));
        let _prepared = request.into_input();
        <Step as Awaiting>::accept((state, completion))
    }
}

#[derive(Clone, Copy)]
enum ProgressStepKey {
    Seed,
    Finish,
}

trait ErasedStepExecutor {
    fn progress(&self, state: i32, input: i32, completion: Result<i32, ()>) -> (i32, i32);
}

struct TypedErasedStep<Step>(PhantomData<fn() -> Step>);
impl<Step> TypedErasedStep<Step> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Step> ErasedStepExecutor for TypedErasedStep<Step>
where
    Step: StepExecutor,
{
    fn progress(&self, state: i32, input: i32, completion: Result<i32, ()>) -> (i32, i32) {
        Executor::progress::<Step>(state, input, completion)
    }
}

struct ErasedExecutor {
    seed: Box<dyn ErasedStepExecutor>,
    finish: Box<dyn ErasedStepExecutor>,
}

impl ErasedExecutor {
    fn new() -> Self {
        Self {
            seed: Box::new(TypedErasedStep::<SeedStep>::new()),
            finish: Box::new(TypedErasedStep::<FinishStep>::new()),
        }
    }

    fn progress(
        &self,
        step: ProgressStepKey,
        state: i32,
        input: i32,
        completion: Result<i32, ()>,
    ) -> (i32, i32) {
        match step {
            ProgressStepKey::Seed => self.seed.progress(state, input, completion),
            ProgressStepKey::Finish => self.finish.progress(state, input, completion),
        }
    }
}

trait StepExecutor: Yielding<In = (i32, i32), Out = (i32, ActionRequest<Self::Action>)>
    + Awaiting<In = (i32, ActionCompletion<Self::Action>), Out = (i32, i32)>
{
    type Action: Action<Dependency = (), In = i32, Out = i32, Err = ()>;
}

impl<A, Prep, Apply> StepExecutor for ActionStep<ProgressAnimal, A, Prep, Apply>
where
    A: Action<Dependency = (), In = i32, Out = i32, Err = ()>,
    Prep: ActionInputMapper<ProgressAnimal, A, In = i32>,
    Apply: ActionOutputMapper<ProgressAnimal, A, Out = i32>,
{
    type Action = A;
}

#[test]
fn workflow_action_set_is_extracted_from_instinct_composite() {
    type Expected = list![SeedAction, FinishAction];
    type Extracted = AnimalActionSet<ProgressAnimals>;
    assert_type_eq!(Extracted, Expected);
}

#[test]
fn executor_progresses_simple_instinct_steps() {
    let (state_after_seed, emitted_seed) = Executor::progress::<SeedStep>(0, 5, Ok(8));
    assert_eq!(state_after_seed, 8);
    assert_eq!(emitted_seed, 8);

    let (state_after_finish, emitted_finish) =
        Executor::progress::<FinishStep>(state_after_seed, 4, Ok(36));
    assert_eq!(state_after_finish, 36);
    assert_eq!(emitted_finish, 36);
}

#[test]
fn erased_executor_progresses_without_step_type_parameters() {
    let executor = ErasedExecutor::new();

    let (state_after_seed, emitted_seed) =
        executor.progress(ProgressStepKey::Seed, 0, 5, Ok(8));
    assert_eq!(state_after_seed, 8);
    assert_eq!(emitted_seed, 8);

    let (state_after_finish, emitted_finish) =
        executor.progress(ProgressStepKey::Finish, state_after_seed, 4, Ok(36));
    assert_eq!(state_after_finish, 36);
    assert_eq!(emitted_finish, 36);
}
