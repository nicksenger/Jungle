use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct ReplayRainforest(Arc<Mutex<ReplayInner>>);

struct ReplayInner {
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
}

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest";
    type Animals = ReplayRainforestAnimals;
}

impl ReplayRainforest {
    async fn next(&self) -> bool {
        let mut inner = self.0.lock().await;
        match inner.query.pop() {
            Some(value) => value,
            None => {
                let _ = inner.end.unbounded_send(());
                inner
                    .recv
                    .next()
                    .await
                    .expect("replay receiver should yield a bool after query exhaustion")
            }
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayState {
    color: bool,
    history: String,
}

impl From<ReplayState> for () {
    fn from(_value: ReplayState) -> Self {}
}

pub struct ReplayColorIsTrue;

impl Predicate<(&ReplayState, &())> for ReplayColorIsTrue {
    fn eval((state, _): &(&ReplayState, &())) -> bool {
        state.color
    }
}

impl Predicate<(ReplayState, ())> for ReplayColorIsTrue {
    fn eval((state, _): &(ReplayState, ())) -> bool {
        state.color
    }
}

pub struct ReplayAlwaysTrue;

impl Predicate<(&ReplayState, &())> for ReplayAlwaysTrue {
    fn eval((_state, _): &(&ReplayState, &())) -> bool {
        true
    }
}

trait ReplayTockRuntime {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send;
}

impl ReplayTockRuntime for () {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        std::future::ready(false)
    }
}

impl ReplayTockRuntime for ReplayRainforest {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        self.next()
    }
}

pub struct Tock;

#[jungle::effect(id = 1001)]
impl<J> Effect<J> for Tock
where
    J: ReplayTockRuntime + Sync,
{
    type In = ();
    type Out = bool;
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(jungle.run_tock().await) }
    }
}

pub struct Tick;

#[jungle::action]
impl Action for Tick {
    type Effect = Tock;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let tocked = output.map_err(|_err| Failure::from("tock should succeed"))?;
            if tocked {
                state.color = true;
            } else {
                state.color = false;
            }
        };
        Ok(__absorb_out_1)
    }
}

pub struct Label<const CH: char>;

#[jungle::action]
impl<const CH: char> Action for Label<CH> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            output.map_err(|_err| Failure::from("label should complete without effect"))?;
            state.history.push(CH);
        };
        Ok(__absorb_out_2)
    }
}

pub struct FlattenReplayChoice;

#[jungle::action]
impl Action for FlattenReplayChoice {
    type Effect = NoEffect;
    type Input = Either<(), ()>;
    type Output = ();
    type Carry = Either<(), ()>;

    fn emit(_state: &ReplayState, input: Self::Input) -> ((), Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            output.map_err(|_err| Failure::from("flatten replay choice should succeed"))?;
            match carry {
                Either::Left(()) | Either::Right(()) => (),
            }
        };
        Ok(__absorb_out_3)
    }
}

#[derive(Flow)]
pub struct Depth1LeftBranch(Step<Label<'L'>>, Step<Tick>, Step<Tick>, Step<Tick>);

#[derive(Flow)]
pub struct Depth1RightBranch(
    Step<Label<'R'>>,
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
);

#[derive(Flow)]
pub struct Depth1InnerBody(
    Step<Label<'I'>>,
    Step<Tick>,
    Step<Tick>,
    Conditional<ReplayColorIsTrue, Depth1LeftBranch, Depth1RightBranch>,
    Step<FlattenReplayChoice>,
);

#[derive(Flow)]
pub struct Depth1OuterBody(
    Step<Label<'O'>>,
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
    While<ReplayColorIsTrue, Depth1InnerBody>,
    Step<Tick>,
    Step<Tick>,
);

#[derive(Flow)]
pub struct Depth1Flow(While<ReplayAlwaysTrue, Depth1OuterBody>);

pub struct Depth1;

#[jungle::animal(observe, id = 1002, generation = 0)]
impl Animal for Depth1 {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = Depth1Flow;
}

impl Observe for Depth1 {
    type Appearance = String;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.history.clone()
    }
}

#[derive(Animals)]
pub struct ReplayRainforestAnimals(Depth1);

const REPLAY_TEST_OWNER_LEASE_TTL_MS: i64 = 250;
const REPLAY_TEST_CLAIMED_WORK_TTL_MS: i64 = 1_000;
const REPLAY_TEST_FIRST_BOUNDARY_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_RECLAIM_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_APPEARANCE_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_RESUME_TICKS_TO_NEXT_LABEL: usize = 6;

fn replay_rainforest(
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
) -> ReplayRainforest {
    ReplayRainforest(Arc::new(Mutex::new(ReplayInner { query, end, recv })))
}

fn spawn_depth1_worker(
    client: FusedClient,
    jungle: ReplayRainforest,
    owner_lease_ttl_ms: i64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = JungleWorker::new(jungle, client).with_owner_lease_ttl_ms(owner_lease_ttl_ms);
        let _ = worker.spawn().await;
    })
}

async fn current_depth1_history(client: &FusedClient, journey_id: uuid::Uuid) -> String {
    let Some(appearance_bytes) = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed")
    else {
        return String::new();
    };
    postcard::from_bytes::<String>(&appearance_bytes)
        .expect("depth1 appearance should deserialize as a String")
}

async fn wait_for_depth1_history_change(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    previous: &str,
) -> String {
    tokio::time::timeout(REPLAY_TEST_APPEARANCE_TIMEOUT, async {
        loop {
            let history = current_depth1_history(client, journey_id).await;
            if history != previous {
                break history;
            }
            let _ = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for appearance change");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("depth1 appearance should change before timeout")
}

async fn assert_replayed_depth1_history_extends_prefix(query: Vec<bool>) {
    let client = FusedClient::builder()
        .claimed_work_ttl_ms(REPLAY_TEST_CLAIMED_WORK_TTL_MS)
        .namespace(format!("depth1-property-{}", uuid::Uuid::new_v4()))
        .build()
        .await
        .expect("fused client should build");

    let (end_tx, mut end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (worker_one_resume_tx, worker_one_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_one = spawn_depth1_worker(
        client.clone(),
        replay_rainforest(query, end_tx.clone(), worker_one_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );

    let journey_id = client
        .spawn::<Depth1>(&ReplayState::default())
        .await
        .expect("depth1 journey should start")
        .journey_id;

    tokio::time::timeout(REPLAY_TEST_FIRST_BOUNDARY_TIMEOUT, end_rx.next())
        .await
        .expect("first depth1 end signal should arrive before timeout")
        .expect("first depth1 end signal channel should remain open");

    let killed_worker_history = current_depth1_history(&client, journey_id).await;

    worker_one.abort();
    let _ = worker_one.await;
    drop(worker_one_resume_tx);

    let (worker_two_resume_tx, worker_two_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_two = spawn_depth1_worker(
        client.clone(),
        replay_rainforest(Vec::new(), end_tx, worker_two_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );

    tokio::time::timeout(REPLAY_TEST_RECLAIM_TIMEOUT, async {
        loop {
            tokio::select! {
                maybe_end = end_rx.next() => {
                    if maybe_end.is_some() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(25)) => {
                    let _ = client
                        .journey_details(journey_id)
                        .await
                        .expect("journey_details should succeed while waiting for replay boundary");
                }
            }
        }
    })
    .await
    .expect("replayed depth1 end signal should arrive before timeout");

    for _ in 0..REPLAY_TEST_RESUME_TICKS_TO_NEXT_LABEL {
        worker_two_resume_tx
            .unbounded_send(true)
            .expect("replayed depth1 receiver should accept replay bools");
    }

    let replayed_history =
        wait_for_depth1_history_change(&client, journey_id, &killed_worker_history).await;

    assert!(
        replayed_history.starts_with(&killed_worker_history),
        "killed worker history should be a prefix of replayed worker history: old={killed_worker_history:?} new={replayed_history:?}"
    );

    worker_two.abort();
    let _ = worker_two.await;
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 16,
        .. ProptestConfig::default()
    })]

    #[test]
    fn depth1_replay_history_from_replayed_worker_has_killed_worker_prefix(
        query in proptest::collection::vec(any::<bool>(), 0..513)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth1_history_extends_prefix(query));
    }
}
