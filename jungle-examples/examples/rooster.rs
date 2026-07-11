use clap::{Args, Parser, Subcommand};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::Client;
use jungle_zoo::agent::{Agent, AgentInput, AgentModelConfig, AgentSettings, AgentState, Tool};
use jungle_zoo::backoff::Backoff;
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
const CONNECT_RETRY_ATTEMPTS: u32 = 20;
const CONNECT_RETRY_DELAY_MS: u64 = 100;
const CONNECT_TIMEOUT_MS: u64 = 250;
const PERTURB_CONNECT_TIMEOUT_MS: u64 = 2_000;
const PERTURB_REQUEST_TIMEOUT_MS: u64 = 2_000;
const CIRCADIAN_PERTURB_BACKOFF_INITIAL_DELAY_MS: u64 = 250;
const CIRCADIAN_PERTURB_BACKOFF_MULTIPLIER: u8 = 2;
const CIRCADIAN_PERTURB_BACKOFF_MAX_DELAY_MS: u64 = 10_000;
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
            let client = tokio::time::timeout(
                Duration::from_millis(PERTURB_CONNECT_TIMEOUT_MS),
                Client::builder()
                    .namespace(RoosterEcosystem::NAME)
                    .remote(input.roost_addr)
                    .server_name(input.server_name)
                    .build(),
            )
            .await
            .map_err(|_| {
                format!(
                    "timed out connecting circadian perturb client after {}ms",
                    PERTURB_CONNECT_TIMEOUT_MS
                )
            })?
            .map_err(|err| format!("failed to connect circadian perturb client: {err}"))?;
            let perturbation = AgentInput {
                prompt: input.prompt,
            };
            let payload = postcard::to_allocvec(&perturbation)
                .map_err(|err| format!("failed to encode rooster perturbation: {err}"))?;
            tokio::time::timeout(
                Duration::from_millis(PERTURB_REQUEST_TIMEOUT_MS),
                client.perturb_animal(input.rooster_journey_id, payload),
            )
            .await
            .map_err(|_| {
                format!(
                    "timed out perturbing rooster journey after {}ms",
                    PERTURB_REQUEST_TIMEOUT_MS
                )
            })?
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

pub struct PrepareCircadianPerturbBackoffInput;
#[jungle::action]
impl Action for PrepareCircadianPerturbBackoffInput {
    type Effect = NoEffect;
    type Input = ();
    type Output = PerturbRoosterInput;
    type Carry = PerturbRoosterInput;

    fn emit(state: &CircadianState, _input: Self::Input) -> ((), PerturbRoosterInput) {
        (
            (),
            PerturbRoosterInput {
                rooster_journey_id: state.rooster_journey_id,
                roost_addr: state.roost_addr,
                server_name: state.server_name.clone(),
                prompt: CIRCADIAN_PROMPT.to_owned(),
            },
        )
    }

    fn absorb(
        _state: &mut CircadianState,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct CircadianPerturbRoosterAttempt;
#[jungle::action]
impl Action for CircadianPerturbRoosterAttempt {
    type Effect = PerturbRoosterEffect;
    type Input = PerturbRoosterInput;
    type Output = ();

    fn emit(_state: &CircadianState, input: Self::Input) -> PerturbRoosterInput {
        input
    }

    fn absorb(
        _state: &mut CircadianState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        if let Err(err) = output {
            warn!(error = %err, "circadian perturb attempt failed; retrying with backoff");
            return Err(Failure::Message(err));
        }
        Ok(())
    }
}

pub struct ExtractCircadianPerturbBackoffResult;
#[jungle::action(carry = (u32, (PerturbRoosterInput, Result<(), Failure>)))]
impl Action for ExtractCircadianPerturbBackoffResult {
    type Effect = NoEffect;
    type Input = (u32, (PerturbRoosterInput, Result<(), Failure>));
    type Output = ();

    fn emit(
        _state: &CircadianState,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut CircadianState,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        carry.1 .1
    }
}

#[derive(Flow)]
pub struct CircadianPerturbBackoff(
    Step<PrepareCircadianPerturbBackoffInput>,
    Backoff<
        CircadianState,
        PerturbRoosterInput,
        (),
        Step<CircadianPerturbRoosterAttempt>,
        CIRCADIAN_PERTURB_BACKOFF_INITIAL_DELAY_MS,
        CIRCADIAN_PERTURB_BACKOFF_MAX_DELAY_MS,
        CIRCADIAN_PERTURB_BACKOFF_MULTIPLIER,
    >,
    Step<ExtractCircadianPerturbBackoffResult>,
);

#[derive(Flow)]
pub struct CircadianBody(CircadianPerturbBackoff, Step<CircadianSleep>);

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
pub struct CircadianFlow(
    Step<SeedState<CircadianState, CircadianState>>,
    While<Always<CircadianState, ()>, CircadianBody>,
);

#[cfg(feature = "viewer")]
mod vision_ui {
    use super::{Circadian, Rooster};
    use iced::widget::{column, container, row, text};
    use iced::{Element, Font, Length, Subscription, Task};
    use uuid::Uuid;

    const WINDOW_WIDTH: f32 = 1600.0;
    const WINDOW_HEIGHT: f32 = 920.0;

    #[derive(Debug, Clone, Copy)]
    enum Panel {
        Rooster,
        Circadian,
    }

    #[derive(Debug, Clone)]
    enum Message {
        Viewer(Panel, jungle_vision::EjectedViewerMessage),
    }

    struct RoosterVisionUi {
        rooster_journey_id: Uuid,
        circadian_journey_id: Uuid,
        rooster_viewer:
            jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
        circadian_viewer:
            jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
    }

    impl RoosterVisionUi {
        fn new<C>(
            client: C,
            rooster_journey_id: Uuid,
            circadian_journey_id: Uuid,
        ) -> (Self, Task<Message>)
        where
            C: jungle_sdk::JungleClient + Clone + 'static,
        {
            let rooster_viewer = jungle_vision::JungleViewerBuilder::new()
                .title("Rooster Agent Flow")
                .eject_live_animal::<Rooster, _>(client.clone(), rooster_journey_id);
            let circadian_viewer = jungle_vision::JungleViewerBuilder::new()
                .title("Circadian Flow")
                .eject_live_animal::<Circadian, _>(client, circadian_journey_id);
            (
                Self {
                    rooster_journey_id,
                    circadian_journey_id,
                    rooster_viewer,
                    circadian_viewer,
                },
                Task::none(),
            )
        }

        fn update(&mut self, message: Message) -> Task<Message> {
            match message {
                Message::Viewer(panel, event) => match panel {
                    Panel::Rooster => self
                        .rooster_viewer
                        .update(event)
                        .map(|next| Message::Viewer(Panel::Rooster, next)),
                    Panel::Circadian => self
                        .circadian_viewer
                        .update(event)
                        .map(|next| Message::Viewer(Panel::Circadian, next)),
                },
            }
        }

        fn subscription(&self) -> Subscription<Message> {
            Subscription::batch([
                self.rooster_viewer
                    .subscription()
                    .map(|event| Message::Viewer(Panel::Rooster, event)),
                self.circadian_viewer
                    .subscription()
                    .map(|event| Message::Viewer(Panel::Circadian, event)),
            ])
        }

        fn view(&self) -> Element<'_, Message> {
            row![
                self.panel(
                    "Rooster",
                    self.rooster_journey_id,
                    self.rooster_viewer
                        .view()
                        .map(|event| Message::Viewer(Panel::Rooster, event)),
                ),
                self.panel(
                    "Circadian",
                    self.circadian_journey_id,
                    self.circadian_viewer
                        .view()
                        .map(|event| Message::Viewer(Panel::Circadian, event)),
                ),
            ]
            .spacing(12)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }

        fn panel<'a>(
            &'a self,
            title: &'a str,
            journey_id: Uuid,
            viewer: Element<'a, Message>,
        ) -> Element<'a, Message> {
            let journey = journey_id.to_string();
            container(
                column![
                    text(title).size(24),
                    text(format!("Journey {}", &journey[..8])).size(14),
                    container(viewer)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(8),
                ]
                .spacing(8),
            )
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .padding(8)
            .into()
        }
    }

    pub fn run_ui<C>(
        client: C,
        rooster_journey_id: Uuid,
        circadian_journey_id: Uuid,
    ) -> Result<(), iced::Error>
    where
        C: jungle_sdk::JungleClient + Clone + 'static,
    {
        let title = "Rooster Vision";
        iced::application(
            move || RoosterVisionUi::new(client.clone(), rooster_journey_id, circadian_journey_id),
            RoosterVisionUi::update,
            RoosterVisionUi::view,
        )
        .title(move |_app: &RoosterVisionUi| title.to_string())
        .subscription(RoosterVisionUi::subscription)
        .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .default_font(Font::with_name("Iosevka"))
        .antialiasing(true)
        .run()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Roost(args) => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(run_roost(args))?;
        }
        Command::Spawn(args) => run_spawn(args)?,
    }
    Ok(())
}

async fn run_roost(args: RoostArgs) -> Result<(), Box<dyn std::error::Error>> {
    info!(listen = %args.listen, "starting rooster roost server");
    #[allow(unused_mut)]
    let mut builder = jungle_sdk::server::ServerBuilder::new().listen(args.listen);
    #[allow(unused_mut)]
    let mut backend_selected = false;

    #[cfg(feature = "postgres")]
    if let Some(connection_string) = args.postgres_connection_string {
        builder = builder.postgres_connection_string(connection_string);
        backend_selected = true;
        info!("roost configured with postgres backend");
    }

    #[cfg(feature = "fjall")]
    if let Some(path) = args.fjall_path {
        ensure_parent_dir_exists(&path)?;
        builder = builder.fjall_path(path);
        backend_selected = true;
        info!("roost configured with fjall backend");
    }

    #[cfg(feature = "fjall")]
    if args.memory {
        builder = builder.memory();
        backend_selected = true;
        info!("roost configured with in-memory backend (--memory)");
    }

    if !backend_selected {
        warn!(
            "no persistence backend configured for roost; defaulting to in-memory backend (pass --fjall-path for persistence)"
        );
        builder = builder.memory();
    }

    builder.run().await?;
    Ok(())
}

struct SpawnSession {
    #[cfg(feature = "viewer")]
    client: Client,
    #[cfg(feature = "viewer")]
    rooster_journey_id: Uuid,
    #[cfg(feature = "viewer")]
    circadian_journey_id: Uuid,
    worker_handle: tokio::task::JoinHandle<()>,
}

async fn setup_spawn_session(args: &SpawnArgs) -> Result<SpawnSession, Box<dyn std::error::Error>> {
    info!(
        roost_addr = %args.roost_addr,
        server_name = %args.server_name,
        "connecting rooster spawn session client"
    );
    let client = connect_client_with_retry(&args).await?;
    info!("connected rooster spawn client");
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
        .clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());
    let rooster_seed = RoosterSeed {
        model_config: AgentModelConfig {
            base_url: args.openai_api_base_url.clone(),
            model: args.openai_model.clone(),
            bearer_token: openai_api_key,
        },
        settings: AgentSettings::default(),
    };
    info!("spawning rooster journey");
    let rooster_journey_id = client.spawn::<Rooster>(&rooster_seed).await?.journey_id;

    let circadian_seed = CircadianState {
        rooster_journey_id,
        interval_secs: args.circadian_interval_secs,
        roost_addr: args.roost_addr,
        server_name: args.server_name.clone(),
    };
    info!("spawning circadian journey");
    let circadian_journey_id = client.spawn::<Circadian>(&circadian_seed).await?.journey_id;

    info!(
        %rooster_journey_id,
        %circadian_journey_id,
        circadian_interval_secs = args.circadian_interval_secs,
        "rooster spawn active"
    );
    println!("spawned rooster journey: {rooster_journey_id}");
    println!("spawned circadian journey: {circadian_journey_id}");

    Ok(SpawnSession {
        #[cfg(feature = "viewer")]
        client,
        #[cfg(feature = "viewer")]
        rooster_journey_id,
        #[cfg(feature = "viewer")]
        circadian_journey_id,
        worker_handle,
    })
}

fn run_spawn(args: SpawnArgs) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating tokio runtime for rooster spawn");
    let runtime = tokio::runtime::Runtime::new()?;
    info!("setting up rooster spawn session");
    let session = runtime.block_on(setup_spawn_session(&args))?;
    info!("rooster spawn session ready");

    #[cfg(feature = "viewer")]
    {
        println!("launching rooster vision UI (close the window to stop)");
        info!("launching rooster vision UI");
        vision_ui::run_ui(
            session.client.clone(),
            session.rooster_journey_id,
            session.circadian_journey_id,
        )?;
        info!("rooster vision closed; shutting down worker");
    }

    #[cfg(not(feature = "viewer"))]
    {
        println!("press ctrl-c to stop this worker");
        runtime.block_on(tokio::signal::ctrl_c())?;
        info!("received ctrl-c; shutting down rooster worker");
    }

    runtime.block_on(async move {
        session.worker_handle.abort();
        let _ = session.worker_handle.await;
    });
    Ok(())
}

async fn connect_client_with_retry(args: &SpawnArgs) -> Result<Client, Box<dyn std::error::Error>> {
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        debug!(
            attempt = attempts,
            max_attempts = CONNECT_RETRY_ATTEMPTS,
            roost_addr = %args.roost_addr,
            "attempting rooster client connection"
        );

        let connect = Client::builder()
            .namespace(RoosterEcosystem::NAME)
            .remote(args.roost_addr)
            .server_name(args.server_name.clone())
            .build();

        match tokio::time::timeout(Duration::from_millis(CONNECT_TIMEOUT_MS), connect).await {
            Ok(Ok(client)) => {
                info!(attempt = attempts, "connected rooster client");
                return Ok(client);
            }
            Ok(Err(err)) => {
                warn!(
                    attempt = attempts,
                    max_attempts = CONNECT_RETRY_ATTEMPTS,
                    error = %err,
                    "failed rooster client connection attempt"
                );
                if attempts >= CONNECT_RETRY_ATTEMPTS {
                    return Err(Box::new(err));
                }
            }
            Err(_) => {
                warn!(
                    attempt = attempts,
                    max_attempts = CONNECT_RETRY_ATTEMPTS,
                    timeout_ms = CONNECT_TIMEOUT_MS,
                    "rooster client connection attempt timed out"
                );
                if attempts >= CONNECT_RETRY_ATTEMPTS {
                    return Err(format!(
                        "timed out connecting to rooster roost at {} after {} attempts ({}ms timeout each)",
                        args.roost_addr, CONNECT_RETRY_ATTEMPTS, CONNECT_TIMEOUT_MS
                    )
                    .into());
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(CONNECT_RETRY_DELAY_MS)).await;
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
