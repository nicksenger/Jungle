use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Act, Condition, Conditional, Ecosystem, Effect, EffectCompletion, Identity, JourneyStatus,
    LoopCondition, Observe, Perturb, StateLens, Step, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::{Animals, JungleClient, Optic, RunnerUpdateOut};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "postgres")]
use testcontainers::runners::AsyncRunner;
#[cfg(feature = "postgres")]
use testcontainers_modules::postgres::Postgres;

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
struct SubFlowState {
    nested: DeepFocusState,
    value: i32,
    updates: i32,
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
struct DeepFocusState {
    value: i32,
    updates: i32,
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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

struct AddOneEffect;
impl jungle_sdk::types::EffectMember for AddOneEffect {}

impl Effect for AddOneEffect {
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

struct AddTwoEffect;
impl jungle_sdk::types::EffectMember for AddTwoEffect {}

impl Effect for AddTwoEffect {
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
impl Act<IntegrationAnimal> for AddOneBeforeFullStateStep {
    type Effect = AddOneEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &IntegrationState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut IntegrationState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.total += output.expect("first pre-focused full-state effect should succeed");
        state.before_steps += 1;
    }
}

struct AddTwoBeforeFullStateStep;
impl Act<IntegrationAnimal> for AddTwoBeforeFullStateStep {
    type Effect = AddTwoEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &IntegrationState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut IntegrationState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.total += output.expect("second pre-focused full-state effect should succeed");
        state.before_steps += 1;
    }
}

struct AddOneFocusedStep;
impl Act<IntegrationAnimal> for AddOneFocusedStep {
    type Effect = AddOneEffect;
    type StateAspect = StateLens<IntegrationState, list![jungle_sdk::typosaurus::num::consts::U1]>;
    type Input = ();
    type Output = ();

    fn emit(_state: &SubFlowState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut SubFlowState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.value += output.expect("first focused integration effect should succeed");
        state.updates += 1;
    }
}

struct AddTwoFocusedStep;
impl Act<IntegrationAnimal> for AddTwoFocusedStep {
    type Effect = AddTwoEffect;
    type StateAspect = StateLens<IntegrationState, list![jungle_sdk::typosaurus::num::consts::U1]>;
    type Input = ();
    type Output = ();

    fn emit(_state: &SubFlowState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut SubFlowState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.value += output.expect("second focused integration effect should succeed");
        state.updates += 1;
    }
}

struct AddOneDeepFocusedStep;
impl Act<IntegrationAnimal> for AddOneDeepFocusedStep {
    type Effect = AddOneEffect;
    type StateAspect = StateLens<
        IntegrationState,
        list![
            jungle_sdk::typosaurus::num::consts::U1,
            jungle_sdk::typosaurus::num::consts::U0
        ],
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DeepFocusState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut DeepFocusState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.value += output.expect("first deep-focused integration effect should succeed");
        state.updates += 1;
    }
}

struct AddTwoDeepFocusedStep;
impl Act<IntegrationAnimal> for AddTwoDeepFocusedStep {
    type Effect = AddTwoEffect;
    type StateAspect = StateLens<
        IntegrationState,
        list![
            jungle_sdk::typosaurus::num::consts::U1,
            jungle_sdk::typosaurus::num::consts::U0
        ],
    >;
    type Input = ();
    type Output = ();

    fn emit(_state: &DeepFocusState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut DeepFocusState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.value += output.expect("second deep-focused integration effect should succeed");
        state.updates += 1;
    }
}

struct AddOneAfterFullStateStep;
impl Act<IntegrationAnimal> for AddOneAfterFullStateStep {
    type Effect = AddOneEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &IntegrationState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut IntegrationState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.total += output.expect("first post-focused full-state effect should succeed");
        state.after_steps += 1;
    }
}

struct AddTwoAfterFullStateStep;
impl Act<IntegrationAnimal> for AddTwoAfterFullStateStep {
    type Effect = AddTwoEffect;
    type StateAspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &IntegrationState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut IntegrationState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.total += output.expect("second post-focused full-state effect should succeed");
        state.after_steps += 1;
    }
}

struct KeepRunning;
impl LoopCondition<IntegrationState> for KeepRunning {
    type Arg = ();

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

type MultiMatchBeforeFlow = Conditional<
    UseFirstBeforeFullStateTask,
    Step<IntegrationAnimal, AddOneBeforeFullStateStep>,
    Conditional<
        UseFirstBeforeFullStateTask,
        Step<IntegrationAnimal, AddOneBeforeFullStateStep>,
        Step<IntegrationAnimal, AddOneBeforeFullStateStep>,
    >,
>;

type LoopBranchFlow = While<
    KeepRunning,
    Conditional<
        UseFirstBeforeFullStateTask,
        Step<IntegrationAnimal, AddOneBeforeFullStateStep>,
        Step<IntegrationAnimal, AddTwoBeforeFullStateStep>,
    >,
>;

type IntegrationJourney = While<
    KeepRunning,
    Conditional<
        IsBeforeFocusedSubFlow,
        Conditional<
            UseFirstBeforeFullStateTask,
            Step<IntegrationAnimal, AddOneBeforeFullStateStep>,
            Step<IntegrationAnimal, AddTwoBeforeFullStateStep>,
        >,
        Conditional<
            IsInFocusedSubFlow,
            Conditional<
                UseFirstFocusedTask,
                Step<IntegrationAnimal, AddOneFocusedStep>,
                Step<IntegrationAnimal, AddTwoFocusedStep>,
            >,
            Conditional<
                IsInDeepFocusedSubFlow,
                Conditional<
                    UseFirstDeepFocusedTask,
                    Step<IntegrationAnimal, AddOneDeepFocusedStep>,
                    Step<IntegrationAnimal, AddTwoDeepFocusedStep>,
                >,
                Conditional<
                    UseFirstAfterFullStateTask,
                    Step<IntegrationAnimal, AddOneAfterFullStateStep>,
                    Step<IntegrationAnimal, AddTwoAfterFullStateStep>,
                >,
            >,
        >,
    >,
>;

animal!(
    IntegrationAnimal,
    jungle_sdk::typosaurus::num::consts::U0,
    IntegrationState,
    IntegrationJourney,
    observe = true,
    perturb = true
);

impl Observe for IntegrationAnimal {
    type Appearance = IntegrationState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct IntegrationPerturbation {
    delta: i32,
}

impl Perturb for IntegrationAnimal {
    type Stimulus = IntegrationPerturbation;

    fn perturb(state: &mut Self::State, stimulus: Self::Stimulus) {
        state.total += stimulus.delta;
    }
}

#[derive(Animals)]
struct IntegrationAnimals(IntegrationAnimal);

struct IntegrationZoo;
impl Ecosystem for IntegrationZoo {
    const NAME: &'static str = "integration-zoo";
    type Animals = IntegrationAnimals;
}

impl From<IntegrationState> for () {
    fn from(_value: IntegrationState) -> Self {}
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

    run_client_worker_flow_runs_to_completion(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn memory_client_worker_flow_runs_to_completion() {
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .memory()
            .run()
            .await
    });

    run_client_worker_flow_runs_to_completion(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_client_worker_flow_runs_to_completion() {
    let postgres = Postgres::default()
        .start()
        .await
        .expect("postgres testcontainer should start");
    let pg_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres mapped port should be available");
    let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let connection_string = connection_string.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .postgres_connection_string(connection_string)
                .run()
                .await
        }
    });

    run_client_worker_flow_runs_to_completion(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn redb_client_worker_streams_step_updates_end_to_end() {
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

    run_client_worker_streams_step_updates_end_to_end(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn memory_client_worker_streams_step_updates_end_to_end() {
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn(async move {
        ServerBuilder::new()
            .listen(listen_addr)
            .memory()
            .run()
            .await
    });

    run_client_worker_streams_step_updates_end_to_end(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_client_worker_streams_step_updates_end_to_end() {
    let postgres = Postgres::default()
        .start()
        .await
        .expect("postgres testcontainer should start");
    let pg_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres mapped port should be available");
    let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let connection_string = connection_string.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .postgres_connection_string(connection_string)
                .run()
                .await
        }
    });

    run_client_worker_streams_step_updates_end_to_end(listen_addr).await;

    server_task.abort();
    let _ = server_task.await;
}

async fn run_client_worker_flow_runs_to_completion(listen_addr: SocketAddr) {
    let client = connect_client_with_retry(listen_addr).await;
    let worker = JungleWorker::new(IntegrationZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });
    let saw_appearance = Arc::new(AtomicBool::new(false));

    let seed = integration_seed();
    let journey_id = client
        .start_journey::<IntegrationAnimal>(seed)
        .await
        .expect("start_journey should succeed");
    let perturb_payload = postcard::to_allocvec(&IntegrationPerturbation { delta: 1000 })
        .expect("perturb payload should serialize");
    client
        .perturb_animal(journey_id, perturb_payload)
        .await
        .expect("perturb_animal should enqueue perturbation");

    let completion = tokio::time::timeout(Duration::from_secs(8), {
        let client_ref = &client;
        let saw_appearance = Arc::clone(&saw_appearance);
        async move {
            loop {
                let status = client_ref
                    .journey_details(journey_id)
                    .await
                    .expect("journey_details should succeed while waiting for completion");
                if let Some(appearance_bytes) = client_ref
                    .animal_appearance(journey_id)
                    .await
                    .expect("animal_appearance should succeed while waiting for completion")
                {
                    let appearance: IntegrationState = postcard::from_bytes(&appearance_bytes)
                        .expect("animal appearance should deserialize");
                    if appearance.before_steps > 0 || appearance.after_steps > 0 {
                        saw_appearance.store(true, Ordering::Relaxed);
                    }
                }
                if status == JourneyStatus::Completed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    })
    .await;
    if completion.is_err() {
        let status = client
            .journey_details(journey_id)
            .await
            .expect("final journey_details should still be queryable");
        panic!("flow did not complete before timeout, last status: {status:?}");
    }
    if worker_handle.is_finished() {
        let joined = worker_handle.await;
        panic!("worker should keep polling, got: {joined:?}");
    }

    assert!(
        saw_appearance.load(Ordering::Relaxed),
        "expected to observe at least one non-initial appearance snapshot"
    );

    let final_appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("final animal_appearance should succeed")
        .expect("final animal_appearance should be present");
    let final_appearance: IntegrationState = postcard::from_bytes(&final_appearance_bytes)
        .expect("final animal appearance should deserialize");
    assert!(
        final_appearance.after_steps > 0,
        "final appearance should reflect progressed state"
    );
    assert!(
        final_appearance.total >= 1000,
        "final appearance should include applied perturbation delta"
    );

    worker_handle.abort();
    let _ = worker_handle.await;
}

async fn run_client_worker_streams_step_updates_end_to_end(listen_addr: SocketAddr) {
    let client = connect_client_with_retry(listen_addr).await;
    let worker = JungleWorker::new(IntegrationZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = integration_seed();
    let journey_id = client
        .start_journey::<IntegrationAnimal>(seed)
        .await
        .expect("start_journey should succeed");

    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");

    let mut started_count = 0_u32;
    let mut succeeded_count = 0_u32;
    let mut failed_count = 0_u32;
    let mut total_step_updates = 0_u32;
    let mut last_sequence_id: Option<u64> = None;

    let completion = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let Some(next) = subscription.next().await else {
                break;
            };
            let update = next.expect("streamed journey update should succeed");

            let (sequence_id, update_journey_id) = match update.event {
                RunnerUpdateOut::EffectInput { uuid, .. } => {
                    started_count += 1;
                    (update.sequence_id, uuid)
                }
                RunnerUpdateOut::EffectSuccessOutput { uuid, .. } => {
                    succeeded_count += 1;
                    (update.sequence_id, uuid)
                }
                RunnerUpdateOut::EffectFailureOutput { uuid, .. } => {
                    failed_count += 1;
                    (update.sequence_id, uuid)
                }
                RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {
                    continue;
                }
            };
            total_step_updates += 1;

            assert_eq!(
                update_journey_id, journey_id,
                "stream update should match journey"
            );
            if let Some(prev) = last_sequence_id {
                assert!(
                    sequence_id > prev,
                    "stream sequence ids must be strictly increasing"
                );
            }
            last_sequence_id = Some(sequence_id);
        }
    })
    .await;
    if completion.is_err() {
        let status = client
            .journey_details(journey_id)
            .await
            .expect("final journey_details should still be queryable");
        panic!("stream did not finish before timeout, last status: {status:?}");
    }
    if worker_handle.is_finished() {
        let joined = worker_handle.await;
        panic!("worker should keep polling, got: {joined:?}");
    }

    let final_status = client
        .journey_details(journey_id)
        .await
        .expect("journey_details should succeed after stream completion");
    assert_eq!(final_status, JourneyStatus::Completed);
    assert!(
        started_count > 0,
        "expected at least one started step update"
    );
    assert!(
        succeeded_count > 0,
        "expected at least one succeeded step update"
    );
    assert_eq!(failed_count, 0, "expected no failed step updates");
    const INTEGRATION_SHORTEST_PATH_STEPS: u32 = 8;
    assert!(
        total_step_updates >= INTEGRATION_SHORTEST_PATH_STEPS,
        "expected streamed step updates ({total_step_updates}) to be >= shortest path steps ({INTEGRATION_SHORTEST_PATH_STEPS})"
    );

    worker_handle.abort();
    let _ = worker_handle.await;
}

fn integration_seed() -> Vec<u8> {
    postcard::to_allocvec(&IntegrationState {
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
    .expect("seed should serialize")
}

#[test]
fn replaced_alias_rewrites_integration_flow_steps() {
    type Actual = jungle_sdk::types::Replace<
        MultiMatchBeforeFlow,
        jungle_sdk::types::SwapLR<AddOneBeforeFullStateStep, AddTwoBeforeFullStateStep>,
    >;
    type Expected = Conditional<
        UseFirstBeforeFullStateTask,
        Step<IntegrationAnimal, AddTwoBeforeFullStateStep>,
        Conditional<
            UseFirstBeforeFullStateTask,
            Step<IntegrationAnimal, AddTwoBeforeFullStateStep>,
            Step<IntegrationAnimal, AddTwoBeforeFullStateStep>,
        >,
    >;
    assert_type_eq!(Actual, Expected);
}

#[test]
fn replaced_nodes_alias_replaces_loop_branch_section() {
    type Actual = jungle_sdk::types::ReplaceNodes<
        LoopBranchFlow,
        jungle_sdk::types::SwapNodeLR<
            LoopBranchFlow,
            Step<IntegrationAnimal, AddOneAfterFullStateStep>,
        >,
    >;
    type Expected = Step<IntegrationAnimal, AddOneAfterFullStateStep>;
    assert_type_eq!(Actual, Expected);
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
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
