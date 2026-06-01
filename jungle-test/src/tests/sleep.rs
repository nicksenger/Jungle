use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::{Animals, JungleClient, Optic};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepState {
    counter: i32,
    phase: u8,
    sleep_for_ms: u64,
}

pub struct AddEffect;
#[jungle::effect(id = 40)]
impl<J> Effect<J> for AddEffect {
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

pub struct SleepNotComplete;
impl Predicate<(&SleepState, &())> for SleepNotComplete {
    fn eval((state, _): &(&SleepState, &())) -> bool {
        state.phase < 3
    }
}

pub struct SleepPhaseZero;
impl Predicate<(SleepState, ())> for SleepPhaseZero {
    fn eval((state, _): &(SleepState, ())) -> bool {
        state.phase == 0
    }
}

pub struct SleepPhaseOne;
impl Predicate<(SleepState, ())> for SleepPhaseOne {
    fn eval((state, _): &(SleepState, ())) -> bool {
        state.phase == 1
    }
}

pub struct AddBeforeSleepSpec;
#[jungle::action]
impl Action for AddBeforeSleepSpec {
    type Effect = AddEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SleepState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut SleepState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            state.counter += output.map_err(|_err| Failure::from("add before sleep should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_1)
    }
}

pub struct SleepForStateWakeSpec;
#[jungle::action]
impl Action for SleepForStateWakeSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &SleepState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(
        state: &mut SleepState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            output.map_err(|_err| Failure::from("sleep should resume successfully"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_2)
    }
}

pub struct AddAfterSleepSpec;
#[jungle::action]
impl Action for AddAfterSleepSpec {
    type Effect = AddEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SleepState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut SleepState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            state.counter += output.map_err(|_err| Failure::from("add after sleep should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_3)
    }
}

pub struct MergeEitherUnitEffect;
#[jungle::effect(id = 41)]
impl<J> Effect<J> for MergeEitherUnitEffect {
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

pub struct MergeEitherUnitSpec;
#[jungle::action]
impl Action for MergeEitherUnitSpec {
    type Effect = MergeEitherUnitEffect;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &SleepState, _input: Self::Input) -> () {}

    fn absorb(
        _state: &mut SleepState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            output.map_err(|_err| Failure::from("merge either unit should succeed"))?;
        };
        Ok(__absorb_out_4)
    }
}

#[derive(Flow)]
pub struct SleepPhaseOneBranch(
    Conditional<SleepPhaseOne, Step<SleepForStateWakeSpec>, Step<AddAfterSleepSpec>>,
    Step<MergeEitherUnitSpec>,
);

#[derive(Flow)]
pub struct SleepLoopBody(
    Conditional<SleepPhaseZero, Step<AddBeforeSleepSpec>, SleepPhaseOneBranch>,
    Step<MergeEitherUnitSpec>,
);

#[derive(Flow)]
pub struct SleepJourneyTemplate(While<SleepNotComplete, SleepLoopBody>);

pub struct SleepAnimal;

#[jungle::animal(observe, id = 0, generation = 0)]
impl Animal for SleepAnimal {
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
pub struct SleepAnimals(SleepAnimal);

pub struct SleepZoo;
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
    let worker_client = connect_client_with_retry(listen_addr).await;
    let worker = JungleWorker::new(SleepZoo, worker_client);
    let worker_handle = tokio::spawn(async move { worker.spawn().await });

    let seed = SleepState {
        counter: 0,
        phase: 0,
        sleep_for_ms: 250,
    };
    let journey_id = client
        .spawn::<SleepAnimal>(&seed)
        .await
        .expect("spawn should succeed for sleep flow");

    let worker_exited_early = Arc::new(AtomicBool::new(false));
    let completion = tokio::time::timeout(Duration::from_secs(8), async {
        let worker_exited_early = Arc::clone(&worker_exited_early);
        loop {
            if worker_handle.is_finished() {
                worker_exited_early.store(true, Ordering::Relaxed);
                break;
            }

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
    if worker_exited_early.load(Ordering::Relaxed) {
        let joined = worker_handle.await;
        panic!("worker exited before journey completion: {joined:?}");
    }
    if completion.is_err() {
        panic!("sleep flow did not complete before timeout");
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
