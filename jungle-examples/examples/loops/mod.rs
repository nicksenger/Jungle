use std::fmt::Display;
use std::io::Write;
use std::marker::PhantomData;

use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::join::Pass;
use jungle_zoo::loops::{Pred, WhileEnumerated};
use jungle_zoo::predicate::Always;
use jungle_zoo::time::{Millis, SleepFor};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const DEFAULT_LOG_FILTER: &str = "warn,loops=info";

pub struct Println<T>(PhantomData<T>);
#[jungle::effect(id = 1)]
impl<T, J> Effect<J> for Println<T>
where
    T: Serialize + DeserializeOwned + Send + Display + 'static,
{
    type In = T;
    type Out = T;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        async move {
            print!("{}\n", &input);
            let _ = std::io::stdout().flush();
            Ok(input)
        }
    }
}

pub struct Print<T>(PhantomData<T>);
#[jungle::action]
impl<T> Action for Print<T>
where
    T: Serialize + DeserializeOwned + Send + Display + 'static,
{
    type Effect = Println<T>;
    type Input = T;
    type Output = T;

    fn emit(_state: &(), input: Self::Input) -> T {
        input
    }

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(output?)
    }
}

#[derive(Flow)]
struct PrintFlow(Step<Print<u32>>);

#[derive(Flow)]
struct LoopBody(
    Join<PrintFlow, Pass<(), ()>>,
    Step<SleepFor<(), Millis<500>, (u32, ())>>,
);

#[derive(Flow)]
struct LoopJourney(WhileEnumerated<(), (), Always<(), ()>, LoopBody>);

struct LoopAnimal;
#[jungle::animal(id = 5, generation = 0)]
impl Animal for LoopAnimal {
    type State = ();
    type Seed = ();
    type Flow = LoopJourney;
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
    let db_path = std::env::temp_dir().join(format!("jungle-loop-{}.fjall", Uuid::new_v4()));

    info!(db_path = %db_path.display(), "starting loop runtime");

    let backend = Server::builder().fjall_path(&db_path).build().await?;
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

    let journey_id = client.spawn::<LoopAnimal>(&()).await?.journey_id;
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
