use super::ecosystem::{
    replay_rainforest, ConditionalCompleteProbe, ConditionalProbe, Depth1, Depth1State, Depth2,
    Depth2State, Depth3, Depth3State, ReplayRainforest, ReplayState, Tock,
};
use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use proptest::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

const REPLAY_TEST_OWNER_LEASE_TTL_MS: i64 = 250;
const REPLAY_TEST_CLAIMED_WORK_TTL_MS: i64 = 1_000;
const REPLAY_TEST_FIRST_BOUNDARY_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_RECLAIM_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_APPEARANCE_TIMEOUT: Duration = Duration::from_secs(10);
const REPLAY_TEST_KILLED_WORKER_APPEARANCE_DRAIN: Duration = Duration::from_secs(1);

fn spawn_replay_worker(
    client: FusedClient,
    jungle: ReplayRainforest,
    owner_lease_ttl_ms: i64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = JungleWorker::new(jungle, client).with_owner_lease_ttl_ms(owner_lease_ttl_ms);
        let _ = worker.spawn().await;
    })
}

async fn current_replay_history(client: &FusedClient, journey_id: uuid::Uuid) -> String {
    let Some(appearance_bytes) = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed")
    else {
        return String::new();
    };
    postcard::from_bytes::<String>(&appearance_bytes)
        .expect("replay appearance should deserialize as a String")
}

async fn wait_for_replay_history_change(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    previous: &str,
) -> String {
    match tokio::time::timeout(REPLAY_TEST_APPEARANCE_TIMEOUT, async {
        loop {
            let history = current_replay_history(client, journey_id).await;
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
    {
        Ok(history) => history,
        Err(_) => {
            let latest_history = current_replay_history(client, journey_id).await;
            panic!(
                "replay appearance should change before timeout: previous={previous:?} latest={latest_history:?}"
            );
        }
    }
}

async fn latest_replay_history_within_window(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    window: Duration,
) -> String {
    let deadline = tokio::time::Instant::now() + window;
    let mut latest_history = current_replay_history(client, journey_id).await;

    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break latest_history;
        }

        let _ = client
            .journey_details(journey_id)
            .await
            .expect("journey_details should succeed while draining killed worker appearance");

        let sleep_for = std::cmp::min(Duration::from_millis(25), deadline - now);
        tokio::time::sleep(sleep_for).await;

        let history = current_replay_history(client, journey_id).await;
        if history.len() >= latest_history.len() {
            latest_history = history;
        }
    }
}

async fn assert_replayed_history_extends_prefix<A, S>(query: Vec<bool>, namespace: &str, seed: S)
where
    A: Animal<State = S, Seed = S> + Observe<Appearance = String>,
    A::Id: AnimalIdValue,
    A::Generation: jungle_sdk::typosaurus::num::Unsigned,
    S: Serialize + Sync,
{
    let client = FusedClient::builder()
        .claimed_work_ttl_ms(REPLAY_TEST_CLAIMED_WORK_TTL_MS)
        .namespace(format!("{namespace}-{}", uuid::Uuid::new_v4()))
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

    tokio::time::timeout(REPLAY_TEST_FIRST_BOUNDARY_TIMEOUT, end_rx.next())
        .await
        .expect("first replay end signal should arrive before timeout")
        .expect("first depth1 end signal channel should remain open");

    let killed_worker_history = latest_replay_history_within_window(
        &client,
        journey_id,
        REPLAY_TEST_KILLED_WORKER_APPEARANCE_DRAIN,
    )
    .await;

    worker_one.abort();
    drop(worker_one_resume_tx);

    let (worker_two_resume_tx, worker_two_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_two = spawn_replay_worker(
        client.clone(),
        replay_rainforest(Vec::new(), end_tx, worker_two_resume_rx),
        REPLAY_TEST_OWNER_LEASE_TTL_MS,
    );

    tokio::time::timeout(REPLAY_TEST_RECLAIM_TIMEOUT, end_rx.next())
        .await
        .expect("replayed end signal should arrive before timeout")
        .expect("replayed end signal channel should remain open");

    worker_two_resume_tx
        .unbounded_send(true)
        .expect("replay resume signal should send once after replay boundary");
    drop(worker_two_resume_tx);

    let replayed_history =
        wait_for_replay_history_change(&client, journey_id, &killed_worker_history).await;

    eprintln!("{namespace} replay case: old={killed_worker_history:?} new={replayed_history:?}");

    assert!(
        replayed_history.starts_with(&killed_worker_history),
        "killed worker history should be a prefix of replayed worker history: old={killed_worker_history:?} new={replayed_history:?}"
    );

    worker_two.abort();
}

async fn assert_replayed_depth1_history_extends_prefix(query: Vec<bool>) {
    assert_replayed_history_extends_prefix::<Depth1, Depth1State>(
        query,
        "depth1-property",
        Depth1State::default(),
    )
    .await;
}

async fn assert_replayed_depth2_history_extends_prefix(query: Vec<bool>) {
    assert_replayed_history_extends_prefix::<Depth2, Depth2State>(
        query,
        "depth2-property",
        Depth2State::default(),
    )
    .await;
}

async fn assert_replayed_depth3_history_extends_prefix(query: Vec<bool>) {
    assert_replayed_history_extends_prefix::<Depth3, Depth3State>(
        query,
        "depth3-property",
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
    fn depth1_replay_history_from_replayed_worker_has_killed_worker_prefix(
        query in proptest::collection::vec(any::<bool>(), 0..65)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth1_history_extends_prefix(query));
    }

    #[test]
    fn depth2_replay_history_from_replayed_worker_has_killed_worker_prefix(
        query in proptest::collection::vec(any::<bool>(), 0..65)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth2_history_extends_prefix(query));
    }

    #[test]
    fn depth3_replay_history_from_replayed_worker_has_killed_worker_prefix(
        query in proptest::collection::vec(any::<bool>(), 0..65)
    ) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime should build for property test");
        runtime.block_on(assert_replayed_depth3_history_extends_prefix(query));
    }
}

#[tokio::test]
async fn depth1_fixed_trace_applies_right_branch_label_before_next_effect_request() {
    let (end_tx, _end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (_resume_tx, resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let jungle = replay_rainforest(vec![false, false, true, false, false], end_tx, resume_rx);
    let mut executor =
        ContextExecutor::<ReplayRainforest, Depth1>::new(Arc::new(jungle), ReplayState::default());

    for _ in 0..5 {
        let _ = executor
            .next_and_complete_with(())
            .await
            .expect("fixed replay step should complete");
    }

    let request = executor
        .next_executable_request(())
        .expect("fixed replay should reach the next executable request");

    assert_eq!(executor.state().history, "O001I00R");
    assert_eq!(
        request.effect_type(),
        std::any::type_name::<Tock>(),
        "right branch label should be absorbed before the next Tick request"
    );
}

#[tokio::test]
async fn depth3_fixed_replay_query_resumes_after_nested_join_replay() {
    assert_replayed_depth3_history_extends_prefix(vec![true, false, false]).await;
}

#[tokio::test]
async fn conditional_probe_applies_right_branch_label_before_next_effect_request() {
    let (end_tx, _end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (_resume_tx, resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let jungle = replay_rainforest(vec![false, false, false], end_tx, resume_rx);
    let mut executor = ContextExecutor::<ReplayRainforest, ConditionalProbe>::new(
        Arc::new(jungle),
        ReplayState::default(),
    );

    for _ in 0..3 {
        let _ = executor
            .next_and_complete_with(())
            .await
            .expect("probe ticks should complete");
    }

    let request = executor
        .next_executable_request(())
        .expect("probe should reach the next executable request");

    assert_eq!(executor.state().history, "000R");
    assert_eq!(request.effect_type(), std::any::type_name::<Tock>());
}

#[tokio::test]
async fn conditional_probe_wraps_completed_right_branch_output_before_following_step() {
    let (end_tx, _end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (_resume_tx, resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let jungle = replay_rainforest(vec![false, false, false, true], end_tx, resume_rx);
    let mut executor = ContextExecutor::<ReplayRainforest, ConditionalCompleteProbe>::new(
        Arc::new(jungle),
        ReplayState::default(),
    );

    for _ in 0..3 {
        let _ = executor
            .next_and_complete_with(())
            .await
            .expect("probe ticks should complete");
    }

    let request = executor
        .next_executable_request(())
        .expect("completed conditional branch should feed the next Tick");

    assert_eq!(executor.state().history, "100R");
    assert_eq!(request.effect_type(), std::any::type_name::<Tock>());
}
