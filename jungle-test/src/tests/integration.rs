use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Action, ActionCompletion, Condition, Conditional, Ecosystem, JourneyStatus, Lens, Identity, Impulse,
    LoopCondition, Reflex, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::Unsigned;
use jungle_sdk::{Animae, JungleClient, Optic};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SubFlowState {
    nested: DeepFocusState,
    value: i32,
    updates: i32,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct DeepFocusState {
    value: i32,
    updates: i32,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct IntegrationState {
    total: i32,
    focused: SubFlowState,
    before_steps: u8,
    after_steps: u8,
}

#[derive(Clone, Copy)]
struct AddOneDependency {
    value: i32,
}

impl From<&IntegrationZoo> for AddOneDependency {
    fn from(_value: &IntegrationZoo) -> Self {
        Self { value: 1 }
    }
}

struct AddOneAction;
impl jungle_sdk::types::ActionMember for AddOneAction {}

impl Action for AddOneAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U1>;
    type Dependency = AddOneDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.value))
    }
}

#[derive(Clone, Copy)]
struct AddTwoDependency {
    value: i32,
}

impl From<&IntegrationZoo> for AddTwoDependency {
    fn from(_value: &IntegrationZoo) -> Self {
        Self { value: 2 }
    }
}

struct AddTwoAction;
impl jungle_sdk::types::ActionMember for AddTwoAction {}

impl Action for AddTwoAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U2>;
    type Dependency = AddTwoDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(dependency.value))
    }
}

struct AddOneBeforeFullStateStep;
impl Reflex<IntegrationAnima> for AddOneBeforeFullStateStep {
    type Action = AddOneAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.total += output.expect("first pre-focused full-state action should succeed");
        state.before_steps += 1;
    }
}

struct AddTwoBeforeFullStateStep;
impl Reflex<IntegrationAnima> for AddTwoBeforeFullStateStep {
    type Action = AddTwoAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.total += output.expect("second pre-focused full-state action should succeed");
        state.before_steps += 1;
    }
}

struct AddOneFocusedStep;
impl Reflex<IntegrationAnima> for AddOneFocusedStep {
    type Action = AddOneAction;
    type Aspect = Lens<IntegrationState, list![jungle_sdk::typosaurus::num::consts::U1]>;
    type In = ();
    type Out = ();

    fn prepare(_state: &SubFlowState, _input: Self::In) -> Self::In {}

    fn process(state: &mut SubFlowState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.value += output.expect("first focused integration action should succeed");
        state.updates += 1;
    }
}

struct AddTwoFocusedStep;
impl Reflex<IntegrationAnima> for AddTwoFocusedStep {
    type Action = AddTwoAction;
    type Aspect = Lens<IntegrationState, list![jungle_sdk::typosaurus::num::consts::U1]>;
    type In = ();
    type Out = ();

    fn prepare(_state: &SubFlowState, _input: Self::In) -> Self::In {}

    fn process(state: &mut SubFlowState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.value += output.expect("second focused integration action should succeed");
        state.updates += 1;
    }
}

struct AddOneDeepFocusedStep;
impl Reflex<IntegrationAnima> for AddOneDeepFocusedStep {
    type Action = AddOneAction;
    type Aspect = Lens<
        IntegrationState,
        list![
            jungle_sdk::typosaurus::num::consts::U1,
            jungle_sdk::typosaurus::num::consts::U0
        ],
    >;
    type In = ();
    type Out = ();

    fn prepare(_state: &DeepFocusState, _input: Self::In) -> Self::In {}

    fn process(state: &mut DeepFocusState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.value += output.expect("first deep-focused integration action should succeed");
        state.updates += 1;
    }
}

struct AddTwoDeepFocusedStep;
impl Reflex<IntegrationAnima> for AddTwoDeepFocusedStep {
    type Action = AddTwoAction;
    type Aspect = Lens<
        IntegrationState,
        list![
            jungle_sdk::typosaurus::num::consts::U1,
            jungle_sdk::typosaurus::num::consts::U0
        ],
    >;
    type In = ();
    type Out = ();

    fn prepare(_state: &DeepFocusState, _input: Self::In) -> Self::In {}

    fn process(state: &mut DeepFocusState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.value += output.expect("second deep-focused integration action should succeed");
        state.updates += 1;
    }
}

struct AddOneAfterFullStateStep;
impl Reflex<IntegrationAnima> for AddOneAfterFullStateStep {
    type Action = AddOneAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.total += output.expect("first post-focused full-state action should succeed");
        state.after_steps += 1;
    }
}

struct AddTwoAfterFullStateStep;
impl Reflex<IntegrationAnima> for AddTwoAfterFullStateStep {
    type Action = AddTwoAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.total += output.expect("second post-focused full-state action should succeed");
        state.after_steps += 1;
    }
}

struct KeepRunning;
impl LoopCondition<IntegrationState> for KeepRunning {
    fn should_continue(state: &IntegrationState) -> bool {
        state.after_steps < 2
    }
}

struct IsBeforeFocusedSubFlow;
impl Condition<(IntegrationState, ())> for IsBeforeFocusedSubFlow {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.before_steps < 2
    }
}

struct UseFirstBeforeFullStateTask;
impl Condition<(IntegrationState, ())> for UseFirstBeforeFullStateTask {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.before_steps == 0
    }
}

struct IsInFocusedSubFlow;
impl Condition<(IntegrationState, ())> for IsInFocusedSubFlow {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.focused.updates < 2
    }
}

struct UseFirstFocusedTask;
impl Condition<(IntegrationState, ())> for UseFirstFocusedTask {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.focused.updates == 0
    }
}

struct IsInDeepFocusedSubFlow;
impl Condition<(IntegrationState, ())> for IsInDeepFocusedSubFlow {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.focused.nested.updates < 2
    }
}

struct UseFirstDeepFocusedTask;
impl Condition<(IntegrationState, ())> for UseFirstDeepFocusedTask {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.focused.nested.updates == 0
    }
}

struct UseFirstAfterFullStateTask;
impl Condition<(IntegrationState, ())> for UseFirstAfterFullStateTask {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.after_steps == 0
    }
}

type BeforeBranchFlow = Conditional<
    UseFirstBeforeFullStateTask,
    Impulse<IntegrationAnima, AddOneBeforeFullStateStep>,
    Impulse<IntegrationAnima, AddTwoBeforeFullStateStep>,
>;

struct SwapBeforeBranchSteps;
impl jungle_sdk::types::ReplaceStep<Impulse<IntegrationAnima, AddOneBeforeFullStateStep>>
    for SwapBeforeBranchSteps
{
    type Output = Impulse<IntegrationAnima, AddTwoBeforeFullStateStep>;
}
impl jungle_sdk::types::ReplaceStep<Impulse<IntegrationAnima, AddTwoBeforeFullStateStep>>
    for SwapBeforeBranchSteps
{
    type Output = Impulse<IntegrationAnima, AddOneBeforeFullStateStep>;
}

type IntegrationJourney = While<
    KeepRunning,
    Conditional<
        IsBeforeFocusedSubFlow,
        Conditional<
            UseFirstBeforeFullStateTask,
            Impulse<IntegrationAnima, AddOneBeforeFullStateStep>,
            Impulse<IntegrationAnima, AddTwoBeforeFullStateStep>,
        >,
        Conditional<
            IsInFocusedSubFlow,
            Conditional<
                UseFirstFocusedTask,
                Impulse<IntegrationAnima, AddOneFocusedStep>,
                Impulse<IntegrationAnima, AddTwoFocusedStep>,
            >,
            Conditional<
                IsInDeepFocusedSubFlow,
                Conditional<
                    UseFirstDeepFocusedTask,
                    Impulse<IntegrationAnima, AddOneDeepFocusedStep>,
                    Impulse<IntegrationAnima, AddTwoDeepFocusedStep>,
                >,
                Conditional<
                    UseFirstAfterFullStateTask,
                    Impulse<IntegrationAnima, AddOneAfterFullStateStep>,
                    Impulse<IntegrationAnima, AddTwoAfterFullStateStep>,
                >,
            >,
        >,
    >,
>;

anima!(
    IntegrationAnima,
    jungle_sdk::typosaurus::num::consts::U0,
    IntegrationState,
    IntegrationJourney
);

#[derive(Animae)]
struct IntegrationAnimae(IntegrationAnima);

struct IntegrationZoo;
impl Ecosystem for IntegrationZoo {
    type Animae = IntegrationAnimae;
}

#[tokio::test]
async fn redb_client_worker_flow_runs_to_completion() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let client = connect_client_with_retry(listen_addr).await;
    let worker = JungleWorker::new(IntegrationZoo, client.clone());
    let worker_future = worker.spawn();
    tokio::pin!(worker_future);

    let seed = postcard::to_allocvec(&IntegrationState {
        total: 0,
        focused: SubFlowState {
            nested: DeepFocusState {
                value: 0,
                updates: 0,
            },
            value: 0,
            updates: 0,
        },
        before_steps: 0,
        after_steps: 0,
    })
    .expect("seed should serialize");
    let ordinal = <jungle_sdk::typosaurus::num::consts::U0 as Unsigned>::U32;
    let journey_id = client
        .start_journey(ordinal, seed)
        .await
        .expect("start_journey should succeed");

    tokio::select! {
        result = &mut worker_future => {
            panic!("worker should keep polling, got: {result:?}");
        }
        completion = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let status = client
                    .journey_details(journey_id)
                    .await
                    .expect("journey_details should succeed while waiting for completion");
                if status == JourneyStatus::Completed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }) => {
            if completion.is_err() {
                let status = client
                    .journey_details(journey_id)
                    .await
                    .expect("final journey_details should still be queryable");
                panic!("flow did not complete before timeout, last status: {status:?}");
            }
        }
    }

    server_task.abort();
    let _ = server_task.await;
}

#[test]
fn replaced_alias_rewrites_integration_flow_steps() {
    type Actual = jungle_sdk::types::Replaced<BeforeBranchFlow, SwapBeforeBranchSteps>;
    type Expected = Conditional<
        UseFirstBeforeFullStateTask,
        Impulse<IntegrationAnima, AddTwoBeforeFullStateStep>,
        Impulse<IntegrationAnima, AddOneBeforeFullStateStep>,
    >;
    assert_type_eq!(Actual, Expected);
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::ClientBuilder::new()
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return client,
            Err(err) if attempt < 39 => {
                std::thread::sleep(Duration::from_millis(25));
                let _ = err;
            }
            Err(err) => panic!("failed to connect to test server: {err}"),
        }
    }

    unreachable!("retry loop always returns or panics")
}
