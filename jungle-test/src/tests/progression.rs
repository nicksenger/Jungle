use inception::Inception;
use jungle_types::{
    Action, ActionCompletion, ActionRequest, ActionStep, AnimalActionSet, AspectStep, Waiting,
    ErasedStep, Id, Ident, JungleAnimals, JungleFlow, TestExecutor, TestFlow,
    TypedErasedStep, Whole, Running,
};
use serde_json::json;
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

struct SeedMapper;
impl AspectStep<ProgressAnimal, SeedAction> for SeedMapper {
    type Aspect = Whole;
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

struct FinishMapper;
impl AspectStep<ProgressAnimal, FinishAction> for FinishMapper {
    type Aspect = Whole;
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

#[derive(Inception)]
#[inception(properties = [JungleFlow])]
struct ProgressInstinct(
    ActionStep<ProgressAnimal, SeedAction, SeedMapper>,
    ActionStep<ProgressAnimal, FinishAction, FinishMapper>,
);

animal!(ProgressAnimal, U0, i32, ProgressInstinct);

#[derive(Inception)]
#[inception(properties = [Ident, JungleAnimals])]
struct ProgressAnimals(ProgressAnimal);

type SeedStep = ActionStep<ProgressAnimal, SeedAction, SeedMapper>;
type FinishStep = ActionStep<ProgressAnimal, FinishAction, FinishMapper>;

struct Executor;
impl Executor {
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

impl<A, Step> StepExecutor for ActionStep<ProgressAnimal, A, Step>
where
    A: Action<Dependency = (), In = i32, Out = i32, Err = ()>,
    Step: AspectStep<ProgressAnimal, A, Aspect = Whole, In = i32, Out = i32>,
{
    type Action = A;
}

impl TestFlow for ProgressInstinct {
    type State = i32;

    fn build_steps() -> Vec<Box<dyn ErasedStep<Self::State>>> {
        vec![
            Box::new(TypedErasedStep::<SeedStep>::new()),
            Box::new(TypedErasedStep::<FinishStep>::new()),
        ]
    }
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
fn test_executor_next_advances_without_step_type_parameters() {
    let mut executor = TestExecutor::<ProgressAnimal>::new(0);

    let emitted_seed = executor.next(json!(5), Ok(json!(8))).expect("seed step");
    assert_eq!(emitted_seed, json!(8));

    let emitted_finish = executor.next(json!(4), Ok(json!(36))).expect("finish step");
    assert_eq!(emitted_finish, json!(36));
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}

#[test]
fn test_executor_advance_to_end_runs_remaining_flow() {
    let mut executor = TestExecutor::<ProgressAnimal>::new(0);
    let emitted = executor
        .advance_to_end(vec![(json!(5), Ok(json!(8))), (json!(4), Ok(json!(36)))])
        .expect("flow should advance");

    assert_eq!(emitted, vec![json!(8), json!(36)]);
    assert!(executor.is_complete());
    assert_eq!(executor.into_state(), 36);
}
