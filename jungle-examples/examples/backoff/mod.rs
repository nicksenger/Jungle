use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::action_backoff::{ExponentialBackoffInput, ExponentialBackoffPolicy};
use jungle_zoo::subflow_backoff::ExponentialBackoffFlowState;
use jungle_zoo::SubflowBackoff;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[cfg(feature = "viewer")]
mod ui;

const DEFAULT_LOG_FILTER: &str = "warn,backoff=info";
const INNER_ATTEMPT_SLEEP_MS: u64 = 200;
const INITIAL_DELAY_MS: u64 = 250;
const MAX_DELAY_MS: u64 = 4_000;
const DELAY_MULTIPLIER: u32 = 2;

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffDemoState {
    started_subflows: u32,
    failed_subflows: u32,
    last_failure_message: Option<String>,
}

pub type BackoffState = ExponentialBackoffFlowState<BackoffDemoState, (), ()>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffAppearance {
    attempts: u32,
    next_delay_ms: u64,
    policy: ExponentialBackoffPolicy,
    last_result: Option<String>,
    demo: BackoffDemoState,
}

impl From<&BackoffState> for BackoffAppearance {
    fn from(state: &BackoffState) -> Self {
        Self {
            attempts: state.attempts,
            next_delay_ms: state.current_delay_ms,
            policy: state.policy,
            last_result: state.last_result.as_ref().map(|result| match result {
                Ok(()) => "unexpected success".to_owned(),
                Err(err) => err.to_string(),
            }),
            demo: state.st.clone(),
        }
    }
}

struct MarkAttemptStarted;
#[jungle::action]
impl Action for MarkAttemptStarted {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BackoffDemoState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut BackoffDemoState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("attempt marker should complete"))?;
        state.started_subflows = state.started_subflows.saturating_add(1);
        state.last_failure_message = None;
        Ok(())
    }
}

struct PauseInsideAttempt;
#[jungle::action]
impl Action for PauseInsideAttempt {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &BackoffDemoState, _input: Self::Input) -> Duration {
        Duration::from_millis(INNER_ATTEMPT_SLEEP_MS)
    }

    fn absorb(
        _state: &mut BackoffDemoState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        Ok(())
    }
}

struct FailAttempt;
#[jungle::action]
impl Action for FailAttempt {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BackoffDemoState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut BackoffDemoState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("failing step should complete"))?;
        state.failed_subflows = state.failed_subflows.saturating_add(1);
        let message = format!(
            "subflow attempt {} failed on purpose",
            state.started_subflows
        );
        state.last_failure_message = Some(message.clone());
        Err(Failure::from(message))
    }
}

#[derive(Flow)]
struct AlwaysFailingSubflow(
    Step<MarkAttemptStarted>,
    Step<PauseInsideAttempt>,
    Step<FailAttempt>,
);

#[derive(Flow)]
struct BackoffJourney(SubflowBackoff<BackoffDemoState, (), (), AlwaysFailingSubflow>);

struct BackoffAnimal;

#[jungle::animal(id = 211, generation = 0)]
impl Animal for BackoffAnimal {
    type State = BackoffState;
    type Seed = ExponentialBackoffInput<()>;
    type Flow = BackoffJourney;
}

impl Observe for BackoffAnimal {
    type Appearance = BackoffAppearance;

    fn observe(state: &Self::State) -> Self::Appearance {
        BackoffAppearance::from(state)
    }
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
        initial_delay_ms = INITIAL_DELAY_MS,
        multiplier = DELAY_MULTIPLIER,
        max_delay_ms = MAX_DELAY_MS,
        inner_attempt_sleep_ms = INNER_ATTEMPT_SLEEP_MS,
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

    let seed = ExponentialBackoffInput {
        action_input: (),
        policy: ExponentialBackoffPolicy {
            initial_delay_ms: INITIAL_DELAY_MS,
            multiplier: DELAY_MULTIPLIER,
            max_delay_ms: MAX_DELAY_MS,
        },
    };
    let journey_id = client.spawn::<BackoffAnimal>(&seed).await?.journey_id;

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

    #[test]
    fn parses_non_negative_img_dump_time_secs() {
        assert_eq!(parse_img_dump_time_secs("0").unwrap(), 0.0);
        assert_eq!(parse_img_dump_time_secs("30.5").unwrap(), 30.5);
    }

    #[test]
    fn rejects_negative_img_dump_time_secs() {
        assert!(parse_img_dump_time_secs("-1").is_err());
    }
}
