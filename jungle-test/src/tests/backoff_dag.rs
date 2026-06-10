use futures::StreamExt;
use jungle_sdk::core::dag::{
    ClusterKind, ClusterLive, Dag, DagSnapshot, LiveDagState, Phase, RuntimeState,
};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::backoff::Println;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

#[derive(Flow)]
struct FailingSubflow(Step<AnnounceFailure<(), ()>>, Step<Fail<()>>);

type BackoffDagFlow =
    jungle_zoo::backoff::Backoff<(), (), (), FailingSubflow, 100u64, 10000u64, 2u8>;

struct BackoffDagAnimal;

#[jungle::animal(id = 212, generation = 0)]
impl Animal for BackoffDagAnimal {
    type State = ();
    type Seed = ();
    type Flow = BackoffDagFlow;
}

#[derive(Animals)]
struct BackoffDagAnimals(BackoffDagAnimal);

struct BackoffDagZoo;

impl Ecosystem for BackoffDagZoo {
    const NAME: &'static str = "backoff-dag-zoo";
    type Animals = BackoffDagAnimals;
}

#[derive(Clone, Debug, PartialEq)]
struct Fail<In>(PhantomData<In>);

#[jungle::action]
impl<In> Action for Fail<In> {
    type Effect = NoEffect;
    type Input = In;
    type Output = In;

    fn emit(_state: &(), _input: Self::Input) {}

    fn absorb(
        _state: &mut (),
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::Message("Failed!".to_string()))
    }
}

struct AnnounceFailure<St, T>(PhantomData<St>, PhantomData<T>);

#[jungle::action(carry = T)]
impl<St, T> Action for AnnounceFailure<St, T> {
    type Effect = Println<String>;
    type Input = T;
    type Output = T;

    fn emit(_state: &St, input: Self::Input) -> (String, T) {
        ("Failing!".to_string(), input)
    }

    fn absorb(
        _state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: T,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

#[tokio::test]
async fn backoff_snapshot_marks_sleep_running_and_fail_failed() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let backend = Server::builder()
        .redb_path(&db_path)
        .build()
        .await
        .expect("local server backend should build");
    let client = FusedClient::builder()
        .namespace(BackoffDagZoo::NAME)
        .backend(backend)
        .build()
        .await
        .expect("local fused client should build");

    let worker_client = client.clone();
    let worker = JungleWorker::new(BackoffDagZoo, worker_client);
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey_id = client
        .spawn::<BackoffDagAnimal>(&())
        .await
        .expect("journey should spawn")
        .journey_id;

    let dag = Dag::from_ast(<BackoffDagFlow as JourneyAstSource>::journey_ast());
    let inc_iter_labels = dag
        .nodes
        .iter()
        .map(|node| node.label.as_str())
        .filter(|label| label.starts_with("IncIter"))
        .collect::<Vec<_>>();
    assert_eq!(
        inc_iter_labels.len(),
        1,
        "backoff DAG should contain exactly one IncIter node, found: {inc_iter_labels:?}"
    );
    let init_iter_labels = dag
        .nodes
        .iter()
        .map(|node| node.label.as_str())
        .filter(|label| *label == "InitLoopIter")
        .collect::<Vec<_>>();
    assert_eq!(
        init_iter_labels.len(),
        1,
        "backoff DAG should contain exactly one InitLoopIter node, found: {init_iter_labels:?}"
    );
    let inc_iter_runtime_id = dag
        .nodes
        .iter()
        .find(|node| node.label == "IncIter")
        .and_then(|node| node.runtime_node_id)
        .expect("IncIter node should have a runtime id");
    let announce_runtime_id = dag
        .nodes
        .iter()
        .find(|node| node.label == "AnnounceFailure")
        .and_then(|node| node.runtime_node_id)
        .expect("AnnounceFailure node should have a runtime id");

    let mut subscription = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("subscribe_step_updates should succeed");

    let mut updates = Vec::new();
    let mut sleep_scheduled_count = 0usize;
    let mut inc_iter_entered_count = 0usize;
    let mut saw_announce_success = false;
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        let maybe_update =
            match tokio::time::timeout(Duration::from_millis(250), subscription.next()).await {
                Ok(update) => update,
                Err(_) => continue,
            };
        let update = match maybe_update {
            Some(Ok(update)) => update,
            Some(Err(err)) => panic!("step update should decode: {err}"),
            None => break,
        };

        if matches!(update.event, RunnerUpdateOut::SleepScheduled { .. }) {
            sleep_scheduled_count += 1;
        }
        if matches!(
            update.event,
            RunnerUpdateOut::NodeLifecycle(NodeLifecycle {
                node_id,
                phase: NodeLifecyclePhase::Entered,
                ..
            }) if node_id == inc_iter_runtime_id
        ) {
            inc_iter_entered_count += 1;
        }
        if matches!(
            update.event,
            RunnerUpdateOut::NodeLifecycle(NodeLifecycle {
                node_id,
                phase: NodeLifecyclePhase::Succeeded,
                ..
            }) if node_id == announce_runtime_id
        ) {
            saw_announce_success = true;
        }
        updates.push(update);
        if sleep_scheduled_count >= 2 && inc_iter_entered_count >= 2 && saw_announce_success {
            break;
        }
    }

    assert!(
        sleep_scheduled_count >= 2,
        "backoff flow should schedule sleep across multiple iterations"
    );
    assert!(
        inc_iter_entered_count >= 2,
        "backoff flow should re-enter IncIter after attempt-side progress"
    );
    assert!(
        saw_announce_success,
        "backoff flow should execute announce step inside attempt branch"
    );

    let mut live = LiveDagState::default();
    live.bind_model(&dag);
    for update in updates {
        let _ = live.apply_update(update);
    }

    let snapshot = DagSnapshot::new(&dag, Some(&live));
    let state_for = |label: &str| {
        let display_id = dag
            .nodes
            .iter()
            .find(|node| node.label == label)
            .map(|node| node.id)
            .unwrap_or_else(|| panic!("missing node with label {label}"));
        snapshot
            .node_states
            .get(&display_id)
            .copied()
            .unwrap_or(RuntimeState::Pending)
    };

    assert_eq!(state_for("IncIter"), RuntimeState::Completed);
    assert_eq!(state_for("SleepMult"), RuntimeState::Running);
    assert_eq!(state_for("Fail"), RuntimeState::Failed);

    let attempt_index = dag
        .cluster_info
        .iter()
        .position(|cluster| matches!(cluster.kind, ClusterKind::Attempt))
        .expect("attempt cluster should exist");
    let attempt_phase = snapshot.cluster_phase(attempt_index);
    assert!(matches!(
        attempt_phase,
        Phase::Live(ClusterLive {
            has_failed: true,
            ..
        })
    ));

    worker_handle.abort();
    let _ = worker_handle.await;
}
