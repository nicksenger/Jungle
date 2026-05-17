use jungle_sdk::animal;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::Animal;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    Act, BoundAct, Condition, Conditional, Ecosystem, EffectCompletion, EffectExec, EffectSchema,
    Identity, JourneyStatus, LoopCondition, Observe, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::consts::*;
use jungle_sdk::{Animals, JungleClient, Optic};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SleepState {
    counter: i32,
    phase: u8,
    sleep_for_ms: u64,
}

struct AddEffect;
impl EffectSchema for AddEffect {
    type Id = Id<U40>;
    type In = ();
    type Out = i32;
    type Err = ();
}

impl<J> EffectExec<J> for AddEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

struct AddBeforeSleep;
impl BoundAct<SleepAnimal> for AddBeforeSleep {
    type Effect = AddEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &SleepState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut SleepState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.counter += output.expect("add before sleep should succeed");
        state.phase += 1;
    }
}

struct SleepForStateWake;
impl BoundAct<SleepAnimal> for SleepForStateWake {
    type Effect = Sleep;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &SleepState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(state: &mut SleepState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("sleep should resume successfully");
        state.phase += 1;
    }
}

struct AddAfterSleep;
impl BoundAct<SleepAnimal> for AddAfterSleep {
    type Effect = AddEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &SleepState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut SleepState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        state.counter += output.expect("add after sleep should succeed");
        state.phase += 1;
    }
}

struct SleepNotComplete;
impl LoopCondition<SleepState> for SleepNotComplete {
    type Arg = ();

    fn should_continue(state: &SleepState) -> bool {
        state.phase < 3
    }
}

struct SleepPhaseZero;
impl Condition<(SleepState, ())> for SleepPhaseZero {
    fn choose((state, _): &(SleepState, ())) -> bool {
        state.phase == 0
    }
}

struct SleepPhaseOne;
impl Condition<(SleepState, ())> for SleepPhaseOne {
    fn choose((state, _): &(SleepState, ())) -> bool {
        state.phase == 1
    }
}

struct AddBeforeSleepSpec;
impl Act for AddBeforeSleepSpec {
    type Effect = AddEffect;
    type Input = ();
    type Output = ();
    type Bind<A: Animal> = AddBeforeSleep;
}

struct SleepForStateWakeSpec;
impl Act for SleepForStateWakeSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();
    type Bind<A: Animal> = SleepForStateWake;
}

struct AddAfterSleepSpec;
impl Act for AddAfterSleepSpec {
    type Effect = AddEffect;
    type Input = ();
    type Output = ();
    type Bind<A: Animal> = AddAfterSleep;
}

#[derive(jungle_sdk::Flow)]
struct SleepJourneyTemplate(
    While<
        SleepNotComplete,
        Conditional<
            SleepPhaseZero,
            Step<AddBeforeSleepSpec>,
            Conditional<SleepPhaseOne, Step<SleepForStateWakeSpec>, Step<AddAfterSleepSpec>>,
        >,
    >,
);

struct SleepAnimal;

#[animal(observe)]
impl Animal for SleepAnimal {
    type Id = Id<U0>;
    type Generation = U0;
    type State = SleepState;
    type Seed = SleepState;
    type Journey = SleepJourneyTemplate;
}

impl Observe for SleepAnimal {
    type Appearance = SleepState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct SleepAnimals(SleepAnimal);

struct SleepZoo;
impl Ecosystem for SleepZoo {
    const NAME: &'static str = "sleep-zoo";
    type Animals = SleepAnimals;
}

impl From<SleepState> for () {
    fn from(_value: SleepState) -> Self {}
}

#[tokio::test]
async fn sleep_effect_suspends_then_resumes_flow_to_completion() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let client = connect_client_with_retry(listen_addr).await;
    let worker = JungleWorker::new(SleepZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&SleepState {
        counter: 0,
        phase: 0,
        sleep_for_ms: 250,
    })
    .expect("sleep test seed should serialize");
    let journey_id = client
        .start_journey::<SleepAnimal>(seed)
        .await
        .expect("start_journey should succeed for sleep flow");

    let completion = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == JourneyStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    if completion.is_err() {
        panic!("sleep flow did not complete before timeout");
    }

    if worker_handle.is_finished() {
        let joined = worker_handle.await;
        panic!("worker should continue polling, got: {joined:?}");
    }

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed")
        .expect("animal_appearance should be present at completion");
    let appearance: SleepState =
        postcard::from_bytes(&appearance_bytes).expect("sleep appearance should deserialize");
    assert_eq!(appearance.counter, 2);

    worker_handle.abort();
    let _ = worker_handle.await;

    server_task.abort();
    let _ = server_task.await;
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return client,
            Err(err) if attempt < 39 => {
                std::thread::sleep(Duration::from_millis(25));
                let _ = err;
            }
            Err(err) => panic!("failed to connect to test server: {err}"),
        }
    }
    unreachable!("retry loop always returns or panics")
}
