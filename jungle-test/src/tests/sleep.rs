use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::{Animals, JungleClient, Optic, RunnerOut};
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
            state.counter +=
                output.map_err(|_err| Failure::from("add before sleep should succeed"))?;
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
            state.counter +=
                output.map_err(|_err| Failure::from("add after sleep should succeed"))?;
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
    type Flow = SleepJourneyTemplate;
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

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedJoinSleepLeftState {
    ran: bool,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedJoinSleepRightState {
    sleep_for_ms: u64,
    woke: bool,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedJoinSleepState {
    #[jungle(focus)]
    left: FocusedJoinSleepLeftState,
    #[jungle(focus)]
    right: FocusedJoinSleepRightState,
    tail_ran: bool,
}

pub struct FocusedJoinSleepLeftSpec;
#[jungle::action]
impl Action for FocusedJoinSleepLeftSpec {
    type Effect = AddEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &FocusedJoinSleepLeftState, _input: Self::Input) {}

    fn absorb(
        state: &mut FocusedJoinSleepLeftState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("focused join left add should succeed"))?;
        state.ran = true;
        Ok(())
    }
}

pub struct FocusedJoinSleepRightSpec;
#[jungle::action]
impl Action for FocusedJoinSleepRightSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &FocusedJoinSleepRightState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(
        state: &mut FocusedJoinSleepRightState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("focused join sleep should resume successfully"))?;
        state.woke = true;
        Ok(())
    }
}

pub struct FocusedJoinSleepTailSpec;
#[jungle::action]
impl Action for FocusedJoinSleepTailSpec {
    type Effect = MergeEitherUnitEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &FocusedJoinSleepState, _input: Self::Input) {}

    fn absorb(
        state: &mut FocusedJoinSleepState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("focused join tail should succeed"))?;
        state.tail_ran = true;
        Ok(())
    }
}

#[derive(Flow)]
#[jungle(focus = FocusedJoinSleepLeftState)]
pub struct FocusedJoinSleepLeftFlow(Step<FocusedJoinSleepLeftSpec>);

#[derive(Flow)]
#[jungle(focus = FocusedJoinSleepRightState)]
pub struct FocusedJoinSleepRightFlow(Step<FocusedJoinSleepRightSpec>);

#[derive(Flow)]
pub struct FocusedJoinSleepJourney(
    Join<FocusedJoinSleepLeftFlow, FocusedJoinSleepRightFlow>,
    Step<FocusedJoinSleepTailSpec>,
);

pub struct FocusedJoinSleepAnimal;

#[jungle::animal(observe, id = 1, generation = 0)]
impl Animal for FocusedJoinSleepAnimal {
    type State = FocusedJoinSleepState;
    type Seed = FocusedJoinSleepState;
    type Flow = FocusedJoinSleepJourney;
}

impl Observe for FocusedJoinSleepAnimal {
    type Appearance = FocusedJoinSleepState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
pub struct FocusedJoinSleepAnimals(FocusedJoinSleepAnimal);

pub struct FocusedJoinSleepZoo;
impl Ecosystem for FocusedJoinSleepZoo {
    const NAME: &'static str = "focused-join-sleep-zoo";
    type Animals = FocusedJoinSleepAnimals;
}

impl From<FocusedJoinSleepState> for () {
    fn from(_value: FocusedJoinSleepState) -> Self {}
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
        .expect("spawn should succeed for sleep flow")
        .journey_id;

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

    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed at completion");
    assert!(
        history
            .iter()
            .any(|event| matches!(event, RunnerOut::SleepScheduled { .. })),
        "root sleep should be scheduled through the backend"
    );
    assert!(
        history
            .iter()
            .any(|event| matches!(event, RunnerOut::SleepFired { .. })),
        "root sleep should resume from a backend wake event"
    );

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

#[tokio::test]
async fn focused_join_sleep_suspends_until_backend_wake() {
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
    let worker = JungleWorker::new(FocusedJoinSleepZoo, worker_client);
    let worker_handle = tokio::spawn(async move { worker.spawn().await });

    let seed = FocusedJoinSleepState {
        left: FocusedJoinSleepLeftState { ran: false },
        right: FocusedJoinSleepRightState {
            sleep_for_ms: 400,
            woke: false,
        },
        tail_ran: false,
    };
    let journey_id = client
        .spawn::<FocusedJoinSleepAnimal>(&seed)
        .await
        .expect("spawn should succeed for focused join sleep flow")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let history = client
                .journey_history(journey_id)
                .await
                .expect("journey_history should succeed");
            if history
                .iter()
                .any(|event| matches!(event, RunnerOut::SleepScheduled { .. }))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("focused join sleep should be scheduled through the backend");

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
        panic!("worker exited before focused join sleep completion: {joined:?}");
    }
    if completion.is_err() {
        panic!("focused join sleep flow did not complete before timeout");
    }

    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed at focused join completion");
    assert!(
        history
            .iter()
            .any(|event| matches!(event, RunnerOut::SleepScheduled { .. })),
        "focused join sleep should be scheduled through the backend"
    );
    assert!(
        history
            .iter()
            .any(|event| matches!(event, RunnerOut::SleepFired { .. })),
        "focused join sleep should resume from a backend wake event"
    );

    let final_appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed at completion")
        .expect("animal_appearance should be present at focused join completion");
    let final_appearance: FocusedJoinSleepState = postcard::from_bytes(&final_appearance_bytes)
        .expect("focused join final appearance should deserialize");
    assert!(final_appearance.left.ran);
    assert!(final_appearance.right.woke);
    assert!(final_appearance.tail_ran);

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
