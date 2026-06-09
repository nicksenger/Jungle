use std::marker::PhantomData;

use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[cfg(feature = "viewer")]
mod ui;

const DEFAULT_LOG_FILTER: &str = "warn,backoff=info";

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

struct BackoffAnimal;
#[jungle::animal(id = 211, generation = 0)]
impl Animal for BackoffAnimal {
    type State = ();
    type Seed = ();
    type Flow = jungle_zoo::backoff::Backoff<(), (), (), Step<Fail<()>>, 100u64, 10000u64, 2u8>;
}

#[derive(Animals)]
struct BackoffAnimals(BackoffAnimal);
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

    let journey_id = client.spawn::<BackoffAnimal>(&()).await?.journey_id;

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

#[cfg(test)]
mod tests {
    use super::*;
    use jungle_zoo::action_backoff::{BackoffLogKind, BackoffSleepLog};
    use tokio::time::{Duration, Instant};

    #[test]
    fn parses_non_negative_img_dump_time_secs() {
        assert_eq!(parse_img_dump_time_secs("0").unwrap(), 0.0);
        assert_eq!(parse_img_dump_time_secs("30.5").unwrap(), 30.5);
    }

    #[test]
    fn rejects_negative_img_dump_time_secs() {
        assert!(parse_img_dump_time_secs("-1").is_err());
    }

    #[test]
    fn join_arms_use_distinct_backoff_policies() {
        assert_ne!(subflow_backoff_policy(), action_backoff_policy());
        assert_ne!(
            subflow_backoff_policy().initial_delay_ms,
            action_backoff_policy().initial_delay_ms
        );
        assert_ne!(
            subflow_backoff_policy().max_delay_ms,
            action_backoff_policy().max_delay_ms
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn join_runs_both_backoff_arms() {
        let namespace = format!("{}-{}", BackoffZoo::NAME, Uuid::new_v4());
        let client = FusedClient::builder()
            .namespace(namespace)
            .build()
            .await
            .expect("local client should build");
        let worker = JungleWorker::new(BackoffZoo, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = ExponentialBackoffInput {
            action_input: (),
            policy: subflow_backoff_policy(),
        };

        let journey_id = client
            .spawn::<BackoffAnimal>(&seed)
            .await
            .expect("backoff journey should start")
            .journey_id;
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut saw_subflow_sleep = false;
        let mut saw_action_sleep = false;
        while Instant::now() < deadline {
            for event in client
                .journey_history(journey_id)
                .await
                .expect("journey_history should succeed while the demo is running")
            {
                let RunnerOut::EffectInput { data, .. } = event else {
                    continue;
                };
                let Ok(log) = postcard::from_bytes::<BackoffSleepLog>(&data) else {
                    continue;
                };
                match log.kind {
                    BackoffLogKind::Subflow => saw_subflow_sleep = true,
                    BackoffLogKind::Action => saw_action_sleep = true,
                }
            }
            if saw_subflow_sleep && saw_action_sleep {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        worker_handle.abort();

        assert!(saw_subflow_sleep, "expected subflow backoff arm activity");
        assert!(
            saw_action_sleep,
            "expected single-action backoff arm activity"
        );
    }
}
