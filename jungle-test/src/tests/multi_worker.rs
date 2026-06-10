use futures::StreamExt;
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, FusedClient, JourneyStatus, JungleClient, RunnerUpdateOut};
use std::time::Duration;

const WORKER_COUNT: usize = 5;
const JOURNEY_COUNT: usize = 2;
const MIN_EXPECTED_EVENTS_PER_JOURNEY: usize = 100;
const SINGLE_WORKER_COUNT: usize = 1;
const SINGLE_JOURNEY_COUNT: usize = 1;
const LOOP_ITERATIONS: u8 = 13;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MultiWorkerState {
    iteration: u8,
}

pub struct MultiWorkerSleepEffect;

#[jungle::effect(id = 510)]
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

pub struct MultiWorkerContinue;
impl Predicate<(&MultiWorkerState, &())> for MultiWorkerContinue {
    fn eval((state, _): &(&MultiWorkerState, &())) -> bool {
        state.iteration < LOOP_ITERATIONS
    }
}

pub struct MultiWorkerChooseLeft;
impl Predicate<(MultiWorkerState, ())> for MultiWorkerChooseLeft {
    fn eval((state, _): &(MultiWorkerState, ())) -> bool {
        state.iteration % 2 == 0
    }
}

pub struct MultiWorkerConditionalLeftSpec;
#[jungle::action]
impl Action for MultiWorkerConditionalLeftSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("conditional left step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerConditionalRightSpec;
#[jungle::action]
impl Action for MultiWorkerConditionalRightSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("conditional right step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerJoinLeftSpec;
#[jungle::action]
impl Action for MultiWorkerJoinLeftSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join left step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerJoinRightSpec;
#[jungle::action]
impl Action for MultiWorkerJoinRightSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join right step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerJoinMergeSpec;
#[jungle::action]
impl Action for MultiWorkerJoinMergeSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join merge step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerWorkSpec;
#[jungle::action]
impl Action for MultiWorkerWorkSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("work step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerShortSleepSpec;
#[jungle::action]
impl Action for MultiWorkerShortSleepSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Duration {
        Duration::from_millis(5)
    }

    fn absorb(
        _state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("short sleep step should succeed"))?;
        Ok(())
    }
}

pub struct MultiWorkerAdvanceIterationSpec;
#[jungle::action]
impl Action for MultiWorkerAdvanceIterationSpec {
    type Effect = MultiWorkerSleepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &MultiWorkerState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut MultiWorkerState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("advance iteration step should succeed"))?;
        state.iteration = state.iteration.saturating_add(1);
        Ok(())
    }
}

pub struct MultiWorkerMetaLoop;
pub struct MultiWorkerMetaConditional;
pub struct MultiWorkerMetaJoin;
pub struct MultiWorkerMetaTail;
pub struct MultiWorkerMetaIteration;

type MultiWorkerConditionalSegment = Transparent<
    MultiWorkerMetaConditional,
    Conditional<
        MultiWorkerChooseLeft,
        Step<MultiWorkerConditionalLeftSpec>,
        Step<MultiWorkerConditionalRightSpec>,
    >,
>;

#[derive(Flow)]
pub struct MultiWorkerJoinSegment(
    jungle_zoo::ClonedJoin<
        Either<(), ()>,
        Step<MultiWorkerJoinLeftSpec>,
        Step<MultiWorkerJoinRightSpec>,
    >,
    Step<MultiWorkerJoinMergeSpec>,
);

#[derive(Flow)]
pub struct MultiWorkerTailSegment(
    Step<MultiWorkerWorkSpec>,
    Step<MultiWorkerShortSleepSpec>,
    Step<MultiWorkerWorkSpec>,
    Step<MultiWorkerShortSleepSpec>,
    Step<MultiWorkerWorkSpec>,
    Step<MultiWorkerAdvanceIterationSpec>,
);

#[derive(Flow)]
pub struct ExampleFlow(
    Transparent<
        MultiWorkerMetaLoop,
        While<MultiWorkerContinue, Transparent<MultiWorkerMetaIteration, ExampleFlowIteration>>,
    >,
);

#[derive(Flow)]
pub struct ExampleFlowIteration(
    MultiWorkerConditionalSegment,
    Transparent<MultiWorkerMetaJoin, MultiWorkerJoinSegment>,
    Transparent<MultiWorkerMetaTail, MultiWorkerTailSegment>,
);

pub struct MultiWorkerAnimal;

#[jungle::animal(id = 91, generation = 0)]
impl Animal for MultiWorkerAnimal {
    type State = MultiWorkerState;
    type Seed = MultiWorkerState;
    type Flow = ExampleFlow;
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
async fn local_client_multi_worker_example_flow_has_expected_events_without_replays() {
    let client = FusedClient::builder()
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

    let seed = MultiWorkerState::default();
    let mut journey_ids = Vec::with_capacity(JOURNEY_COUNT);
    for _ in 0..JOURNEY_COUNT {
        let journey_id = client
            .spawn::<MultiWorkerAnimal>(&seed)
            .await
            .expect("journey should start")
            .journey_id;
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

    assert!(
        journey_one_counts.started > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "first journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} started-step events"
    );
    assert!(
        journey_one_counts.succeeded > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "first journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} succeeded-step events"
    );
    assert_eq!(
        journey_one_counts.failed, 0,
        "first journey should not fail any step"
    );

    assert!(
        journey_two_counts.started > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "second journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} started-step events"
    );
    assert!(
        journey_two_counts.succeeded > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "second journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} succeeded-step events"
    );
    assert_eq!(
        journey_two_counts.failed, 0,
        "second journey should not fail any step"
    );
    assert_eq!(
        journey_one_counts.total_events, journey_two_counts.total_events,
        "both journeys should receive the same total event count"
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

#[tokio::test]
async fn local_client_single_worker_single_journey_example_flow_has_expected_events_without_replays(
) {
    let client = FusedClient::builder()
        .namespace("single-worker-regression")
        .build()
        .await
        .expect("local client should build");

    let mut worker_handles = Vec::with_capacity(SINGLE_WORKER_COUNT);
    for _ in 0..SINGLE_WORKER_COUNT {
        let worker = JungleWorker::new(MultiWorkerZoo, client.clone());
        worker_handles.push(tokio::spawn(async move {
            let _ = worker.spawn().await;
        }));
    }

    let seed = MultiWorkerState::default();
    let mut journey_ids = Vec::with_capacity(SINGLE_JOURNEY_COUNT);
    for _ in 0..SINGLE_JOURNEY_COUNT {
        let journey_id = client
            .spawn::<MultiWorkerAnimal>(&seed)
            .await
            .expect("journey should start")
            .journey_id;
        journey_ids.push(journey_id);
    }

    let journey_id = journey_ids[0];

    let mut journey_stream = client
        .subscribe_step_updates(journey_id, None)
        .await
        .expect("journey subscribe_step_updates should succeed");

    let journey_counts = tokio::time::timeout(Duration::from_secs(45), async {
        consume_journey_updates(&mut journey_stream, journey_id).await
    })
    .await
    .expect("journey update stream should complete before timeout");

    assert!(
        journey_counts.started > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} started-step events"
    );
    assert!(
        journey_counts.succeeded > MIN_EXPECTED_EVENTS_PER_JOURNEY,
        "journey should emit more than {MIN_EXPECTED_EVENTS_PER_JOURNEY} succeeded-step events"
    );
    assert_eq!(journey_counts.failed, 0, "journey should not fail any step");

    assert_eq!(
        client
            .journey_details(journey_id)
            .await
            .expect("journey_details should succeed"),
        JourneyStatus::Completed
    );

    for worker_handle in worker_handles {
        worker_handle.abort();
        let _ = worker_handle.await;
    }
}

struct JourneyCounts {
    total_events: usize,
    started: usize,
    succeeded: usize,
    failed: usize,
}

async fn consume_journey_updates(
    stream: &mut JourneyUpdateSubscription,
    journey_id: uuid::Uuid,
) -> JourneyCounts {
    let mut total_events = 0_usize;
    let mut started = 0_usize;
    let mut succeeded = 0_usize;
    let mut failed = 0_usize;
    let mut last_sequence_id = None;

    while let Some(next) = stream.next().await {
        let update = next.expect("streamed journey update should succeed");
        total_events += 1;
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
            RunnerUpdateOut::NodeLifecycle(node) => {
                assert_eq!(
                    node.uuid, journey_id,
                    "lifecycle must match subscribed journey"
                );
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
    }

    JourneyCounts {
        total_events,
        started,
        succeeded,
        failed,
    }
}
