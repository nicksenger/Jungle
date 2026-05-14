use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Action, ActionCompletion, Animal, AnimalMember, Animals as AnimalsTrait, Ecosystem, Id,
    Identity, Observe, Pulse, Step, SupportedAnimal,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U33, U70, U71};
use jungle_sdk::{Animals, Journey, JungleClient};
use std::net::SocketAddr;
use std::time::Duration;

struct LegacyAction;
impl jungle_sdk::types::ActionMember for LegacyAction {}

#[derive(Clone, Copy)]
struct LegacyDependency;

impl Action for LegacyAction {
    type Id = Id<U70>;
    type Dependency = LegacyDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(10))
    }
}

struct ModernAction;
impl jungle_sdk::types::ActionMember for ModernAction {}

#[derive(Clone, Copy)]
struct ModernDependency;

impl Action for ModernAction {
    type Id = Id<U71>;
    type Dependency = ModernDependency;
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(99))
    }
}

struct LegacyStep;
impl Pulse<LegacyAnimal> for LegacyStep {
    type Action = LegacyAction;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(_state: &i32, _input: Self::Arg) -> Self::Arg {}

    fn absorb(state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Ret {
        *state = output.expect("legacy step should succeed");
    }
}

struct ModernStep;
impl Pulse<ModernAnimal> for ModernStep {
    type Action = ModernAction;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = ();

    fn emit(_state: &i32, _input: Self::Arg) -> Self::Arg {}

    fn absorb(state: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Ret {
        *state = output.expect("modern step should succeed");
    }
}

#[derive(Journey)]
struct LegacyJourney(Step<LegacyAnimal, LegacyStep>);

#[derive(Journey)]
struct ModernJourney(Step<ModernAnimal, ModernStep>);

struct LegacyAnimal;
impl AnimalMember for LegacyAnimal {}
impl Animal for LegacyAnimal {
    type Id = Id<U33>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = LegacyJourney;
}
impl jungle_sdk::types::AnimalObservation for LegacyAnimal {
    type Bridge = jungle_sdk::types::ObserveObservation;
}
impl jungle_sdk::types::AnimalPerturbation for LegacyAnimal {
    type Bridge = jungle_sdk::types::NoopPerturbation;
}
impl Observe for LegacyAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl AnimalsTrait for LegacyAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U33, LegacyAnimal>;
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for LegacyAnimal {
    type Id = U33;
}

struct ModernAnimal;
impl AnimalMember for ModernAnimal {}
impl Animal for ModernAnimal {
    type Id = Id<U33>;
    type Generation = U1;
    type State = i32;
    type Seed = i32;
    type Journey = ModernJourney;
}
impl jungle_sdk::types::AnimalObservation for ModernAnimal {
    type Bridge = jungle_sdk::types::ObserveObservation;
}
impl jungle_sdk::types::AnimalPerturbation for ModernAnimal {
    type Bridge = jungle_sdk::types::NoopPerturbation;
}
impl Observe for ModernAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl AnimalsTrait for ModernAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U33, ModernAnimal>;
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for ModernAnimal {
    type Id = U33;
}

struct FutureAnimal;
impl AnimalMember for FutureAnimal {}
impl Animal for FutureAnimal {
    type Id = Id<U33>;
    type Generation = U2;
    type State = i32;
    type Seed = i32;
    type Journey = ModernJourney;
}
impl jungle_sdk::types::AnimalObservation for FutureAnimal {
    type Bridge = jungle_sdk::types::ObserveObservation;
}
impl jungle_sdk::types::AnimalPerturbation for FutureAnimal {
    type Bridge = jungle_sdk::types::NoopPerturbation;
}
impl Observe for FutureAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl AnimalsTrait for FutureAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U33, FutureAnimal>;
}
#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for FutureAnimal {
    type Id = U33;
}

#[derive(Animals)]
struct VersionedAnimals(LegacyAnimal, ModernAnimal);

struct VersionedZoo;
impl Ecosystem for VersionedZoo {
    const NAME: &'static str = "versioned-zoo";
    type Animals = VersionedAnimals;
}

impl From<&VersionedZoo> for LegacyDependency {
    fn from(_value: &VersionedZoo) -> Self {
        Self
    }
}

impl From<&VersionedZoo> for ModernDependency {
    fn from(_value: &VersionedZoo) -> Self {
        Self
    }
}

#[test]
fn generations_helper_collects_all_generations_for_animal_id() {
    type Expected = list![U1, U0];
    type Actual = jungle_sdk::types::Generations<VersionedZoo, Id<U33>>;
    assert_type_eq!(Actual, Expected);
}

#[test]
fn highest_generation_helper_picks_latest_for_animal_id() {
    type Expected = U1;
    type Actual = jungle_sdk::types::HighestGeneration<VersionedZoo, Id<U33>>;
    assert_type_eq!(Actual, Expected);
}

#[tokio::test]
async fn multiple_generations_share_id_but_dispatch_uses_latest_generation() {
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
    let supported = vec![
        SupportedAnimal {
            animal_id: 33,
            generation: 0,
        },
        SupportedAnimal {
            animal_id: 33,
            generation: 1,
        },
    ];

    assert!(
        client
            .poll_work(supported.clone())
            .await
            .expect("poll_work registration should succeed")
            .is_none(),
        "registration poll should not claim work yet"
    );

    let seed = postcard::to_allocvec(&0_i32).expect("seed should serialize");
    let journey_id = client
        .start_journey::<LegacyAnimal>(seed)
        .await
        .expect("start_journey legacy should succeed");

    let worker = JungleWorker::new(VersionedZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let completion = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == jungle_sdk::types::JourneyStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    if completion.is_err() {
        panic!("versioned journey did not complete before timeout");
    }

    if worker_handle.is_finished() {
        let joined = worker_handle.await;
        panic!("worker should continue polling, got: {joined:?}");
    }

    let appearance_bytes = client
        .animal_appearance(journey_id)
        .await
        .expect("animal_appearance should succeed")
        .expect("final appearance should be present");
    let value: i32 =
        postcard::from_bytes(&appearance_bytes).expect("appearance should deserialize");
    assert_eq!(
        value, 99,
        "latest generation journey behavior should execute"
    );

    worker_handle.abort();
    let _ = worker_handle.await;

    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn create_journey_fails_when_client_generation_exceeds_server_latest() {
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
    assert!(client
        .poll_work(vec![SupportedAnimal {
            animal_id: 33,
            generation: 1,
        }])
        .await
        .expect("poll_work registration should succeed")
        .is_none());

    let seed = postcard::to_allocvec(&0_i32).expect("seed should serialize");
    let err = client
        .start_journey::<FutureAnimal>(seed)
        .await
        .expect_err("start_journey should fail when client generation is ahead");
    let message = err.to_string();
    assert!(
        message.contains("client generation 2 exceeds latest server generation 1"),
        "unexpected error message: {message}"
    );

    server_task.abort();
    let _ = server_task.await;
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::client::Client<VersionedZoo> {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
            .ecosystem::<VersionedZoo>()
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
