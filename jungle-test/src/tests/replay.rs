use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Pulse, Action, ActionCompletion, Condition, Conditional, Ecosystem, Identity, JourneyStatus,
    LoopCondition, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::Unsigned;
use jungle_sdk::{Animals, JungleClient, RunnerOut};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

const PRE_STEPS: usize = 2;
const POST_STEPS: usize = 2;
const TEST_OWNER_LEASE_TTL_MS: i64 = 1_500;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ReplayGateState {
    phase: u8,
}

#[derive(Clone)]
struct ReplayGateZoo {
    pre_counter: Arc<AtomicUsize>,
    post_counter: Arc<AtomicUsize>,
    reached_tx: mpsc::UnboundedSender<()>,
    gate: Arc<Semaphore>,
}

#[derive(Clone)]
struct ReplayGatePreDependency {
    pre_counter: Arc<AtomicUsize>,
}

impl From<&ReplayGateZoo> for ReplayGatePreDependency {
    fn from(value: &ReplayGateZoo) -> Self {
        Self {
            pre_counter: Arc::clone(&value.pre_counter),
        }
    }
}

#[derive(Clone)]
struct ReplayGatePostDependency {
    post_counter: Arc<AtomicUsize>,
}

impl From<&ReplayGateZoo> for ReplayGatePostDependency {
    fn from(value: &ReplayGateZoo) -> Self {
        Self {
            post_counter: Arc::clone(&value.post_counter),
        }
    }
}

#[derive(Clone)]
struct ReplayGateDependency {
    reached_tx: mpsc::UnboundedSender<()>,
    gate: Arc<Semaphore>,
}

impl From<&ReplayGateZoo> for ReplayGateDependency {
    fn from(value: &ReplayGateZoo) -> Self {
        Self {
            reached_tx: value.reached_tx.clone(),
            gate: Arc::clone(&value.gate),
        }
    }
}

struct ReplayPreIncrementAction;
impl jungle_sdk::types::ActionMember for ReplayPreIncrementAction {}
impl Action for ReplayPreIncrementAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U41>;
    type Dependency = ReplayGatePreDependency;
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        dependency.pre_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayPostIncrementAction;
impl jungle_sdk::types::ActionMember for ReplayPostIncrementAction {}
impl Action for ReplayPostIncrementAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U42>;
    type Dependency = ReplayGatePostDependency;
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        dependency.post_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayGateAction;
impl jungle_sdk::types::ActionMember for ReplayGateAction {}
impl Action for ReplayGateAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U43>;
    type Dependency = ReplayGateDependency;
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let reached_tx = dependency.reached_tx.clone();
        let gate = Arc::clone(&dependency.gate);
        async move {
            reached_tx.send(()).map_err(|_| ())?;
            let permit = gate.acquire().await.map_err(|_| ())?;
            permit.forget();
            Ok(())
        }
    }
}

struct ReplayPreStep;
impl Pulse<ReplayGateAnimal> for ReplayPreStep {
    type Action = ReplayPreIncrementAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &ReplayGateState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut ReplayGateState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("pre increment should succeed");
        state.phase += 1;
    }
}

struct ReplayPostStep;
impl Pulse<ReplayGateAnimal> for ReplayPostStep {
    type Action = ReplayPostIncrementAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &ReplayGateState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut ReplayGateState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("post increment should succeed");
        state.phase += 1;
    }
}

struct ReplayGateStep;
impl Pulse<ReplayGateAnimal> for ReplayGateStep {
    type Action = ReplayGateAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &ReplayGateState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut ReplayGateState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("gate action should succeed");
        state.phase += 1;
    }
}

struct ReplayGateNotComplete;
impl LoopCondition<ReplayGateState> for ReplayGateNotComplete {
    type CarryIn = ();

    fn should_continue(state: &ReplayGateState) -> bool {
        state.phase < 5
    }
}

struct ReplayGatePhaseZero;
impl Condition<(ReplayGateState, ())> for ReplayGatePhaseZero {
    fn choose((state, _): &(ReplayGateState, ())) -> bool {
        state.phase == 0
    }
}

struct ReplayGatePhaseOne;
impl Condition<(ReplayGateState, ())> for ReplayGatePhaseOne {
    fn choose((state, _): &(ReplayGateState, ())) -> bool {
        state.phase == 1
    }
}

struct ReplayGatePhaseTwo;
impl Condition<(ReplayGateState, ())> for ReplayGatePhaseTwo {
    fn choose((state, _): &(ReplayGateState, ())) -> bool {
        state.phase == 2
    }
}

struct ReplayGatePhaseThree;
impl Condition<(ReplayGateState, ())> for ReplayGatePhaseThree {
    fn choose((state, _): &(ReplayGateState, ())) -> bool {
        state.phase == 3
    }
}

type ReplayGateJourney = While<
    ReplayGateNotComplete,
    Conditional<
        ReplayGatePhaseZero,
        Step<ReplayGateAnimal, ReplayPreStep>,
        Conditional<
            ReplayGatePhaseOne,
            Step<ReplayGateAnimal, ReplayPreStep>,
            Conditional<
                ReplayGatePhaseTwo,
                Step<ReplayGateAnimal, ReplayGateStep>,
                Conditional<
                    ReplayGatePhaseThree,
                    Step<ReplayGateAnimal, ReplayPostStep>,
                    Step<ReplayGateAnimal, ReplayPostStep>,
                >,
            >,
        >,
    >,
>;

animal!(
    ReplayGateAnimal,
    jungle_sdk::typosaurus::num::consts::U0,
    ReplayGateState,
    ReplayGateJourney
);

#[derive(Animals)]
struct ReplayGateAnimals(ReplayGateAnimal);

impl Ecosystem for ReplayGateZoo {
    type Animals = ReplayGateAnimals;
}

#[tokio::test]
async fn replay_after_worker_crash_does_not_repeat_pre_gate_side_effects() {
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

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = connect_client_with_retry(listen_addr).await;
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    tokio::task::LocalSet::new()
        .run_until(async {
            let pre_counter = Arc::new(AtomicUsize::new(0));
            let post_counter = Arc::new(AtomicUsize::new(0));
            let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
            let gate = Arc::new(Semaphore::new(0));

            let worker_one = tokio::task::spawn_local({
                let client = worker_one_client.clone();
                let zoo = ReplayGateZoo {
                    pre_counter: Arc::clone(&pre_counter),
                    post_counter: Arc::clone(&post_counter),
                    reached_tx: reached_tx.clone(),
                    gate: Arc::clone(&gate),
                };
                async move {
                    let worker = JungleWorker::new(zoo, client);
                    let _ = worker.spawn().await;
                }
            });

            let seed =
                postcard::to_allocvec(&ReplayGateState { phase: 0 }).expect("seed should serialize");
            let ordinal = <jungle_sdk::typosaurus::num::consts::U0 as Unsigned>::U32;
            let journey_id = control_client
                .start_journey(ordinal, seed)
                .await
                .expect("start_journey should succeed");

            tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
                .await
                .expect("first gate notification should arrive")
                .expect("first gate notification channel should remain open");

            assert_eq!(
                pre_counter.load(Ordering::SeqCst),
                PRE_STEPS,
                "pre-gate side effects should run exactly once before crash"
            );
            assert_eq!(post_counter.load(Ordering::SeqCst), 0);

            worker_one.abort();
            let _ = worker_one.await;

            let worker_two = tokio::task::spawn_local({
                let client = worker_two_client.clone();
                let zoo = ReplayGateZoo {
                    pre_counter: Arc::clone(&pre_counter),
                    post_counter: Arc::clone(&post_counter),
                    reached_tx,
                    gate: Arc::clone(&gate),
                };
                async move {
                    let worker = JungleWorker::new(zoo, client);
                    let _ = worker.spawn().await;
                }
            });

            tokio::time::timeout(Duration::from_secs(45), async {
                loop {
                    if reached_rx.try_recv().is_ok() {
                        break;
                    }
                    let _ = control_client
                        .journey_details(journey_id)
                        .await
                        .expect("journey_details should succeed while waiting for replay gate");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            })
            .await
            .expect("replay gate notification should arrive after reclaim");

            assert_eq!(
                pre_counter.load(Ordering::SeqCst),
                PRE_STEPS,
                "replay should not rerun pre-gate side effects"
            );
            assert_eq!(post_counter.load(Ordering::SeqCst), 0);

            gate.add_permits(1);

            wait_for_completed(listen_addr, journey_id, Duration::from_secs(10)).await;

            assert_eq!(pre_counter.load(Ordering::SeqCst), PRE_STEPS);
            assert_eq!(post_counter.load(Ordering::SeqCst), POST_STEPS);

            worker_two.abort();
            let _ = worker_two.await;
        })
        .await;
    server_task.abort();
    let _ = server_task.await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ReplayTimeoutState {
    phase: u8,
    sleep_for_ms: u64,
}

#[derive(Clone)]
struct ReplayTimeoutZoo {
    global_pre_counter: Arc<AtomicUsize>,
    global_post_counter: Arc<AtomicUsize>,
    worker_pre_counter: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct ReplayTimeoutPreDependency {
    global_pre_counter: Arc<AtomicUsize>,
    worker_pre_counter: Arc<AtomicUsize>,
}

impl From<&ReplayTimeoutZoo> for ReplayTimeoutPreDependency {
    fn from(value: &ReplayTimeoutZoo) -> Self {
        Self {
            global_pre_counter: Arc::clone(&value.global_pre_counter),
            worker_pre_counter: Arc::clone(&value.worker_pre_counter),
        }
    }
}

#[derive(Clone)]
struct ReplayTimeoutPostDependency {
    global_post_counter: Arc<AtomicUsize>,
}

impl From<&ReplayTimeoutZoo> for ReplayTimeoutPostDependency {
    fn from(value: &ReplayTimeoutZoo) -> Self {
        Self {
            global_post_counter: Arc::clone(&value.global_post_counter),
        }
    }
}

struct ReplayTimeoutPreIncrementAction;
impl jungle_sdk::types::ActionMember for ReplayTimeoutPreIncrementAction {}
impl Action for ReplayTimeoutPreIncrementAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U44>;
    type Dependency = ReplayTimeoutPreDependency;
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        dependency.global_pre_counter.fetch_add(1, Ordering::SeqCst);
        dependency.worker_pre_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayTimeoutPostIncrementAction;
impl jungle_sdk::types::ActionMember for ReplayTimeoutPostIncrementAction {}
impl Action for ReplayTimeoutPostIncrementAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U45>;
    type Dependency = ReplayTimeoutPostDependency;
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        dependency.global_post_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayTimeoutPreStep;
impl Pulse<ReplayTimeoutAnimal> for ReplayTimeoutPreStep {
    type Action = ReplayTimeoutPreIncrementAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut ReplayTimeoutState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("pre-timeout increment should succeed");
        state.phase += 1;
    }
}

struct ReplayTimeoutSleepStep;
impl Pulse<ReplayTimeoutAnimal> for ReplayTimeoutSleepStep {
    type Action = Sleep;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(state: &ReplayTimeoutState, _input: Self::In) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(state: &mut ReplayTimeoutState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("timeout sleep should succeed");
        state.phase += 1;
    }
}

struct ReplayTimeoutPostStep;
impl Pulse<ReplayTimeoutAnimal> for ReplayTimeoutPostStep {
    type Action = ReplayTimeoutPostIncrementAction;
    type Aspect = Identity;
    type In = ();
    type Out = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::In) -> Self::In {}

    fn absorb(state: &mut ReplayTimeoutState, output: ActionCompletion<Self::Action>) -> Self::Out {
        output.expect("post-timeout increment should succeed");
        state.phase += 1;
    }
}

struct ReplayTimeoutNotComplete;
impl LoopCondition<ReplayTimeoutState> for ReplayTimeoutNotComplete {
    type CarryIn = ();

    fn should_continue(state: &ReplayTimeoutState) -> bool {
        state.phase < 5
    }
}

struct ReplayTimeoutPhaseZero;
impl Condition<(ReplayTimeoutState, ())> for ReplayTimeoutPhaseZero {
    fn choose((state, _): &(ReplayTimeoutState, ())) -> bool {
        state.phase == 0
    }
}

struct ReplayTimeoutPhaseOne;
impl Condition<(ReplayTimeoutState, ())> for ReplayTimeoutPhaseOne {
    fn choose((state, _): &(ReplayTimeoutState, ())) -> bool {
        state.phase == 1
    }
}

struct ReplayTimeoutPhaseTwo;
impl Condition<(ReplayTimeoutState, ())> for ReplayTimeoutPhaseTwo {
    fn choose((state, _): &(ReplayTimeoutState, ())) -> bool {
        state.phase == 2
    }
}

struct ReplayTimeoutPhaseThree;
impl Condition<(ReplayTimeoutState, ())> for ReplayTimeoutPhaseThree {
    fn choose((state, _): &(ReplayTimeoutState, ())) -> bool {
        state.phase == 3
    }
}

type ReplayTimeoutJourney = While<
    ReplayTimeoutNotComplete,
    Conditional<
        ReplayTimeoutPhaseZero,
        Step<ReplayTimeoutAnimal, ReplayTimeoutPreStep>,
        Conditional<
            ReplayTimeoutPhaseOne,
            Step<ReplayTimeoutAnimal, ReplayTimeoutPreStep>,
            Conditional<
                ReplayTimeoutPhaseTwo,
                Step<ReplayTimeoutAnimal, ReplayTimeoutSleepStep>,
                Conditional<
                    ReplayTimeoutPhaseThree,
                    Step<ReplayTimeoutAnimal, ReplayTimeoutPostStep>,
                    Step<ReplayTimeoutAnimal, ReplayTimeoutPostStep>,
                >,
            >,
        >,
    >,
>;

animal!(
    ReplayTimeoutAnimal,
    jungle_sdk::typosaurus::num::consts::U0,
    ReplayTimeoutState,
    ReplayTimeoutJourney
);

#[derive(Animals)]
struct ReplayTimeoutAnimals(ReplayTimeoutAnimal);

impl Ecosystem for ReplayTimeoutZoo {
    type Animals = ReplayTimeoutAnimals;
}

#[tokio::test]
async fn replay_after_owner_dies_during_timeout_uses_other_worker_without_repeating_pre_steps() {
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

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = connect_client_with_retry(listen_addr).await;
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    tokio::task::LocalSet::new()
        .run_until(async {
            let global_pre_counter = Arc::new(AtomicUsize::new(0));
            let global_post_counter = Arc::new(AtomicUsize::new(0));
            let worker_one_pre_counter = Arc::new(AtomicUsize::new(0));
            let worker_two_pre_counter = Arc::new(AtomicUsize::new(0));

            let mut worker_one = Some(tokio::task::spawn_local({
                let client = worker_one_client.clone();
                let zoo = ReplayTimeoutZoo {
                    global_pre_counter: Arc::clone(&global_pre_counter),
                    global_post_counter: Arc::clone(&global_post_counter),
                    worker_pre_counter: Arc::clone(&worker_one_pre_counter),
                };
                async move {
                    let worker =
                        JungleWorker::new(zoo, client).with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS);
                    let _ = worker.spawn().await;
                }
            }));
            let mut worker_two = Some(tokio::task::spawn_local({
                let client = worker_two_client.clone();
                let zoo = ReplayTimeoutZoo {
                    global_pre_counter: Arc::clone(&global_pre_counter),
                    global_post_counter: Arc::clone(&global_post_counter),
                    worker_pre_counter: Arc::clone(&worker_two_pre_counter),
                };
                async move {
                    let worker =
                        JungleWorker::new(zoo, client).with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS);
                    let _ = worker.spawn().await;
                }
            }));

            let seed = postcard::to_allocvec(&ReplayTimeoutState {
                phase: 0,
                sleep_for_ms: 4_000,
            })
            .expect("timeout test seed should serialize");
            let ordinal = <jungle_sdk::typosaurus::num::consts::U0 as Unsigned>::U32;
            let journey_id = control_client
                .start_journey(ordinal, seed)
                .await
                .expect("start_journey should succeed");

            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if global_pre_counter.load(Ordering::SeqCst) == PRE_STEPS {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await
            .expect("pre-timeout increments should finish");

            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let history = control_client
                        .journey_history(journey_id)
                        .await
                        .expect("journey_history should succeed");
                    if history
                        .iter()
                        .any(|event| matches!(event, RunnerOut::SleepScheduled { .. }))
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await
            .expect("sleep should be scheduled before owner kill");

            let owner_is_worker_one = worker_one_pre_counter.load(Ordering::SeqCst) > 0;
            if owner_is_worker_one {
                if let Some(handle) = worker_one.take() {
                    handle.abort();
                    let _ = handle.await;
                }
            } else if let Some(handle) = worker_two.take() {
                handle.abort();
                let _ = handle.await;
            }

            wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

            assert_eq!(
                global_pre_counter.load(Ordering::SeqCst),
                PRE_STEPS,
                "replay after timeout failover should not rerun pre-timeout side effects"
            );
            assert_eq!(
                global_post_counter.load(Ordering::SeqCst),
                POST_STEPS,
                "post-timeout side effects should run once after resume"
            );

            let worker_one_pre = worker_one_pre_counter.load(Ordering::SeqCst);
            let worker_two_pre = worker_two_pre_counter.load(Ordering::SeqCst);
            assert_eq!(
                worker_one_pre + worker_two_pre,
                PRE_STEPS,
                "only one worker should have executed original pre-timeout steps"
            );

            if let Some(handle) = worker_one {
                handle.abort();
                let _ = handle.await;
            }
            if let Some(handle) = worker_two {
                handle.abort();
                let _ = handle.await;
            }
        })
        .await;
    server_task.abort();
    let _ = server_task.await;
}

async fn wait_for_completed(remote: SocketAddr, journey_id: uuid::Uuid, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    let mut client = connect_client_with_retry(remote).await;
    loop {
        match client.journey_details(journey_id).await {
            Ok(status) => {
                if status == JourneyStatus::Completed {
                    return;
                }
            }
            Err(_) => {
                client = connect_client_with_retry(remote).await;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "journey should complete before timeout"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
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
