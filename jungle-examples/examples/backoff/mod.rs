use std::fmt::Display;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::backoff::Println;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[cfg(feature = "viewer")]
mod ui;

const DEFAULT_LOG_FILTER: &str = "warn,backoff=info,iced_winit=error";

#[derive(Flow)]
pub struct FailingSubflow(Step<AnnounceFailure<(), ()>>, Step<Fail<()>>);

struct BackoffBeetle;
#[jungle::animal(id = 211, generation = 0)]
impl Animal for BackoffBeetle {
    type State = ();
    type Seed = ();
    type Flow = jungle_zoo::backoff::Backoff<(), (), (), FailingSubflow, 100u64, 10000u64, 2u8>;
}

#[derive(Animals)]
struct BackoffAnimals(BackoffBeetle);
struct BackoffZoo;
impl Ecosystem for BackoffZoo {
    const NAME: &'static str = "backoff-zoo";
    type Animals = BackoffAnimals;
}

#[derive(Debug, Parser)]
#[command(name = "backoff")]
struct Cli {
    #[arg(
        long = "img-dump",
        help = "Capture the backoff UI to this PNG path and then exit"
    )]
    img_dump: Option<PathBuf>,
    #[arg(
        long = "img-dump-time-secs",
        requires = "img_dump",
        value_parser = parse_img_dump_time_secs,
        help = "Seconds to wait after the UI starts before capturing --img-dump"
    )]
    img_dump_time_secs: Option<f64>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    let db_path = std::env::temp_dir().join(format!("jungle-backoff-{}.redb", Uuid::new_v4()));

    info!(
        db_path = %db_path.display(),
        "starting backoff runtime"
    );

    let backend = Server::builder().redb_path(&db_path).build().await?;
    let client = FusedClient::builder()
        .namespace(BackoffZoo::NAME)
        .backend(backend)
        .build()
        .await?;

    let worker_client = client.clone();
    let worker_handle = tokio::spawn(async move {
        let worker = JungleWorker::new(BackoffZoo, worker_client);
        if let Err(err) = worker.spawn().await {
            warn!(error = %err, "backoff worker exited");
        }
    });

    let journey_id = client.spawn::<BackoffBeetle>(&()).await?.journey_id;

    info!(
        %journey_id,
        db_path = %db_path.display(),
        "backoff demo active"
    );

    #[cfg(feature = "viewer")]
    {
        let image_dump = cli.img_dump.map(|output_path| {
            ui::ImageDumpConfig::new(
                output_path,
                Duration::from_secs_f64(cli.img_dump_time_secs.unwrap_or(0.0)),
            )
        });
        tokio::task::block_in_place(|| ui::run_ui(client.clone(), journey_id, image_dump))?;
    }

    #[cfg(not(feature = "viewer"))]
    if cli.img_dump.is_some() {
        warn!("--img-dump was ignored because backoff was built without the `viewer` feature");
    }
    #[cfg(not(feature = "viewer"))]
    tokio::signal::ctrl_c().await?;
    #[cfg(not(feature = "viewer"))]
    info!("received ctrl-c; shutting down backoff worker");
    #[cfg(feature = "viewer")]
    info!("backoff viewer closed; shutting down worker");

    worker_handle.abort();
    let _ = worker_handle.await;

    Ok(())
}

fn parse_img_dump_time_secs(value: &str) -> Result<f64, String> {
    let secs = value
        .parse::<f64>()
        .map_err(|err| format!("invalid img dump time `{value}`: {err}"))?;
    if secs.is_sign_negative() {
        return Err("img dump time must be non-negative".to_owned());
    }
    Ok(secs)
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
    debug!("backoff tracing initialized");
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fail<In>(PhantomData<In>);
#[jungle::action]
impl<In> Action for Fail<In> {
    type Effect = NoEffect;
    type Input = In;
    type Output = In;

    fn emit(_state: &(), _input: Self::Input) {}

    fn absorb(
        state: &mut (),
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::Message("Failed!".to_string()))
    }
}

pub struct AnnounceFailure<St, T>(PhantomData<St>, PhantomData<T>);
#[jungle::action(carry = T)]
impl<St, T> Action for AnnounceFailure<St, T> {
    type Effect = Println<String>;
    type Input = T;
    type Output = T;

    fn emit(_state: &St, input: Self::Input) -> (String, T) {
        ("Failing!".to_string(), input)
    }

    fn absorb(
        _state: &mut St,
        output: EffectCompletion<Self::Effect>,
        carry: T,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}
