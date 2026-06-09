use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::time::{Millis, SleepFor};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Write;
use std::marker::PhantomData;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const DEFAULT_LOG_FILTER: &str = "warn,loops=info";

type LoopValue<T> = (u64, T);

pub struct Forever<State, Input>(PhantomData<fn() -> (State, Input)>);
impl<State, Input> Predicate<(&State, &Input)> for Forever<State, Input> {
    fn eval((_state, _input): &(&State, &Input)) -> bool {
        true
    }
}

pub struct PrintCounter;
#[jungle::effect(id = 1)]
impl<J> Effect<J> for PrintCounter {
    type In = u64;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        counter: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        async move {
            print!("{counter}\n");
            let _ = std::io::stdout().flush();
            Ok(())
        }
    }
}

pub struct Print<T>(PhantomData<fn() -> T>);
#[jungle::action(carry = LoopValue<T>)]
impl<T> Action for Print<T>
where
    T: Serialize + DeserializeOwned + Send + 'static,
{
    type Effect = PrintCounter;
    type Input = LoopValue<T>;
    type Output = LoopValue<T>;

    fn emit(_state: &(), input: Self::Input) -> (u64, LoopValue<T>) {
        (input.0, input)
    }

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
        carry: LoopValue<T>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::from)?;
        Ok(carry)
    }
}

pub struct Increment<T>(PhantomData<fn() -> T>);
#[jungle::action(carry = LoopValue<T>)]
impl<T> Action for Increment<T>
where
    T: Serialize + DeserializeOwned + Send + 'static,
{
    type Effect = NoEffect;
    type Input = LoopValue<T>;
    type Output = LoopValue<T>;

    fn emit(_state: &(), input: Self::Input) -> ((), LoopValue<T>) {
        ((), input)
    }

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
        carry: LoopValue<T>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("increment should complete without effect"))?;
        Ok((carry.0 + 1, carry.1))
    }
}

#[derive(Flow)]
struct LoopBody<T: Serialize + DeserializeOwned + Send + 'static>(
    Step<Print<T>>,
    Step<Increment<T>>,
    Step<SleepFor<(), Millis<500>, LoopValue<T>>>,
);

#[derive(Flow)]
struct LoopJourney<T: Serialize + DeserializeOwned + Send + 'static>(
    While<Forever<(), LoopValue<T>>, LoopBody<T>>,
);

struct LoopAnimal;
#[jungle::animal(id = 5, generation = 0)]
impl Animal for LoopAnimal {
    type State = ();
    type Seed = LoopValue<()>;
    type Flow = LoopJourney<()>;
}

#[derive(Animals)]
struct LoopAnimals(LoopAnimal);

struct LoopZoo;
impl Ecosystem for LoopZoo {
    const NAME: &'static str = "loop-zoo";
    type Animals = LoopAnimals;
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let db_path = std::env::temp_dir().join(format!("jungle-loop-{}.redb", Uuid::new_v4()));

    info!(db_path = %db_path.display(), "starting loop runtime");

    let backend = Server::builder().redb_path(&db_path).build().await?;
    let client = FusedClient::builder()
        .namespace(LoopZoo::NAME)
        .backend(backend)
        .build()
        .await?;

    let worker_client = client.clone();
    let worker_handle = tokio::spawn(async move {
        let worker = JungleWorker::new(LoopZoo, worker_client);
        if let Err(err) = worker.spawn().await {
            warn!(error = %err, "loop worker exited");
        }
    });

    let journey_id = client.spawn::<LoopAnimal>(&(0, ())).await?.journey_id;
    info!(%journey_id, db_path = %db_path.display(), "loop demo active");

    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c; shutting down loop worker");

    worker_handle.abort();
    let _ = worker_handle.await;

    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
    debug!("loop tracing initialized");
}
