use jungle_sdk::act;
use jungle_sdk::animal;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::Animal;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    Act, Condition, Conditional, Ecosystem, EffectCompletion, EffectExec, EffectSchema,
    JourneyStatus, LoopCondition, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::consts::*;
use jungle_sdk::{Animals, JungleClient, RunnerOut};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

const PRE_STEPS: usize = 2;
const POST_STEPS: usize = 2;
const TEST_OWNER_LEASE_TTL_MS: i64 = 1_500;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

struct ReplayPreIncrementEffect;
impl EffectSchema for ReplayPreIncrementEffect {
    type Id = Id<U41>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl EffectExec<()> for ReplayPreIncrementEffect {
    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

impl EffectExec<ReplayGateZoo> for ReplayPreIncrementEffect {
    fn effect(
        jungle: &ReplayGateZoo,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.pre_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayPostIncrementEffect;
impl EffectSchema for ReplayPostIncrementEffect {
    type Id = Id<U42>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl EffectExec<()> for ReplayPostIncrementEffect {
    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

impl EffectExec<ReplayGateZoo> for ReplayPostIncrementEffect {
    fn effect(
        jungle: &ReplayGateZoo,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.post_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayGateEffect;
impl EffectSchema for ReplayGateEffect {
    type Id = Id<U43>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl EffectExec<()> for ReplayGateEffect {
    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

impl EffectExec<ReplayGateZoo> for ReplayGateEffect {
    fn effect(
        jungle: &ReplayGateZoo,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let reached_tx = jungle.reached_tx.clone();
        let gate = Arc::clone(&jungle.gate);
        async move {
            reached_tx.send(()).map_err(|_| ())?;
            let permit = gate.acquire().await.map_err(|_| ())?;
            permit.forget();
            Ok(())
        }
    }
}

trait ReplayPhaseState {
    fn phase(&self) -> u8;
}

impl ReplayPhaseState for ReplayGateState {
    fn phase(&self) -> u8 {
        self.phase
    }
}

struct ReplayPhaseNotComplete;
impl<S> LoopCondition<S> for ReplayPhaseNotComplete
where
    S: ReplayPhaseState,
{
    type Arg = ();

    fn should_continue(state: &S) -> bool {
        state.phase() < 5
    }
}

struct ReplayPhaseIs<const N: u8>;
impl<S, Arg, const N: u8> Condition<(S, Arg)> for ReplayPhaseIs<N>
where
    S: ReplayPhaseState,
{
    fn choose((state, _): &(S, Arg)) -> bool {
        state.phase() == N
    }
}

type ReplayPhaseRouterFlow<Pre, Mid, Post> = While<
    ReplayPhaseNotComplete,
    Conditional<
        ReplayPhaseIs<0>,
        Step<Pre>,
        Conditional<
            ReplayPhaseIs<1>,
            Step<Pre>,
            Conditional<
                ReplayPhaseIs<2>,
                Step<Mid>,
                Conditional<ReplayPhaseIs<3>, Step<Post>, Step<Post>>,
            >,
        >,
    >,
>;

struct ReplayPreSpec;
#[act]
impl Act for ReplayPreSpec {
    type Effect = ReplayPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut ReplayGateState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("pre increment should succeed");
        state.phase += 1;
    }
}

struct ReplayGateSpec;
#[act]
impl Act for ReplayGateSpec {
    type Effect = ReplayGateEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut ReplayGateState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("gate effect should succeed");
        state.phase += 1;
    }
}

struct ReplayPostSpec;
#[act]
impl Act for ReplayPostSpec {
    type Effect = ReplayPostIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut ReplayGateState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("post increment should succeed");
        state.phase += 1;
    }
}

#[derive(jungle_sdk::Flow)]
struct ReplayGateTemplate(ReplayPhaseRouterFlow<ReplayPreSpec, ReplayGateSpec, ReplayPostSpec>);

type ReplayGateJourney = ReplayGateTemplate;

struct ReplayGateAnimal;

#[animal(id = 0, generation = 0)]
impl Animal for ReplayGateAnimal {
    type State = ReplayGateState;
    type Seed = ReplayGateState;
    type Journey = ReplayGateJourney;
}

#[derive(Animals)]
struct ReplayGateAnimals(ReplayGateAnimal);

impl Ecosystem for ReplayGateZoo {
    const NAME: &'static str = "replay-gate-zoo";
    type Animals = ReplayGateAnimals;
}

impl From<ReplayGateState> for () {
    fn from(_value: ReplayGateState) -> Self {}
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

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
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

    let seed = postcard::to_allocvec(&ReplayGateState { phase: 0 }).expect("seed should serialize");
    let journey_id = control_client
        .start_journey::<ReplayGateAnimal>(seed)
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

    let worker_two = tokio::spawn({
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
    server_task.abort();
    let _ = server_task.await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ReplayTimeoutState {
    phase: u8,
    sleep_for_ms: u64,
}

impl Default for ReplayTimeoutState {
    fn default() -> Self {
        Self {
            phase: 0,
            sleep_for_ms: 4_000,
        }
    }
}

impl ReplayPhaseState for ReplayTimeoutState {
    fn phase(&self) -> u8 {
        self.phase
    }
}

#[derive(Clone)]
struct ReplayTimeoutZoo {
    global_pre_counter: Arc<AtomicUsize>,
    global_post_counter: Arc<AtomicUsize>,
    worker_pre_counter: Arc<AtomicUsize>,
}

struct ReplayTimeoutPreIncrementEffect;
impl EffectSchema for ReplayTimeoutPreIncrementEffect {
    type Id = Id<U44>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl EffectExec<()> for ReplayTimeoutPreIncrementEffect {
    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

impl EffectExec<ReplayTimeoutZoo> for ReplayTimeoutPreIncrementEffect {
    fn effect(
        jungle: &ReplayTimeoutZoo,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.global_pre_counter.fetch_add(1, Ordering::SeqCst);
        jungle.worker_pre_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayTimeoutPostIncrementEffect;
impl EffectSchema for ReplayTimeoutPostIncrementEffect {
    type Id = Id<U45>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl EffectExec<()> for ReplayTimeoutPostIncrementEffect {
    fn effect(
        _jungle: &(),
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

impl EffectExec<ReplayTimeoutZoo> for ReplayTimeoutPostIncrementEffect {
    fn effect(
        jungle: &ReplayTimeoutZoo,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.global_post_counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

struct ReplayTimeoutPreSpec;
#[act]
impl Act for ReplayTimeoutPreSpec {
    type Effect = ReplayTimeoutPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("pre-timeout increment should succeed");
        state.phase += 1;
    }
}

struct ReplayTimeoutSleepSpec;
#[act]
impl Act for ReplayTimeoutSleepSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &ReplayTimeoutState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("timeout sleep should succeed");
        state.phase += 1;
    }
}

struct ReplayTimeoutPostSpec;
#[act]
impl Act for ReplayTimeoutPostSpec {
    type Effect = ReplayTimeoutPostIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("post-timeout increment should succeed");
        state.phase += 1;
    }
}

#[derive(jungle_sdk::Flow)]
struct ReplayTimeoutTemplate(
    ReplayPhaseRouterFlow<ReplayTimeoutPreSpec, ReplayTimeoutSleepSpec, ReplayTimeoutPostSpec>,
);

type ReplayTimeoutJourney = ReplayTimeoutTemplate;

struct ReplayTimeoutAnimal;

#[animal(id = 0, generation = 0)]
impl Animal for ReplayTimeoutAnimal {
    type State = ReplayTimeoutState;
    type Seed = ReplayTimeoutState;
    type Journey = ReplayTimeoutJourney;
}

#[derive(Animals)]
struct ReplayTimeoutAnimals(ReplayTimeoutAnimal);

impl Ecosystem for ReplayTimeoutZoo {
    const NAME: &'static str = "replay-timeout-zoo";
    type Animals = ReplayTimeoutAnimals;
}

impl From<ReplayTimeoutState> for () {
    fn from(_value: ReplayTimeoutState) -> Self {}
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

    let global_pre_counter = Arc::new(AtomicUsize::new(0));
    let global_post_counter = Arc::new(AtomicUsize::new(0));
    let worker_one_pre_counter = Arc::new(AtomicUsize::new(0));
    let worker_two_pre_counter = Arc::new(AtomicUsize::new(0));

    let mut worker_one = Some(tokio::spawn({
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
    let mut worker_two = Some(tokio::spawn({
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
    let journey_id = control_client
        .start_journey::<ReplayTimeoutAnimal>(seed)
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
