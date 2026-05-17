use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Act, Animal, Condition, Conditional, Ecosystem, EffectCompletion, EffectExec, EffectSchema, Id,
    Identity, LoopCondition, Observe, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U14};
use jungle_sdk::{Animals, JungleClient, Optic};
use serde::{Deserialize, Serialize};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Duration;
use uuid::Uuid;

pub fn reserve_local_addr() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
        .expect("should bind temporary udp socket for port reservation");
    socket
        .local_addr()
        .expect("temporary udp socket should expose local address")
}

pub async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return client,
            Err(err) if attempt < 39 => {
                let _ = err;
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(err) => panic!("failed to connect to server: {err}"),
        }
    }
    unreachable!("retry loop always returns or panics")
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct ObserveState {
    pub tick: u64,
    pub sleep_ms: u64,
}

pub struct BumpEffect;
impl EffectSchema for BumpEffect {
    type Id = Id<U14>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J> EffectExec<J> for BumpEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

pub struct ObserveSleep;
impl Act<ObserveAnimal> for ObserveSleep {
    type Effect = Sleep;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &ObserveState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_ms)
    }

    fn absorb(state: &mut ObserveState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("sleep branch should complete");
        state.tick = state.tick.saturating_add(1);
    }
}

pub struct ObserveBump;
impl Act<ObserveAnimal> for ObserveBump {
    type Effect = BumpEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &ObserveState, _input: Self::Input) -> Self::Input {}

    fn absorb(state: &mut ObserveState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("bump branch should complete");
        state.tick = state.tick.saturating_add(1);
    }
}

pub struct ObserveChooseSleep;
impl Condition<(ObserveState, ())> for ObserveChooseSleep {
    fn choose((state, _): &(ObserveState, ())) -> bool {
        state.tick % 2 == 0
    }
}

pub struct ObserveLoopForever;
impl LoopCondition<ObserveState> for ObserveLoopForever {
    type Arg = ();

    fn should_continue(_state: &ObserveState) -> bool {
        true
    }
}

type ObserveBody = Conditional<
    ObserveChooseSleep,
    Step<ObserveAnimal, ObserveSleep>,
    Step<ObserveAnimal, ObserveBump>,
>;

type ObserveJourney = While<ObserveLoopForever, ObserveBody>;

pub struct ObserveAnimal;
impl Animal for ObserveAnimal {
    type Id = Id<U1>;
    type Generation = U0;
    type State = ObserveState;
    type Seed = ObserveState;
    type Journey = ObserveJourney;
}
impl jungle_sdk::types::Observable for ObserveAnimal {
    type Observation = jungle_sdk::types::ObserveObservation;
}
impl jungle_sdk::types::Perturbable for ObserveAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
}
impl Observe for ObserveAnimal {
    type Appearance = ObserveState;

    fn observe(state: &Self::State) -> Self::Appearance {
        *state
    }
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for ObserveAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U1, ObserveAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for ObserveAnimal {
    type Id = U1;
}

#[derive(Animals)]
pub struct ObserveAnimals(ObserveAnimal);

pub struct ObserveEcosystem;
impl Ecosystem for ObserveEcosystem {
    const NAME: &'static str = "observe-ecosystem";
    type Animals = ObserveAnimals;
}

impl From<ObserveState> for () {
    fn from(_value: ObserveState) -> Self {}
}

fn spawn_server_runtime(listen_addr: SocketAddr, db_path: std::path::PathBuf) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("server runtime should start");
        runtime.block_on(async move {
            let _ = ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await;
        });
    });
}

pub fn spawn_observe_runtime() -> (jungle_sdk::Client, Uuid) {
    let listen_addr = reserve_local_addr();
    let db_path = std::env::temp_dir().join(format!("jungle-examples-{}.redb", Uuid::new_v4()));

    spawn_server_runtime(listen_addr, db_path);

    let setup_runtime = tokio::runtime::Runtime::new().expect("setup runtime should start");

    let client = setup_runtime.block_on(connect_client_with_retry(listen_addr));
    let worker_client = setup_runtime.block_on(connect_client_with_retry(listen_addr));

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("worker runtime should start");
        runtime.block_on(async move {
            let handle = tokio::spawn(async move {
                let worker = JungleWorker::new(ObserveEcosystem, worker_client);
                let _ = worker.spawn().await;
            });
            let _ = handle.await;
        });
    });

    let seed = postcard::to_allocvec(&ObserveState {
        tick: 0,
        sleep_ms: 1_200,
    })
    .expect("observe seed should serialize");

    let journey_id = setup_runtime
        .block_on(client.start_journey::<ObserveAnimal>(seed))
        .expect("start_journey observe animal should succeed");

    (client, journey_id)
}

fn spawn_gorilla_runtime_with_start<F>(start_journey: F) -> (jungle_sdk::Client, Uuid)
where
    F: FnOnce(&tokio::runtime::Runtime, &jungle_sdk::Client, Vec<u8>) -> Uuid,
{
    let listen_addr = reserve_local_addr();
    let db_path = std::env::temp_dir().join(format!("jungle-examples-{}.redb", Uuid::new_v4()));

    spawn_server_runtime(listen_addr, db_path);

    let setup_runtime = tokio::runtime::Runtime::new().expect("setup runtime should start");

    let client = setup_runtime.block_on(connect_client_with_retry(listen_addr));
    let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
        .expect("gorilla seed should serialize");
    let journey_id = start_journey(&setup_runtime, &client, seed);

    (client, journey_id)
}

pub fn spawn_gorilla_runtime_by_animal() -> (jungle_sdk::Client, Uuid) {
    spawn_gorilla_runtime_with_start(|runtime, client, seed| {
        runtime
            .block_on(client.start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed))
            .expect("start_journey gorilla should succeed")
    })
}
