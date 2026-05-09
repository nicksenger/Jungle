use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Action, ActionCompletion, Condition, Ecosystem, FlowStatus, Identity, Impulse, LoopCondition,
    Task, While,
};
use jungle_sdk::typosaurus::num::Unsigned;
use jungle_sdk::{Animae, JungleClient};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct IntegrationState(i32);

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

struct AddOneStep;
impl Task<IntegrationAnima> for AddOneStep {
    type Action = AddOneAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("first integration action should succeed");
    }
}

struct AddTwoStep;
impl Task<IntegrationAnima> for AddTwoStep {
    type Action = AddTwoAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn prepare(_state: &IntegrationState, _input: Self::In) -> Self::In {}

    fn process(state: &mut IntegrationState, output: ActionCompletion<Self::Action>) -> Self::Out {
        state.0 += output.expect("second integration action should succeed");
    }
}

struct KeepRunning;
impl LoopCondition<IntegrationState> for KeepRunning {
    fn should_continue(state: &IntegrationState) -> bool {
        state.0 < 3
    }
}

struct UseFirstStep;
impl Condition<(IntegrationState, ())> for UseFirstStep {
    fn choose((state, _): &(IntegrationState, ())) -> bool {
        state.0 % 2 == 0
    }
}

type IntegrationJourney = While<
    KeepRunning,
    jungle_sdk::types::Conditional<
        UseFirstStep,
        Impulse<IntegrationAnima, AddOneStep>,
        Impulse<IntegrationAnima, AddTwoStep>,
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

    let seed = postcard::to_allocvec(&IntegrationState(0)).expect("seed should serialize");
    let ordinal = <jungle_sdk::typosaurus::num::consts::U0 as Unsigned>::U32;
    let flow_id = client
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
                    .journey_details(flow_id)
                    .await
                    .expect("journey_details should succeed while waiting for completion");
                if status == FlowStatus::Completed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }) => {
            if completion.is_err() {
                let status = client
                    .journey_details(flow_id)
                    .await
                    .expect("final journey_details should still be queryable");
                panic!("flow did not complete before timeout, last status: {status:?}");
            }
        }
    }

    server_task.abort();
    let _ = server_task.await;
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
