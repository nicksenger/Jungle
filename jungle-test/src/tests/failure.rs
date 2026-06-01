use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, JourneyStatus, JungleClient, LocalClient};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureState {
    steps_completed: u8,
    attempt_failures: u8,
    attempt_successes: u8,
}

pub struct PassStep;
#[jungle::action]
impl Action for PassStep {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &FailureState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("pass step should complete"))?;
        state.steps_completed = state.steps_completed.saturating_add(1);
        Ok(())
    }
}

pub struct FailStep;
pub struct FailStepEffect;
#[jungle::effect(id = 520)]
impl<J> Effect<J> for FailStepEffect {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

#[jungle::action]
impl Action for FailStep {
    type Effect = FailStepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &FailureState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("intentional absorb failure"))
    }
}

pub struct ExpectAttemptErrStep;
pub struct AttemptOutcomeEffect;
#[jungle::effect(id = 521)]
impl<J> Effect<J> for AttemptOutcomeEffect {
    type In = Result<(), Failure>;
    type Out = Result<(), Failure>;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

#[jungle::action]
impl Action for ExpectAttemptErrStep {
    type Effect = AttemptOutcomeEffect;
    type Input = Result<(), Failure>;
    type Output = ();

    fn emit(_state: &FailureState, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let attempt_outcome = output?;
        match attempt_outcome {
            Ok(()) => Err(Failure::from("expected attempt to return Err")),
            Err(_failure) => {
                state.attempt_failures = state.attempt_failures.saturating_add(1);
                Ok(())
            }
        }
    }
}

pub struct ExpectAttemptOkStep;
#[jungle::action]
impl Action for ExpectAttemptOkStep {
    type Effect = AttemptOutcomeEffect;
    type Input = Result<(), Failure>;
    type Output = ();

    fn emit(_state: &FailureState, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let attempt_outcome = output?;
        match attempt_outcome {
            Ok(()) => {
                state.attempt_successes = state.attempt_successes.saturating_add(1);
                Ok(())
            }
            Err(_failure) => Err(Failure::from("expected attempt to return Ok")),
        }
    }
}

#[derive(Flow)]
pub struct FailureJourney(Step<PassStep>, Step<FailStep>);

pub struct FailureAnimal;

#[jungle::animal(id = 97, generation = 0)]
impl Animal for FailureAnimal {
    type State = FailureState;
    type Seed = ();
    type Journey = FailureJourney;
}

#[derive(Flow)]
pub struct AttemptFailureJourney(
    Step<PassStep>,
    Attempt<Step<FailStep>>,
    Step<ExpectAttemptErrStep>,
    Step<PassStep>,
);

pub struct AttemptFailureAnimal;

#[jungle::animal(id = 98, generation = 0)]
impl Animal for AttemptFailureAnimal {
    type State = FailureState;
    type Seed = ();
    type Journey = AttemptFailureJourney;
}

#[derive(Flow)]
pub struct AttemptSuccessJourney(
    Step<PassStep>,
    Attempt<Step<PassStep>>,
    Step<ExpectAttemptOkStep>,
    Step<PassStep>,
);

pub struct AttemptSuccessAnimal;

#[jungle::animal(id = 99, generation = 0)]
impl Animal for AttemptSuccessAnimal {
    type State = FailureState;
    type Seed = ();
    type Journey = AttemptSuccessJourney;
}

#[derive(Animals)]
pub struct FailureAnimals(FailureAnimal, AttemptFailureAnimal, AttemptSuccessAnimal);

pub struct FailureZoo;
impl Ecosystem for FailureZoo {
    const NAME: &'static str = "failure-zoo";
    type Animals = FailureAnimals;
}

async fn wait_for_status(
    client: &LocalClient,
    journey_id: uuid::Uuid,
    target: JourneyStatus,
) -> JourneyStatus {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == target {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("journey should reach expected status before timeout")
}

#[tokio::test]
async fn local_client_marks_journey_dead_when_absorb_returns_failure() {
    let client = LocalClient::builder()
        .namespace("absorb-failure-dead")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&()).expect("seed should serialize");
    let journey_id = client
        .start_journey::<FailureAnimal>(seed)
        .await
        .expect("journey should start");

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Dead).await;

    assert_eq!(final_status, JourneyStatus::Dead);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn attempt_catches_failure_and_journey_completes() {
    let client = LocalClient::builder()
        .namespace("attempt-catches-failure")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&()).expect("seed should serialize");
    let journey_id = client
        .start_journey::<AttemptFailureAnimal>(seed)
        .await
        .expect("journey should start");

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Completed).await;
    assert_eq!(final_status, JourneyStatus::Completed);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn attempt_wraps_success_and_journey_completes() {
    let client = LocalClient::builder()
        .namespace("attempt-wraps-success")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&()).expect("seed should serialize");
    let journey_id = client
        .start_journey::<AttemptSuccessAnimal>(seed)
        .await
        .expect("journey should start");

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Completed).await;
    assert_eq!(final_status, JourneyStatus::Completed);

    worker_handle.abort();
    let _ = worker_handle.await;
}
