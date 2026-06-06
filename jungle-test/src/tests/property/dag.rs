use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::core::dag::{ClusterKind, Dag, DagSnapshot, LiveDagState, Phase, RuntimeState};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct ReplayRainforest(Arc<Mutex<ReplayInner>>);

struct ReplayInner {
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayState {
    color: bool,
    history: String,
}

impl From<ReplayState> for () {
    fn from(_value: ReplayState) -> Self {}
}

pub(crate) struct ReplayColorIsTrue;

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

pub(crate) struct ReplayAlwaysTrue;

impl Predicate<(&ReplayState, &())> for ReplayAlwaysTrue {
    fn eval((_state, _): &(&ReplayState, &())) -> bool {
        true
    }
}

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest-dag";
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

#[jungle::effect(id = 1003)]
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
        let tocked = output.map_err(|_err| Failure::from("tock should succeed"))?;
        if tocked {
            state.color = true;
            state.history.push('1');
        } else {
            state.color = false;
            state.history.push('0');
        }
        Ok(())
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
        output.map_err(|_err| Failure::from("label should complete without effect"))?;
        state.history.push(CH);
        Ok(())
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
        output.map_err(|_err| Failure::from("flatten replay choice should succeed"))?;
        match carry {
            Either::Left(()) | Either::Right(()) => (),
        }
        Ok(())
    }
}

#[derive(Flow)]
struct Depth1LeftBranch(Step<Label<'L'>>, Step<Tick>, Step<Tick>, Step<Tick>);

#[derive(Flow)]
struct Depth1RightBranch(
    Step<Label<'R'>>,
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
);

#[derive(Flow)]
struct Depth1InnerBody(
    Step<Tick>,
    Step<Tick>,
    Conditional<ReplayColorIsTrue, Depth1LeftBranch, Depth1RightBranch>,
    Step<FlattenReplayChoice>,
);

#[derive(Flow)]
struct Depth1OuterBody(
    Step<Tick>,
    Step<Tick>,
    Step<Tick>,
    While<ReplayColorIsTrue, Depth1InnerBody>,
    Step<Tick>,
    Step<Tick>,
);

#[derive(Flow)]
struct Depth1Flow(While<ReplayAlwaysTrue, Depth1OuterBody>);

struct Depth1;

#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Depth1 {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = Depth1Flow;
}

#[derive(Animals)]
struct ReplayRainforestAnimals(Depth1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedDagNode {
    id: u32,
    label: String,
    runtime_id: Option<u32>,
    phase: Phase<RuntimeState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedDagCluster {
    id: u32,
    kind: ClusterKind,
    label: String,
    parent: Option<usize>,
    nodes: Vec<u32>,
    phase: Phase<jungle_sdk::core::dag::ClusterLive>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedDag {
    nodes: Vec<RenderedDagNode>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<RenderedDagCluster>,
}

const REPLAY_TEST_OWNER_LEASE_TTL_MS: i64 = 250;
const REPLAY_TEST_CLAIMED_WORK_TTL_MS: i64 = 1_000;
const DAG_TEST_BOUNDARY_TIMEOUT: Duration = Duration::from_secs(10);
const DAG_TEST_STREAM_SETTLE_TIMEOUT: Duration = Duration::from_millis(100);

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

fn rendered_graph_from_updates(
    updates: impl IntoIterator<Item = JourneyUpdateEvent>,
) -> RenderedDag {
    let dag = Dag::from_ast(<Depth1Flow as JourneyAstSource>::journey_ast());
    let mut live = LiveDagState::default();
    live.bind_model(&dag);
    for update in updates {
        let _ = live.apply_update(update);
    }
    let snapshot = DagSnapshot::new(&dag, Some(&live));

    RenderedDag {
        nodes: dag
            .nodes
            .iter()
            .map(|node| RenderedDagNode {
                id: node.id,
                label: node.label.clone(),
                runtime_id: node.runtime_node_id,
                phase: snapshot.node_phase(node.id),
            })
            .collect(),
        edges: dag.edges.clone(),
        clusters: dag
            .cluster_info
            .iter()
            .enumerate()
            .map(|(index, cluster)| RenderedDagCluster {
                id: cluster.id,
                kind: cluster.kind,
                label: cluster.label.clone(),
                parent: cluster.parent,
                nodes: cluster.nodes.clone(),
                phase: snapshot.cluster_phase(index),
            })
            .collect(),
    }
}

async fn collect_updates_until_boundary(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    end_rx: &mut UnboundedReceiver<()>,
) -> Vec<JourneyUpdateEvent> {
    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");
    let mut updates = Vec::new();

    tokio::time::timeout(DAG_TEST_BOUNDARY_TIMEOUT, async {
        loop {
            tokio::select! {
                maybe_end = end_rx.next() => {
                    maybe_end.expect("end signal channel should remain open until boundary");
                    break;
                }
                maybe_update = subscription.next() => {
                    let update = maybe_update
                        .expect("journey update stream should remain open until boundary")
                        .expect("streamed journey update should succeed");
                    updates.push(update);
                }
            }
        }
    })
    .await
    .expect("boundary should arrive before timeout");

    loop {
        match tokio::time::timeout(DAG_TEST_STREAM_SETTLE_TIMEOUT, subscription.next()).await {
            Ok(Some(Ok(update))) => updates.push(update),
            Ok(Some(Err(err))) => panic!("streamed journey update should succeed: {err}"),
            Ok(None) | Err(_) => break,
        }
    }

    updates
}

async fn assert_replayed_depth1_graph_matches_live(query: Vec<bool>) {
    let client = FusedClient::builder()
        .claimed_work_ttl_ms(REPLAY_TEST_CLAIMED_WORK_TTL_MS)
        .namespace(format!("depth1-dag-property-{}", uuid::Uuid::new_v4()))
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

    let live_graph = rendered_graph_from_updates(
        collect_updates_until_boundary(&client, journey_id, &mut end_rx).await,
    );

    worker_one.abort();
    let _ = worker_one.await;
    drop(worker_one_resume_tx);

    let (worker_two_resume_tx, worker_two_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_two = spawn_depth1_worker(
        client.clone(),
        replay_rainforest(Vec::new(), end_tx, worker_two_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );

    let replay_graph = rendered_graph_from_updates(
        collect_updates_until_boundary(&client, journey_id, &mut end_rx).await,
    );

    assert_eq!(
        live_graph, replay_graph,
        "live DAG render state should match replay DAG render state"
    );

    worker_two.abort();
    let _ = worker_two.await;
    drop(worker_two_resume_tx);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 16,
        .. ProptestConfig::default()
    })]

    #[test]
    fn depth1_replay_graph_matches_live_graph(
        query in proptest::collection::vec(any::<bool>(), 0..513)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth1_graph_matches_live(query));
    }
}
