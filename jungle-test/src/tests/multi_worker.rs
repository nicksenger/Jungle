use futures::StreamExt;
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, JourneyStatus, JungleClient, LocalClient, RunnerUpdateOut};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

const STEPS_PER_JOURNEY: usize = 100;
const WORKER_COUNT: usize = 5;
const JOURNEY_COUNT: usize = 2;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiWorkerState {
    completed_steps: u16,
}

pub struct MultiWorkerSleepEffect;

#[jungle::effect(id = 501)]
impl<J> Effect<J> for MultiWorkerSleepEffect {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        }
    }
}

pub struct MultiWorkerSleepStepSpec;

#[jungle::act]
impl Act for MultiWorkerSleepStepSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("multi-worker sleep effect should succeed");
        state.completed_steps += 1;
    }
}

#[derive(Flow)]
pub struct MultiWorkerTenStepFlow(
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
    Step<MultiWorkerSleepStepSpec>,
);

#[derive(Flow)]
pub struct MultiWorkerHundredStepFlow(
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
    MultiWorkerTenStepFlow,
);

pub struct MultiWorkerAnimal;

#[jungle::animal(id = 91, generation = 0)]
impl Animal for MultiWorkerAnimal {
    type State = MultiWorkerState;
    type Seed = MultiWorkerState;
    type Journey = MultiWorkerHundredStepFlow;
}

#[derive(Animals)]
pub struct MultiWorkerAnimals(MultiWorkerAnimal);

pub struct MultiWorkerZoo;

impl Ecosystem for MultiWorkerZoo {
    const NAME: &'static str = "multi-worker-zoo";
    type Animals = MultiWorkerAnimals;
}

impl From<MultiWorkerState> for () {
    fn from(_value: MultiWorkerState) -> Self {}
}

#[tokio::test]
async fn local_client_multi_worker_does_not_repeat_completed_steps() {
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

    let mut journey_ids = Vec::with_capacity(JOURNEY_COUNT);
    for _ in 0..JOURNEY_COUNT {
        let seed =
            postcard::to_allocvec(&MultiWorkerState::default()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<MultiWorkerAnimal>(seed)
            .await
            .expect("journey should start");
        journey_ids.push(journey_id);
    }
    assert_eq!(journey_ids.len(), JOURNEY_COUNT);
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
        tokio::time::timeout(Duration::from_secs(30), async {
            tokio::join!(
                consume_journey_updates(&mut journey_one_stream, journey_one),
                consume_journey_updates(&mut journey_two_stream, journey_two),
            )
        })
        .await
        .expect("journey update streams should complete before timeout");

    assert_eq!(
        journey_one_counts.started, STEPS_PER_JOURNEY,
        "first journey should emit exactly one step start per node"
    );
    assert_eq!(
        journey_one_counts.succeeded, STEPS_PER_JOURNEY,
        "first journey should emit exactly one step success per node"
    );
    assert_eq!(
        journey_one_counts.failed, 0,
        "first journey should not fail any step"
    );

    assert_eq!(
        journey_two_counts.started, STEPS_PER_JOURNEY,
        "second journey should emit exactly one step start per node"
    );
    assert_eq!(
        journey_two_counts.succeeded, STEPS_PER_JOURNEY,
        "second journey should emit exactly one step success per node"
    );
    assert_eq!(
        journey_two_counts.failed, 0,
        "second journey should not fail any step"
    );

    let journey_one_status = client
        .journey_details(journey_one)
        .await
        .expect("first journey_details should succeed");
    let journey_two_status = client
        .journey_details(journey_two)
        .await
        .expect("second journey_details should succeed");

    assert_eq!(journey_one_status, JourneyStatus::Completed);
    assert_eq!(journey_two_status, JourneyStatus::Completed);

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
    let mut started_nodes = HashSet::new();
    let mut succeeded_nodes = HashSet::new();
    let mut failed_nodes = HashSet::new();

    while let Some(next) = stream.next().await {
        let update = next.expect("streamed journey update should succeed");

        match update.event {
            RunnerUpdateOut::EffectInput { uuid, node_id } => {
                assert_eq!(
                    uuid, journey_id,
                    "step start must belong to subscribed journey"
                );
                assert!(
                    started_nodes.insert(node_id),
                    "duplicate step start observed for journey {journey_id} node {node_id}"
                );
            }
            RunnerUpdateOut::EffectSuccessOutput { uuid, node_id } => {
                assert_eq!(
                    uuid, journey_id,
                    "step success must belong to subscribed journey"
                );
                assert!(
                    succeeded_nodes.insert(node_id),
                    "duplicate step success observed for journey {journey_id} node {node_id}"
                );
            }
            RunnerUpdateOut::EffectFailureOutput { uuid, node_id } => {
                assert_eq!(
                    uuid, journey_id,
                    "step failure must belong to subscribed journey"
                );
                assert!(
                    failed_nodes.insert(node_id),
                    "duplicate step failure observed for journey {journey_id} node {node_id}"
                );
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
    }

    JourneyCounts {
        started: started_nodes.len(),
        succeeded: succeeded_nodes.len(),
        failed: failed_nodes.len(),
    }
}
