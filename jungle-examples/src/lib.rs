extern crate jungle_sdk as inception;
extern crate jungle_sdk as jungle_types;

use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::types::{
    Action, ActionCompletion, Animal, AnimalMember, Condition, Conditional, Ecosystem, Id,
    Identity, Join, LoopCondition, Observe, Pulse, Select, Sleep, Step, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U10, U11, U12, U13, U14};
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
pub struct StaticState {
    pub value: i32,
}

pub struct AddOneAction;
impl jungle_sdk::types::ActionMember for AddOneAction {}
impl Action for AddOneAction {
    type Id = Id<U10>;
    type Dependency = ();
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(1))
    }
}

pub struct AddTwoAction;
impl jungle_sdk::types::ActionMember for AddTwoAction {}
impl Action for AddTwoAction {
    type Id = Id<U11>;
    type Dependency = ();
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(2))
    }
}

pub struct FastAction;
impl jungle_sdk::types::ActionMember for FastAction {}
impl Action for FastAction {
    type Id = Id<U12>;
    type Dependency = ();
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(9))
    }
}

pub struct SlowAction;
impl jungle_sdk::types::ActionMember for SlowAction {}
impl Action for SlowAction {
    type Id = Id<U13>;
    type Dependency = ();
    type In = ();
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(7))
    }
}

pub struct StaticAddOne;
impl Pulse<StaticAnimal> for StaticAddOne {
    type Action = AddOneAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = i32;

    fn emit(_state: &StaticState, _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(state: &mut StaticState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let value = output.expect("static add one should succeed");
        state.value += value;
        state.value
    }
}

pub struct StaticAddTwo;
impl Pulse<StaticAnimal> for StaticAddTwo {
    type Action = AddTwoAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = i32;

    fn emit(_state: &StaticState, _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(state: &mut StaticState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        let value = output.expect("static add two should succeed");
        state.value += value;
        state.value
    }
}

pub struct StaticFast;
impl Pulse<StaticAnimal> for StaticFast {
    type Action = FastAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = i32;

    fn emit(_state: &StaticState, _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(_state: &mut StaticState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        output.expect("static fast should succeed")
    }
}

pub struct StaticSlow;
impl Pulse<StaticAnimal> for StaticSlow {
    type Action = SlowAction;
    type Aspect = Identity;
    type CarryIn = ();
    type CarryOut = i32;

    fn emit(_state: &StaticState, _input: Self::CarryIn) -> Self::CarryIn {}

    fn absorb(_state: &mut StaticState, output: ActionCompletion<Self::Action>) -> Self::CarryOut {
        output.expect("static slow should succeed")
    }
}

pub struct StaticCondition;
impl Condition<(StaticState, ())> for StaticCondition {
    fn choose((state, _): &(StaticState, ())) -> bool {
        state.value % 2 == 0
    }
}

pub struct StaticLoopCondition;
impl LoopCondition<StaticState> for StaticLoopCondition {
    type CarryIn = ();

    fn should_continue(state: &StaticState) -> bool {
        state.value < 6
    }
}

pub struct StaticOuterCondition;
impl Condition<(StaticState, ())> for StaticOuterCondition {
    fn choose((state, _): &(StaticState, ())) -> bool {
        state.value % 3 == 0
    }
}

pub struct StaticOuterLoopCondition;
impl LoopCondition<StaticState> for StaticOuterLoopCondition {
    type CarryIn = ();

    fn should_continue(state: &StaticState) -> bool {
        state.value < 14
    }
}

type StaticInnerConditional = Conditional<
    StaticCondition,
    Step<StaticAnimal, StaticAddOne>,
    Step<StaticAnimal, StaticAddTwo>,
>;

type StaticInnerLoop = While<StaticLoopCondition, StaticInnerConditional>;

type StaticOuterConditional = Conditional<
    StaticOuterCondition,
    StaticInnerLoop,
    Step<StaticAnimal, StaticAddTwo>,
>;

type StaticOuterLoopFlow = While<StaticOuterLoopCondition, StaticOuterConditional>;

#[derive(Journey)]
pub struct StaticJourney(
    Step<StaticAnimal, StaticAddOne>,
    StaticOuterLoopFlow,
    Join<Step<StaticAnimal, StaticAddOne>, Step<StaticAnimal, StaticAddTwo>>,
    Select<Step<StaticAnimal, StaticFast>, Step<StaticAnimal, StaticSlow>>,
    Step<StaticAnimal, StaticAddTwo>,
);

pub struct StaticAnimal;
impl AnimalMember for StaticAnimal {}
impl Animal for StaticAnimal {
    type Id = Id<U0>;
    type Generation = U0;
    type State = StaticState;
    type Seed = StaticState;
    type Journey = StaticJourney;
}
impl jungle_sdk::types::AnimalObservation for StaticAnimal {
    type Adapter = jungle_sdk::types::NoopObservation;
}
impl jungle_sdk::types::AnimalPerturbation for StaticAnimal {
    type Adapter = jungle_sdk::types::NoopPerturbation;
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

#[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for ObserveAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U1, ObserveAnimal>;
}

#[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
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
    use super::*;
    use jungle_sdk::types::{JourneyAst, JourneyAstSource};

    #[test]
    fn static_journey_ast_shape_is_expanded_sequence() {
        let ast = <StaticJourney as JourneyAstSource>::journey_ast();
        let JourneyAst::Sequence(nodes) = ast else {
            panic!("expected StaticJourney AST to be a top-level sequence");
        };

        assert_eq!(nodes.len(), 5);
        assert!(matches!(nodes[0], JourneyAst::Step { .. }));
        assert!(matches!(nodes[1], JourneyAst::While { .. }));
        assert!(matches!(nodes[2], JourneyAst::Join { .. }));
        assert!(matches!(nodes[3], JourneyAst::Select { .. }));
        assert!(matches!(nodes[4], JourneyAst::Step { .. }));

        let JourneyAst::While { body, .. } = &nodes[1] else {
            panic!("expected second node to be an outer while");
        };
        let JourneyAst::Conditional { left, right, .. } = body.as_ref() else {
            panic!("expected outer while body to be a conditional");
        };
        assert!(matches!(left.as_ref(), JourneyAst::While { .. }));
        assert!(matches!(right.as_ref(), JourneyAst::Step { .. }));
    }
}
