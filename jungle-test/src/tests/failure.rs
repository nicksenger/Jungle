use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, JourneyStatus, JungleClient, LocalClient};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureState {
    steps_completed: u8,
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

#[derive(Flow)]
pub struct FailureJourney(Step<PassStep>, Step<FailStep>);

pub struct FailureAnimal;

#[jungle::animal(id = 97, generation = 0)]
impl Animal for FailureAnimal {
    type State = FailureState;
    type Seed = ();
    type Journey = FailureJourney;
}

#[derive(Animals)]
pub struct FailureAnimals(FailureAnimal);

pub struct FailureZoo;
impl Ecosystem for FailureZoo {
    const NAME: &'static str = "failure-zoo";
    type Animals = FailureAnimals;
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

    let final_status = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == JourneyStatus::Dead {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("journey should reach dead status before timeout");

    assert_eq!(final_status, JourneyStatus::Dead);

    worker_handle.abort();
    let _ = worker_handle.await;
}
