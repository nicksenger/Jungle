use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Action, ActionCompletion, Animal, AnimalMember, Condition, Conditional, Ecosystem, Id,
    Identity, LoopCondition, Observe, Pulse, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U14};
use jungle_sdk::{Animals, Journey, JungleClient, Optic};
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

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObserveState {
    pub tick: u64,
    pub sleep_ms: u64,
}

pub struct BumpAction;
impl jungle_sdk::types::ActionMember for BumpAction {}
impl Action for BumpAction {
    type Id = Id<U14>;
    type Dependency = ();
    type In = ();
    type Out = ();
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

pub struct ObserveSleep;
impl Pulse<ObserveAnimal> for ObserveSleep {
    type Action = Sleep;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &ObserveState, _input: Self::CarryIn) -> Duration {
        Duration::from_millis(state.sleep_ms)
    }

    fn absorb(state: &mut ObserveState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        output.expect("sleep branch should complete");
        state.tick = state.tick.saturating_add(1);
    }
}

pub struct ObserveBump;
impl Pulse<ObserveAnimal> for ObserveBump {
    type Action = BumpAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(_state: &ObserveState, _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(state: &mut ObserveState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
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
    type CarryIn = ();

    fn should_continue(_state: &ObserveState) -> bool {
        true
    }
}

type ObserveBody = Conditional<
    ObserveChooseSleep,
    Step<ObserveAnimal, ObserveSleep>,
    Step<ObserveAnimal, ObserveBump>,
>;

#[derive(Journey)]
pub struct ObserveJourney(While<ObserveLoopForever, ObserveBody>);

pub struct ObserveAnimal;
impl AnimalMember for ObserveAnimal {}
impl Animal for ObserveAnimal {
    type Id = Id<U1>;
    type Generation = U0;
    type State = ObserveState;
    type Seed = ObserveState;
    type Journey = ObserveJourney;
}
impl jungle_sdk::types::AnimalObservation for ObserveAnimal {
    type Adapter = jungle_sdk::types::ObserveObservation;
}
impl jungle_sdk::types::AnimalPerturbation for ObserveAnimal {
    type Adapter = jungle_sdk::types::NoopPerturbation;
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

impl From<&ObserveEcosystem> for () {
    fn from(_value: &ObserveEcosystem) -> Self {}
}

pub fn spawn_observe_runtime() -> (jungle_sdk::Client, Uuid) {
    let listen_addr = reserve_local_addr();
    let db_path = std::env::temp_dir().join(format!("jungle-examples-{}.redb", Uuid::new_v4()));

    std::thread::spawn({
        let db_path = db_path.clone();
        move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("server runtime should start");
            runtime.block_on(async move {
                let _ = ServerBuilder::new()
                    .listen(listen_addr)
                    .redb_path(db_path)
                    .run()
                    .await;
            });
        }
    });

    let setup_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("setup runtime should start");

    let client = setup_runtime.block_on(connect_client_with_retry(listen_addr));
    let worker_client = setup_runtime.block_on(connect_client_with_retry(listen_addr));

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("worker runtime should start");
        runtime.block_on(async move {
            let worker = JungleWorker::new(ObserveEcosystem, worker_client);
            let _ = worker.spawn().await;
        });
    });

    let seed = postcard::to_allocvec(&ObserveState {
        tick: 0,
        sleep_ms: 1_200,
    })
    .expect("observe seed should serialize");

    let journey_id = setup_runtime
        .block_on(client.start_journey_for::<ObserveAnimal>(seed))
        .expect("start_journey_for observe animal should succeed");

    (client, journey_id)
}

#[cfg(test)]
mod tests {
    use jungle_sdk::types::{JourneyAst, JourneyAstSource};
    use jungle_zoo::animals::gorilla::GorillaJourney;

    #[test]
    fn gorilla_journey_ast_exposes_top_level_growth_loop() {
        let ast = <GorillaJourney as JourneyAstSource>::journey_ast();
        assert!(matches!(ast, JourneyAst::While { .. }));
    }
}
