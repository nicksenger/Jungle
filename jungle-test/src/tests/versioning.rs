use jungle_sdk::animal;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::effect;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Act, Animal, BindAnimal, BoundAct, Ecosystem, EffectCompletion, Generations,
    HighestGeneration, Id, Identity, JourneyStatus, Observe, Step, SupportedAnimal,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U33, U70, U71};
use jungle_sdk::{Animals, JungleClient};
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::time::Duration;

struct LegacyEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for LegacyEffect {
    type Id = Id<U70>;
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(10))
    }
}

struct ModernEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for ModernEffect {
    type Id = Id<U71>;
    type In = ();
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(99))
    }
}

struct LegacyStepSpec;
impl Act for LegacyStepSpec {
    type Effect = LegacyEffect;
    type Input = i32;
    type Output = ();
    type Bind<A: Animal> = LegacyStep<A>;
}

struct LegacyStep<A>(PhantomData<fn() -> A>);
impl<A> BoundAct<A> for LegacyStep<A>
where
    A: Animal<State = i32>,
{
    type Effect = LegacyEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> () {}

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        *state = output.expect("legacy step should succeed");
    }
}

struct ModernStepSpec;
impl Act for ModernStepSpec {
    type Effect = ModernEffect;
    type Input = i32;
    type Output = ();
    type Bind<A: Animal> = ModernStep<A>;
}

struct ModernStep<A>(PhantomData<fn() -> A>);
impl<A> BoundAct<A> for ModernStep<A>
where
    A: Animal<State = i32>,
{
    type Effect = ModernEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> () {}

    fn absorb(state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        *state = output.expect("modern step should succeed");
    }
}

#[derive(jungle_sdk::Flow)]
struct LegacyFlowTemplate(Step<LegacyStepSpec>);

#[derive(jungle_sdk::Flow)]
struct ModernFlowTemplate(Step<ModernStepSpec>);

struct LegacyAnimal;
#[animal(observe)]
impl Animal for LegacyAnimal {
    type Id = Id<U33>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = <LegacyFlowTemplate as BindAnimal<LegacyAnimal>>::Bound;
}
impl Observe for LegacyAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

struct ModernAnimal;
#[animal(observe)]
impl Animal for ModernAnimal {
    type Id = Id<U33>;
    type Generation = U1;
    type State = i32;
    type Seed = i32;
    type Journey = <ModernFlowTemplate as BindAnimal<ModernAnimal>>::Bound;
}
impl Observe for ModernAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

struct FutureAnimal;
#[animal(observe)]
impl Animal for FutureAnimal {
    type Id = Id<U33>;
    type Generation = U2;
    type State = i32;
    type Seed = i32;
    type Journey = <ModernFlowTemplate as BindAnimal<FutureAnimal>>::Bound;
}
impl Observe for FutureAnimal {
    type Appearance = i32;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[derive(Animals)]
struct VersionedAnimals(LegacyAnimal, ModernAnimal);

struct VersionedZoo;
impl Ecosystem for VersionedZoo {
    const NAME: &'static str = "versioned-zoo";
    type Animals = VersionedAnimals;
}

#[test]
fn generations_helper_collects_all_generations_for_animal_id() {
    type Expected = list![U1, U0];
    type Actual = Generations<VersionedZoo, Id<U33>>;
    assert_type_eq!(Actual, Expected);
}

#[test]
fn highest_generation_helper_picks_latest_for_animal_id() {
    type Expected = U1;
    type Actual = HighestGeneration<VersionedZoo, Id<U33>>;
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
            if status == JourneyStatus::Completed {
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
