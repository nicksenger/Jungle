use super::while_loop::{ExampleFlow, NestedState};
use futures::StreamExt;
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, JourneyStatus, JungleClient, LocalClient, RunnerUpdateOut};
use std::time::Duration;

const WORKER_COUNT: usize = 5;
const JOURNEY_COUNT: usize = 2;
const EXPECTED_STEPS_PER_JOURNEY: usize = 104;

pub struct MultiWorkerAnimal;

#[jungle::animal(id = 91, generation = 0)]
impl Animal for MultiWorkerAnimal {
    type State = NestedState;
    type Seed = NestedState;
    type Journey = ExampleFlow;
}

#[derive(Animals)]
pub struct MultiWorkerAnimals(MultiWorkerAnimal);

pub struct MultiWorkerZoo;

impl Ecosystem for MultiWorkerZoo {
    const NAME: &'static str = "multi-worker-zoo";
    type Animals = MultiWorkerAnimals;
}

impl From<NestedState> for () {
    fn from(_value: NestedState) -> Self {}
}

#[tokio::test]
async fn local_client_multi_worker_example_flow_has_expected_events_without_replays() {
    let client = LocalClient::builder()
        .namespace("multi-worker-regression")
        .build()
        .await
        .expect("local client should build");

    let mut worker_handles = Vec::with_capacity(WORKER_COUNT);
    for _ in 0..WORKER_COUNT {
        let worker = JungleWorker::new(MultiWorkerZoo, client.clone());
        worker_handles.push(tokio::spawn(async move {
            let _ = worker.spawn().await;
        }));
    }

    let seed = postcard::to_allocvec(&NestedState::default()).expect("seed should serialize");
    let mut journey_ids = Vec::with_capacity(JOURNEY_COUNT);
    for _ in 0..JOURNEY_COUNT {
        let journey_id = client
            .start_journey::<MultiWorkerAnimal>(seed.clone())
            .await
            .expect("journey should start");
        journey_ids.push(journey_id);
    }

    let journey_one = journey_ids[0];
    let journey_two = journey_ids[1];

    let mut journey_one_stream = client
        .subscribe_step_updates(journey_one, None)
        .await
        .expect("first journey subscribe_step_updates should succeed");
    let mut journey_two_stream = client
        .subscribe_step_updates(journey_two, None)
        .await
        .expect("second journey subscribe_step_updates should succeed");

    let (journey_one_counts, journey_two_counts) =
        tokio::time::timeout(Duration::from_secs(45), async {
            tokio::join!(
                consume_journey_updates(&mut journey_one_stream, journey_one),
                consume_journey_updates(&mut journey_two_stream, journey_two),
            )
        })
        .await
        .expect("journey update streams should complete before timeout");

    assert_eq!(
        journey_one_counts.started, EXPECTED_STEPS_PER_JOURNEY,
        "first journey should emit expected started-step count"
    );
    assert_eq!(
        journey_one_counts.succeeded, EXPECTED_STEPS_PER_JOURNEY,
        "first journey should emit expected succeeded-step count"
    );
    assert_eq!(
        journey_one_counts.failed, 0,
        "first journey should not fail any step"
    );

    assert_eq!(
        journey_two_counts.started, EXPECTED_STEPS_PER_JOURNEY,
        "second journey should emit expected started-step count"
    );
    assert_eq!(
        journey_two_counts.succeeded, EXPECTED_STEPS_PER_JOURNEY,
        "second journey should emit expected succeeded-step count"
    );
    assert_eq!(
        journey_two_counts.failed, 0,
        "second journey should not fail any step"
    );

    assert_eq!(
        client
            .journey_details(journey_one)
            .await
            .expect("first journey_details should succeed"),
        JourneyStatus::Completed
    );
    assert_eq!(
        client
            .journey_details(journey_two)
            .await
            .expect("second journey_details should succeed"),
        JourneyStatus::Completed
    );

    for worker_handle in worker_handles {
        worker_handle.abort();
        let _ = worker_handle.await;
    }
}

struct JourneyCounts {
    started: usize,
    succeeded: usize,
    failed: usize,
}

async fn consume_journey_updates(
    stream: &mut JourneyUpdateSubscription,
    journey_id: uuid::Uuid,
) -> JourneyCounts {
    let mut started = 0_usize;
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;
    let mut last_sequence_id = None;

    while let Some(next) = stream.next().await {
        let update = next.expect("streamed journey update should succeed");
        if let Some(previous) = last_sequence_id {
            assert!(
                update.sequence_id > previous,
                "sequence ids must be strictly increasing"
            );
        }
        last_sequence_id = Some(update.sequence_id);

        match update.event {
            RunnerUpdateOut::EffectInput { uuid, .. } => {
                assert_eq!(uuid, journey_id, "step start must match subscribed journey");
                started += 1;
            }
            RunnerUpdateOut::EffectSuccessOutput { uuid, .. } => {
                assert_eq!(
                    uuid, journey_id,
                    "step success must match subscribed journey"
                );
                succeeded += 1;
            }
            RunnerUpdateOut::EffectFailureOutput { uuid, .. } => {
                assert_eq!(
                    uuid, journey_id,
                    "step failure must match subscribed journey"
                );
                failed += 1;
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
    }

    JourneyCounts {
        started,
        succeeded,
        failed,
    }
}
