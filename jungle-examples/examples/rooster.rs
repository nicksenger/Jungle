use clap::{Args, Parser, Subcommand};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Client, JungleClient};
use jungle_zoo::agent::{Agent, AgentInput, AgentModelConfig, AgentSettings, AgentState, Tool};
use jungle_zoo::predicate::Always;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
#[cfg(feature = "fjall")]
use std::path::Path;
#[cfg(feature = "fjall")]
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const DEFAULT_LOG_FILTER: &str = "warn,rooster=info";
const DEFAULT_SERVER_ADDR: &str = "[::1]:4433";
const DEFAULT_SERVER_NAME: &str = "localhost";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1";
const DEFAULT_CIRCADIAN_INTERVAL: &str = "1h";
const CIRCADIAN_PROMPT: &str =
    "You are a rooster. Use the 'Cockadoodledoo' and 'Cluck' tools to make sounds if you want.";

#[derive(Debug, Parser)]
#[command(name = "rooster")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start a long-lived Jungle server for rooster workers.
    Roost(RoostArgs),
    /// Start a rooster worker and spawn rooster + circadian journeys.
    Spawn(SpawnArgs),
}

#[derive(Debug, Args)]
struct RoostArgs {
    #[arg(long, default_value = DEFAULT_SERVER_ADDR)]
    listen: SocketAddr,
    #[cfg(feature = "postgres")]
    #[arg(long = "postgres-connection-string")]
    postgres_connection_string: Option<String>,
    #[cfg(feature = "fjall")]
    #[arg(long = "fjall-path")]
    fjall_path: Option<PathBuf>,
    #[cfg(feature = "fjall")]
    #[arg(long = "memory")]
    memory: bool,
}

#[derive(Debug, Args, Clone)]
struct SpawnArgs {
    #[arg(long = "roost-addr")]
    roost_addr: SocketAddr,
    #[arg(long, default_value = DEFAULT_SERVER_NAME)]
    server_name: String,
    #[arg(long = "openai-api-base-url")]
    openai_api_base_url: String,
    #[arg(long = "openai-model", default_value = DEFAULT_OPENAI_MODEL)]
    openai_model: String,
    #[arg(long = "openai-api-key")]
    openai_api_key: Option<String>,
    #[arg(
        long = "circadian-interval",
        default_value = DEFAULT_CIRCADIAN_INTERVAL,
        value_parser = parse_circadian_interval_secs
    )]
    circadian_interval_secs: u64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RoosterInnerState {}

type RoosterTools = list![CluckTool, CockadoodledooTool];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSeed {
    model_config: AgentModelConfig,
    settings: AgentSettings,
}

impl From<RoosterSeed> for AgentState<RoosterInnerState, RoosterTools> {
    fn from(seed: RoosterSeed) -> Self {
        AgentState::new(RoosterInnerState {}, seed.model_config).with_settings(seed.settings)
    }
}

pub struct Rooster;
#[jungle::animal(perturb, id = 34, generation = 0)]
impl Animal for Rooster {
    type State = AgentState<RoosterInnerState, RoosterTools>;
    type Seed = RoosterSeed;
    type Flow = RoosterFlow;
}
impl Perturb for Rooster {
    type Stimulus = AgentInput;

    fn perturb(state: &mut Self::State, stimulus: Self::Stimulus) {
        state.enqueue_input(stimulus);
    }
}

#[derive(Optic, Clone, Debug, Serialize, Deserialize)]
pub struct CircadianState {
    rooster_journey_id: Uuid,
    interval_secs: u64,
    roost_addr: SocketAddr,
    server_name: String,
}

impl Default for CircadianState {
    fn default() -> Self {
        Self {
            rooster_journey_id: Uuid::nil(),
            interval_secs: 60 * 60,
            roost_addr: DEFAULT_SERVER_ADDR
                .parse()
                .expect("default rooster server address is valid"),
            server_name: DEFAULT_SERVER_NAME.to_owned(),
        }
    }
}

pub struct Circadian;
#[jungle::animal(id = 35, generation = 0)]
impl Animal for Circadian {
    type State = CircadianState;
    type Seed = CircadianState;
    type Flow = CircadianFlow;
}

#[derive(Animals)]
pub struct RoosterAnimals(Rooster, Circadian);

#[derive(Clone, Copy, Debug, Default)]
pub struct RoosterEcosystem;

impl Ecosystem for RoosterEcosystem {
    const NAME: &'static str = "rooster-ecosystem";
    type Animals = RoosterAnimals;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSoundInput {
    amplitude: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoosterSoundOutput {
    sound: String,
    amplitude: u8,
}

pub struct CluckEffect;
#[jungle::effect(id = 700)]
impl<J> Effect<J> for CluckEffect {
    type In = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            println!("A rooster clucked.");
            Ok(RoosterSoundOutput {
                sound: "cluck".to_owned(),
                amplitude: input.amplitude,
            })
        }
    }
}

pub struct CockadoodledooEffect;
#[jungle::effect(id = 701)]
impl<J> Effect<J> for CockadoodledooEffect {
    type In = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            println!("A rooster cock-a-doodle-dooed.");
            Ok(RoosterSoundOutput {
                sound: "cockadoodledoo".to_owned(),
                amplitude: input.amplitude,
            })
        }
    }
}

pub struct CluckTool;
impl Tool for CluckTool {
    const NAME: &'static str = "Cluck";
    type Effect = CluckEffect;
    type Args = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn description() -> &'static str {
        "Make a short cluck sound from the rooster."
    }

    fn parameters_schema_json() -> &'static str {
        r#"{
            "type":"object",
            "properties":{
                "amplitude":{"type":"integer","minimum":0,"maximum":255}
            },
            "required":["amplitude"],
            "additionalProperties":false
        }"#
    }
}

pub struct CockadoodledooTool;
impl Tool for CockadoodledooTool {
    const NAME: &'static str = "Cockadoodledoo";
    type Effect = CockadoodledooEffect;
    type Args = RoosterSoundInput;
    type Out = RoosterSoundOutput;
    type Err = String;

    fn description() -> &'static str {
        "Make a loud cock-a-doodle-doo rooster call."
    }

    fn parameters_schema_json() -> &'static str {
        r#"{
            "type":"object",
            "properties":{
                "amplitude":{"type":"integer","minimum":0,"maximum":255}
            },
            "required":["amplitude"],
            "additionalProperties":false
        }"#
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerturbRoosterInput {
    rooster_journey_id: Uuid,
    roost_addr: SocketAddr,
    server_name: String,
    prompt: String,
}

pub struct PerturbRoosterEffect;
#[jungle::effect(id = 702)]
impl<J> Effect<J> for PerturbRoosterEffect {
    type In = PerturbRoosterInput;
    type Out = ();
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            let client = Client::builder()
                .namespace(RoosterEcosystem::NAME)
                .remote(input.roost_addr)
                .server_name(input.server_name)
                .build()
                .await
                .map_err(|err| format!("failed to connect circadian perturb client: {err}"))?;
            let perturbation = AgentInput {
                prompt: input.prompt,
            };
            let payload = postcard::to_allocvec(&perturbation)
                .map_err(|err| format!("failed to encode rooster perturbation: {err}"))?;
            client
                .perturb_animal(input.rooster_journey_id, payload)
                .await
                .map_err(|err| format!("failed to perturb rooster journey: {err}"))?;
            Ok(())
        }
    }
}

pub struct CircadianSleep;
#[jungle::action]
impl Action for CircadianSleep {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &CircadianState, _input: Self::Input) -> Duration {
        Duration::from_secs(state.interval_secs.max(1))
    }

    fn absorb(
        _state: &mut CircadianState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        Ok(())
    }
}

pub struct CircadianPerturbRooster;
#[jungle::action]
impl Action for CircadianPerturbRooster {
    type Effect = PerturbRoosterEffect;
    type Input = ();
    type Output = ();

    fn emit(state: &CircadianState, _input: Self::Input) -> PerturbRoosterInput {
        PerturbRoosterInput {
            rooster_journey_id: state.rooster_journey_id,
            roost_addr: state.roost_addr,
            server_name: state.server_name.clone(),
            prompt: CIRCADIAN_PROMPT.to_owned(),
        }
    }

    fn absorb(
        _state: &mut CircadianState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(Failure::Message)?;
        Ok(())
    }
}

pub struct SeedState<Seed, State>(std::marker::PhantomData<fn() -> (Seed, State)>);
#[jungle::action(carry = Seed)]
impl<Seed, State> Action for SeedState<Seed, State>
where
    Seed: Into<State>,
{
    type Effect = NoEffect;
    type Input = Seed;
    type Output = ();

    fn emit(_state: &State, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Seed) {
        ((), input)
    }

    fn absorb(
        state: &mut State,
        _output: EffectCompletion<Self::Effect>,
        seed: Seed,
    ) -> Result<Self::Output, Failure> {
        *state = seed.into();
        Ok(())
    }
}

#[derive(Flow)]
pub struct RoosterFlow(
    Step<SeedState<RoosterSeed, AgentState<RoosterInnerState, RoosterTools>>>,
    Agent<RoosterInnerState, RoosterTools>,
);

#[derive(Flow)]
pub struct CircadianBody(Step<CircadianSleep>, Step<CircadianPerturbRooster>);

#[derive(Flow)]
pub struct CircadianFlow(
    Step<SeedState<CircadianState, CircadianState>>,
    While<Always<CircadianState, ()>, CircadianBody>,
);

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Roost(args) => run_roost(args).await?,
        Command::Spawn(args) => run_spawn(args).await?,
    }
    Ok(())
}

async fn run_roost(args: RoostArgs) -> Result<(), Box<dyn std::error::Error>> {
    info!(listen = %args.listen, "starting rooster roost server");
    #[allow(unused_mut)]
    let mut builder = jungle_sdk::server::ServerBuilder::new().listen(args.listen);

    #[cfg(feature = "postgres")]
    if let Some(connection_string) = args.postgres_connection_string {
        builder = builder.postgres_connection_string(connection_string);
    }

    #[cfg(feature = "fjall")]
    if let Some(path) = args.fjall_path {
        ensure_parent_dir_exists(&path)?;
        builder = builder.fjall_path(path);
    }

    #[cfg(feature = "fjall")]
    if args.memory {
        builder = builder.memory();
    }

    builder.run().await?;
    Ok(())
}

async fn run_spawn(args: SpawnArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client_with_retry(&args).await?;
    let worker_client = client.clone();
    let worker_ecosystem = RoosterEcosystem;
    let worker_handle = tokio::spawn(async move {
        let worker = JungleWorker::new(worker_ecosystem, worker_client);
        if let Err(err) = worker.spawn().await {
            warn!(error = %err, "rooster worker exited");
        }
    });

    let openai_api_key = args
        .openai_api_key
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());
    let rooster_seed = RoosterSeed {
        model_config: AgentModelConfig {
            base_url: args.openai_api_base_url.clone(),
            model: args.openai_model.clone(),
            bearer_token: openai_api_key,
        },
        settings: AgentSettings::default(),
    };
    let rooster_journey_id = client.spawn::<Rooster>(&rooster_seed).await?.journey_id;

    let circadian_seed = CircadianState {
        rooster_journey_id,
        interval_secs: args.circadian_interval_secs,
        roost_addr: args.roost_addr,
        server_name: args.server_name.clone(),
    };
    let circadian_journey_id = client.spawn::<Circadian>(&circadian_seed).await?.journey_id;

    info!(
        %rooster_journey_id,
        %circadian_journey_id,
        circadian_interval_secs = args.circadian_interval_secs,
        "rooster spawn active"
    );
    println!("spawned rooster journey: {rooster_journey_id}");
    println!("spawned circadian journey: {circadian_journey_id}");
    println!("press ctrl-c to stop this worker");

    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c; shutting down rooster worker");
    worker_handle.abort();
    let _ = worker_handle.await;
    Ok(())
}

async fn connect_client_with_retry(args: &SpawnArgs) -> Result<Client, Box<dyn std::error::Error>> {
    let mut attempts = 0_u32;
    loop {
        match Client::builder()
            .namespace(RoosterEcosystem::NAME)
            .remote(args.roost_addr)
            .server_name(args.server_name.clone())
            .build()
            .await
        {
            Ok(client) => return Ok(client),
            Err(err) => {
                attempts = attempts.saturating_add(1);
                if attempts >= 50 {
                    return Err(Box::new(err));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

fn parse_circadian_interval_secs(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return Err(format!(
            "invalid interval `{value}`; use values like `1s`, `5m`, or `12h`"
        ));
    }

    let unit = trimmed
        .chars()
        .last()
        .expect("validated minimum interval length");
    let amount_str = &trimmed[..trimmed.len() - unit.len_utf8()];
    let amount = amount_str
        .parse::<u64>()
        .map_err(|_| format!("invalid interval amount in `{value}`"))?;
    if amount == 0 {
        return Err("circadian interval must be greater than zero".to_owned());
    }

    let secs = match unit.to_ascii_lowercase() {
        's' => amount,
        'm' => amount
            .checked_mul(60)
            .ok_or_else(|| format!("interval `{value}` overflowed seconds range"))?,
        'h' => amount
            .checked_mul(60)
            .and_then(|mins| mins.checked_mul(60))
            .ok_or_else(|| format!("interval `{value}` overflowed seconds range"))?,
        _ => {
            return Err(format!(
                "invalid interval unit in `{value}`; use `s`, `m`, or `h`"
            ));
        }
    };
    Ok(secs)
}

#[cfg(feature = "fjall")]
fn ensure_parent_dir_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
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
    debug!("rooster tracing initialized");
}

#[cfg(test)]
mod tests {
    use super::parse_circadian_interval_secs;

    #[test]
    fn parses_supported_circadian_units() {
        assert_eq!(parse_circadian_interval_secs("1s").unwrap(), 1);
        assert_eq!(parse_circadian_interval_secs("5m").unwrap(), 300);
        assert_eq!(parse_circadian_interval_secs("12h").unwrap(), 43_200);
    }

    #[test]
    fn rejects_invalid_circadian_units() {
        assert!(parse_circadian_interval_secs("9d").is_err());
        assert!(parse_circadian_interval_secs("0m").is_err());
        assert!(parse_circadian_interval_secs("abc").is_err());
    }
}
