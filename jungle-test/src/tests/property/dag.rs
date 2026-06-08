use super::ecosystem::{
    replay_rainforest, Depth1, Depth1Flow, Depth2, Depth2Flow, Depth2State, Depth3, Depth3Flow,
    Depth3State, ReplayRainforest, ReplayState,
};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::core::dag::{ClusterKind, Dag, DagSnapshot, LiveDagState, Phase, RuntimeState};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use proptest::prelude::*;
use serde::Serialize;
use std::time::Duration;
use tokio::task::JoinHandle;

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
const DAG_TEST_END_POLL_INTERVAL: Duration = Duration::from_millis(25);
const DAG_TEST_STREAM_SETTLE_TIMEOUT: Duration = Duration::from_millis(100);
const DAG_TEST_TRUE_PUBLISH_INTERVAL: Duration = Duration::from_millis(10);

fn spawn_replay_worker(
    client: FusedClient,
    jungle: ReplayRainforest,
    owner_lease_ttl_ms: i64,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let worker = JungleWorker::new(jungle, client).with_owner_lease_ttl_ms(owner_lease_ttl_ms);
        let _ = worker.spawn().await;
    })
}

struct UpdateCollector {
    stop_tx: tokio::sync::oneshot::Sender<()>,
    join: JoinHandle<Vec<JourneyUpdateEvent>>,
}

impl UpdateCollector {
    async fn finish(self) -> Vec<JourneyUpdateEvent> {
        let _ = self.stop_tx.send(());
        self.join
            .await
            .expect("journey update collector task should complete cleanly")
    }
}

struct TruePublisher {
    stop_tx: tokio::sync::oneshot::Sender<()>,
    join: JoinHandle<()>,
}

impl TruePublisher {
    async fn stop(self) {
        let _ = self.stop_tx.send(());
        let _ = self.join.await;
    }
}

fn rendered_graph_from_updates<F>(
    updates: impl IntoIterator<Item = JourneyUpdateEvent>,
) -> RenderedDag
where
    F: JourneyAstSource,
{
    let dag = Dag::from_ast(<F as JourneyAstSource>::journey_ast());
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

async fn spawn_update_collector(client: &FusedClient, journey_id: uuid::Uuid) -> UpdateCollector {
    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let mut updates = Vec::new();

        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                maybe_update = subscription.next() => {
                    let update = maybe_update
                        .expect("journey update stream should remain open until collector stop")
                        .expect("streamed journey update should succeed");
                    updates.push(update);
                }
            }
        }

        loop {
            match tokio::time::timeout(DAG_TEST_STREAM_SETTLE_TIMEOUT, subscription.next()).await {
                Ok(Some(Ok(update))) => updates.push(update),
                Ok(Some(Err(err))) => panic!("streamed journey update should succeed: {err}"),
                Ok(None) | Err(_) => break,
            }
        }

        updates
    });

    UpdateCollector { stop_tx, join }
}

fn spawn_true_publisher(sender: UnboundedSender<bool>) -> TruePublisher {
    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = tokio::time::sleep(DAG_TEST_TRUE_PUBLISH_INTERVAL) => {
                    if sender.unbounded_send(true).is_err() {
                        break;
                    }
                }
            }
        }
    });

    TruePublisher { stop_tx, join }
}

async fn wait_for_end_signal(end_rx: &mut UnboundedReceiver<()>, label: &str) {
    tokio::time::timeout(DAG_TEST_BOUNDARY_TIMEOUT, end_rx.next())
        .await
        .unwrap_or_else(|_| panic!("{label} end signal should arrive before timeout"))
        .unwrap_or_else(|| panic!("{label} end signal channel should remain open"));
}

async fn wait_for_quiescent_end_signal(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    end_rx: &mut UnboundedReceiver<()>,
) {
    tokio::time::timeout(DAG_TEST_BOUNDARY_TIMEOUT, async {
        let mut saw_end = false;

        loop {
            match tokio::time::timeout(DAG_TEST_STREAM_SETTLE_TIMEOUT, end_rx.next()).await {
                Ok(Some(())) => saw_end = true,
                Ok(None) => {
                    panic!("end signal channel should remain open while waiting for quiescence")
                }
                Err(_) if saw_end => break,
                Err(_) => {
                    let _ = client.journey_details(journey_id).await.expect(
                        "journey_details should succeed while waiting for replay quiescence",
                    );
                    tokio::time::sleep(DAG_TEST_END_POLL_INTERVAL).await;
                }
            }
        }
    })
    .await
    .expect("replayed journey should reach a quiescent end signal before timeout");
}

async fn assert_replayed_graph_matches_live<A, S, F>(query: Vec<bool>, namespace: &str, seed: S)
where
    A: Animal<State = S, Seed = S, Flow = F>,
    A::Id: AnimalIdValue,
    A::Generation: jungle_sdk::typosaurus::num::Unsigned,
    S: Serialize + Sync,
    F: JourneyAstSource,
{
    let client = FusedClient::builder()
        .claimed_work_ttl_ms(REPLAY_TEST_CLAIMED_WORK_TTL_MS)
        .namespace(format!("{namespace}-dag-property-{}", uuid::Uuid::new_v4()))
        .build()
        .await
        .expect("fused client should build");

    let (end_tx, mut end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (worker_one_resume_tx, worker_one_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_one = spawn_replay_worker(
        client.clone(),
        replay_rainforest(query, end_tx.clone(), worker_one_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );

    let journey_id = client
        .spawn::<A>(&seed)
        .await
        .expect("replay journey should start")
        .journey_id;

    let live_collector = spawn_update_collector(&client, journey_id).await;

    wait_for_end_signal(&mut end_rx, "initial").await;

    worker_one.abort();
    let _ = worker_one.await;
    drop(worker_one_resume_tx);

    let (worker_two_resume_tx, worker_two_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let true_publisher = spawn_true_publisher(worker_two_resume_tx.clone());
    let worker_two = spawn_replay_worker(
        client.clone(),
        replay_rainforest(Vec::new(), end_tx, worker_two_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );
    let replay_collector = spawn_update_collector(&client, journey_id).await;

    true_publisher.stop().await;
    wait_for_quiescent_end_signal(&client, journey_id, &mut end_rx).await;

    let live_graph = rendered_graph_from_updates::<F>(live_collector.finish().await);
    let replay_graph = rendered_graph_from_updates::<F>(replay_collector.finish().await);

    assert_eq!(
        live_graph, replay_graph,
        "live DAG render state should match replay DAG render state"
    );

    worker_two.abort();
    let _ = worker_two.await;
    drop(worker_two_resume_tx);
}

async fn assert_replayed_depth1_graph_matches_live(query: Vec<bool>) {
    assert_replayed_graph_matches_live::<Depth1, ReplayState, Depth1Flow>(
        query,
        "depth1",
        ReplayState::default(),
    )
    .await;
}

async fn assert_replayed_depth2_graph_matches_live(query: Vec<bool>) {
    assert_replayed_graph_matches_live::<Depth2, Depth2State, Depth2Flow>(
        query,
        "depth2",
        Depth2State::default(),
    )
    .await;
}

async fn assert_replayed_depth3_graph_matches_live(query: Vec<bool>) {
    assert_replayed_graph_matches_live::<Depth3, Depth3State, Depth3Flow>(
        query,
        "depth3",
        Depth3State::default(),
    )
    .await;
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

    #[test]
    fn depth2_replay_graph_matches_live_graph(
        query in proptest::collection::vec(any::<bool>(), 0..513)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth2_graph_matches_live(query));
    }

    #[test]
    fn depth3_replay_graph_matches_live_graph(
        query in proptest::collection::vec(any::<bool>(), 0..513)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth3_graph_matches_live(query));
    }
}

