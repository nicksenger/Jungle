mod animals;
mod assets;
mod audio;
mod ecosystem;
mod effect;
mod instrumentation;
mod metronome;
mod ui;

use std::collections::BTreeSet;
#[cfg(feature = "transport")]
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use jungle_sdk::core::JungleWorker;
#[cfg(feature = "transport")]
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::JungleClient;
#[cfg(not(feature = "transport"))]
use jungle_sdk::LocalClient;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};
#[cfg(feature = "redb")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use testcontainers::runners::AsyncRunner;
#[cfg(feature = "postgres")]
use testcontainers_modules::postgres::Postgres;

use crate::{
    animals::{Bass as BassAnimal, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist},
    audio::{AudioEngine, AudioHandle, StubAudioKeepAlive},
    ecosystem::TheJungle,
};

const DEFAULT_BPM: f32 = 123.0;
const UI_MIN_UPTIME_BEFORE_SHUTDOWN: Duration = Duration::from_secs(5 * 60);

#[cfg(feature = "transport")]
pub(crate) type RuntimeClient = jungle_sdk::Client<TheJungle>;
#[cfg(not(feature = "transport"))]
pub(crate) type RuntimeClient = LocalClient;

#[cfg(feature = "postgres")]
type PostgresContainer = testcontainers::ContainerAsync<Postgres>;

#[derive(Default)]
struct RuntimeKeepAlive {
    audio_engine: Option<AudioEngine>,
    stub_audio: Option<StubAudioKeepAlive>,
    server_task: Option<tokio::task::JoinHandle<()>>,
    #[cfg(feature = "postgres")]
    postgres_container: Option<PostgresContainer>,
}

impl RuntimeKeepAlive {
    fn shutdown(&mut self) {
        if let Some(task) = self.server_task.take() {
            task.abort();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let args = parse_cli_args()?;
    if args.headless {
        return run_headless(args.bpm, args.mute, args.omit);
    }
    run_with_ui(args.bpm, args.mute, args.omit)
}

struct CliArgs {
    bpm: f32,
    headless: bool,
    mute: bool,
    omit: BTreeSet<OmittedAnimal>,
}

fn run_with_ui(
    bpm: f32,
    mute: bool,
    omit: BTreeSet<OmittedAnimal>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (setup_tx, setup_rx) = std::sync::mpsc::sync_channel::<Result<UiSetup, String>>(1);
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Instant>(1);
    let ui_shutdown = ui::ShutdownFlag::new();
    let shutdown_for_runtime = ui_shutdown.clone();
    let runtime_thread = std::thread::spawn(move || {
        run_runtime_thread(bpm, mute, omit, shutdown_for_runtime, setup_tx, started_rx)
    });

    let setup_result = setup_rx.recv().map_err(|err| {
        error!(error = %err, "failed receiving UI setup from runtime thread");
        std::io::Error::other(format!(
            "failed to receive UI setup from runtime thread: {err}"
        ))
    })?;
    let setup = setup_result.map_err(|err| {
        error!(error = %err, "runtime thread setup failed");
        std::io::Error::other(err)
    })?;

    let ui_started_at = Instant::now();
    started_tx.send(ui_started_at).map_err(|err| {
        error!(error = %err, "failed notifying runtime thread that UI started");
        std::io::Error::other(format!(
            "failed to notify runtime thread that UI started: {err}"
        ))
    })?;

    ui::run_ui(setup.client, setup.journeys, ui_shutdown)?;

    let thread_result = runtime_thread.join().map_err(|_| {
        error!("runtime thread panicked while running welcome example");
        std::io::Error::other("runtime thread panicked while running welcome example")
    })?;
    thread_result.map_err(|err| {
        error!(error = %err, "runtime thread returned an error");
        std::io::Error::other(err)
    })?;
    Ok(())
}

fn run_headless(
    bpm: f32,
    mute: bool,
    omit: BTreeSet<OmittedAnimal>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(bpm, mute, "running welcome example in headless mode");

    let (setup_tx, setup_rx) = std::sync::mpsc::sync_channel::<Result<UiSetup, String>>(1);
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Instant>(1);
    let ui_shutdown = ui::ShutdownFlag::new();
    let shutdown_for_runtime = ui_shutdown.clone();
    let runtime_thread = std::thread::spawn(move || {
        run_runtime_thread(bpm, mute, omit, shutdown_for_runtime, setup_tx, started_rx)
    });

    let setup_result = setup_rx.recv().map_err(|err| {
        error!(
            error = %err,
            "failed receiving runtime setup from runtime thread"
        );
        std::io::Error::other(format!(
            "failed to receive runtime setup from runtime thread: {err}"
        ))
    })?;
    let _setup = setup_result.map_err(|err| {
        error!(error = %err, "runtime thread setup failed");
        std::io::Error::other(err)
    })?;

    started_tx.send(Instant::now()).map_err(|err| {
        error!(
            error = %err,
            "failed notifying runtime thread that headless run started"
        );
        std::io::Error::other(format!(
            "failed to notify runtime thread that headless run started: {err}"
        ))
    })?;

    let thread_result = runtime_thread.join().map_err(|_| {
        error!("runtime thread panicked while running welcome example headless");
        std::io::Error::other("runtime thread panicked while running welcome example headless")
    })?;
    thread_result.map_err(|err| {
        error!(error = %err, "runtime thread returned an error");
        std::io::Error::other(err)
    })?;
    Ok(())
}

fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,welcome=debug"));
    let _ = fmt().with_env_filter(env_filter).compact().try_init();
}

struct UiSetup {
    client: RuntimeClient,
    journeys: ui::JourneyIds,
}

fn run_runtime_thread(
    bpm: f32,
    mute: bool,
    omit: BTreeSet<OmittedAnimal>,
    ui_shutdown: ui::ShutdownFlag,
    setup_tx: std::sync::mpsc::SyncSender<Result<UiSetup, String>>,
    started_rx: std::sync::mpsc::Receiver<Instant>,
) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|err| {
        error!(error = %err, "failed creating tokio runtime for welcome example");
        err.to_string()
    })?;

    let setup = runtime.block_on(async {
        let (client, mut keep_alive) = setup_runtime_client().await?;
        let (audio_handle, audio_engine, stub_audio) = if mute {
            info!("starting welcome runtime in muted mode using audio stub");
            let (audio_handle, stub_audio) = AudioHandle::stub();
            (audio_handle, None, Some(stub_audio))
        } else {
            let audio_engine = AudioEngine::start_default().await.map_err(|err| {
                error!(error = %err, "failed starting audio engine");
                err.to_string()
            })?;
            let audio_handle = audio_engine.handle();
            (audio_handle, Some(audio_engine), None)
        };
        let ecosystem = TheJungle::new(audio_handle, bpm);
        let worker_client = client.clone();
        let _worker_task = tokio::spawn(async move {
            let worker = JungleWorker::new(ecosystem, worker_client);
            if let Err(err) = worker.spawn().await {
                error!(error = %err, "welcome worker task exited with error");
            }
        });

        let seed = postcard::to_allocvec(&()).map_err(|err| {
            error!(error = %err, "failed serializing journey seed");
            err.to_string()
        })?;
        if !omit.is_empty() {
            let omitted = omit
                .iter()
                .map(|animal| animal.as_cli_name())
                .collect::<Vec<_>>();
            info!(animals = ?omitted, "omitting requested welcome animals");
        }
        let journeys = ui::JourneyIds {
            lead_vocalist: if omit.contains(&OmittedAnimal::LeadVocalist) {
                None
            } else {
                Some(
                    client
                        .start_journey::<LeadVocalist>(seed.clone())
                        .await
                        .map_err(|err| {
                            error!(error = %err, "failed starting lead vocalist journey");
                            err.to_string()
                        })?,
                )
            },
            lead_guitarist: if omit.contains(&OmittedAnimal::LeadGuitarist) {
                None
            } else {
                Some(
                    client
                        .start_journey::<LeadGuitarist>(seed.clone())
                        .await
                        .map_err(|err| {
                            error!(error = %err, "failed starting lead guitarist journey");
                            err.to_string()
                        })?,
                )
            },
            rhythm_guitarist: if omit.contains(&OmittedAnimal::RhythmGuitarist) {
                None
            } else {
                Some(
                    client
                        .start_journey::<RhythmGuitarist>(seed.clone())
                        .await
                        .map_err(|err| {
                            error!(error = %err, "failed starting rhythm guitarist journey");
                            err.to_string()
                        })?,
                )
            },
            bass: if omit.contains(&OmittedAnimal::Bassist) {
                None
            } else {
                Some(
                    client
                        .start_journey::<BassAnimal>(seed.clone())
                        .await
                        .map_err(|err| {
                            error!(error = %err, "failed starting bass journey");
                            err.to_string()
                        })?,
                )
            },
            drums: if omit.contains(&OmittedAnimal::Drummer) {
                None
            } else {
                Some(client.start_journey::<Drums>(seed).await.map_err(|err| {
                    error!(error = %err, "failed starting drums journey");
                    err.to_string()
                })?)
            },
        };

        keep_alive.audio_engine = audio_engine;
        keep_alive.stub_audio = stub_audio;
        Ok::<(UiSetup, RuntimeKeepAlive), String>((UiSetup { client, journeys }, keep_alive))
    });

    let (setup, mut keep_alive) = match setup {
        Ok(value) => value,
        Err(err) => {
            let _ = setup_tx.send(Err(err.clone()));
            return Err(err);
        }
    };

    if let Err(err) = setup_tx.send(Ok(setup)) {
        keep_alive.shutdown();
        error!(error = %err, "failed sending UI setup to main thread");
        return Err(format!("failed to send UI setup to main thread: {err}"));
    }

    let ui_started_at = started_rx.recv().map_err(|err| {
        keep_alive.shutdown();
        error!(
            error = %err,
            "failed receiving UI start signal from main thread"
        );
        format!("failed to receive UI start signal from main thread: {err}")
    })?;

    let result = runtime.block_on(play_audio_and_schedule_shutdown(
        bpm,
        ui_shutdown,
        ui_started_at,
    ));
    keep_alive.shutdown();
    result
}

#[cfg(feature = "transport")]
fn reserve_local_addr() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
        .expect("should bind temporary udp socket for port reservation");
    socket
        .local_addr()
        .expect("temporary udp socket should expose local address")
}

async fn setup_runtime_client() -> Result<(RuntimeClient, RuntimeKeepAlive), String> {
    #[cfg(feature = "transport")]
    {
        setup_transport_runtime_client().await
    }
    #[cfg(not(feature = "transport"))]
    {
        setup_local_runtime_client().await
    }
}

#[cfg(not(feature = "transport"))]
async fn setup_local_runtime_client() -> Result<(RuntimeClient, RuntimeKeepAlive), String> {
    #[cfg(feature = "postgres")]
    let mut keep_alive = RuntimeKeepAlive::default();
    #[cfg(not(feature = "postgres"))]
    let keep_alive = RuntimeKeepAlive::default();

    #[cfg(feature = "postgres")]
    {
        let postgres = Postgres::default().start().await.map_err(|err| {
            error!(error = %err, "failed starting postgres testcontainer");
            err.to_string()
        })?;
        let pg_port = postgres.get_host_port_ipv4(5432).await.map_err(|err| {
            error!(error = %err, "failed resolving postgres mapped port");
            err.to_string()
        })?;
        let connection_string =
            format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");
        let backend = jungle_sdk::server::Server::builder()
            .postgres_connection_string(connection_string)
            .build()
            .await
            .map_err(|err| {
                error!(error = %err, "failed building postgres server backend");
                err.to_string()
            })?;
        let client = LocalClient::builder()
            .backend(backend)
            .build()
            .await
            .map_err(|err| {
                error!(error = %err, "failed building local client with postgres backend");
                err.to_string()
            })?;
        keep_alive.postgres_container = Some(postgres);
        return Ok((client, keep_alive));
    }

    #[cfg(all(feature = "redb", not(feature = "postgres")))]
    {
        let db_path = std::env::temp_dir().join(format!("jungle-welcome-{}.redb", Uuid::new_v4()));
        let backend = jungle_sdk::server::Server::builder()
            .redb_path(db_path)
            .build()
            .await
            .map_err(|err| {
                error!(error = %err, "failed building redb server backend");
                err.to_string()
            })?;
        let client = LocalClient::builder()
            .backend(backend)
            .build()
            .await
            .map_err(|err| {
                error!(error = %err, "failed building local client with redb backend");
                err.to_string()
            })?;
        return Ok((client, keep_alive));
    }

    #[cfg(not(any(feature = "redb", feature = "postgres")))]
    {
        let client = LocalClient::builder().build().await.map_err(|err| {
            error!(error = %err, "failed building in-memory local client");
            err.to_string()
        })?;
        Ok((client, keep_alive))
    }
}

#[cfg(feature = "transport")]
async fn setup_transport_runtime_client() -> Result<(RuntimeClient, RuntimeKeepAlive), String> {
    let mut keep_alive = RuntimeKeepAlive::default();
    let listen_addr = reserve_local_addr();

    #[cfg(feature = "postgres")]
    {
        let postgres = Postgres::default().start().await.map_err(|err| {
            error!(error = %err, "failed starting postgres testcontainer");
            err.to_string()
        })?;
        let pg_port = postgres.get_host_port_ipv4(5432).await.map_err(|err| {
            error!(error = %err, "failed resolving postgres mapped port");
            err.to_string()
        })?;
        let connection_string =
            format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");
        let server_task = tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .listen(listen_addr)
                .postgres_connection_string(connection_string)
                .run()
                .await
            {
                error!(error = %err, "welcome transport server exited with error");
            }
        });
        let client = connect_transport_client_with_retry(listen_addr).await?;
        keep_alive.server_task = Some(server_task);
        keep_alive.postgres_container = Some(postgres);
        return Ok((client, keep_alive));
    }

    #[cfg(all(feature = "redb", not(feature = "postgres")))]
    {
        let db_path = std::env::temp_dir().join(format!("jungle-welcome-{}.redb", Uuid::new_v4()));
        let server_task = tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
            {
                error!(error = %err, "welcome transport server exited with error");
            }
        });
        let client = connect_transport_client_with_retry(listen_addr).await?;
        keep_alive.server_task = Some(server_task);
        return Ok((client, keep_alive));
    }

    #[cfg(not(any(feature = "redb", feature = "postgres")))]
    {
        let server_task = tokio::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .listen(listen_addr)
                .memory()
                .run()
                .await
            {
                error!(error = %err, "welcome transport server exited with error");
            }
        });
        let client = connect_transport_client_with_retry(listen_addr).await?;
        keep_alive.server_task = Some(server_task);
        Ok((client, keep_alive))
    }
}

#[cfg(feature = "transport")]
async fn connect_transport_client_with_retry(remote: SocketAddr) -> Result<RuntimeClient, String> {
    for attempt in 0..40 {
        match jungle_sdk::Client::builder()
            .ecosystem::<TheJungle>()
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return Ok(client),
            Err(err) if attempt < 39 => {
                tokio::time::sleep(Duration::from_millis(25)).await;
                let _ = err;
            }
            Err(err) => {
                error!(error = %err, "failed connecting transport client to welcome server");
                return Err(err.to_string());
            }
        }
    }

    Err("transport client retry loop exhausted".to_string())
}

async fn play_audio_and_schedule_shutdown(
    bpm: f32,
    ui_shutdown: ui::ShutdownFlag,
    ui_started_at: Instant,
) -> Result<(), String> {
    info!(
        bpm,
        "starting welcome playback with direct non-flow instruments disabled"
    );
    let elapsed_since_ui_start = ui_started_at.elapsed();
    if elapsed_since_ui_start < UI_MIN_UPTIME_BEFORE_SHUTDOWN {
        tokio::time::sleep(UI_MIN_UPTIME_BEFORE_SHUTDOWN - elapsed_since_ui_start).await;
    }
    ui_shutdown.request_shutdown();
    Ok(())
}

fn parse_cli_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut bpm = DEFAULT_BPM;
    let mut headless = false;
    let mut mute = false;
    let mut omit = BTreeSet::new();

    while let Some(arg) = args.next() {
        if arg == "--headless" {
            headless = true;
            continue;
        }

        if arg == "--mute" {
            mute = true;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--omit=") {
            parse_omit_list(value, &mut omit)?;
            continue;
        }

        if arg == "--omit" {
            let value = args
                .next()
                .ok_or_else(|| "--omit requires a value".to_string())?;
            parse_omit_list(&value, &mut omit)?;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--bpm=") {
            bpm = parse_bpm_value(value)?;
            continue;
        }

        if arg == "--bpm" {
            let value = args
                .next()
                .ok_or_else(|| "--bpm requires a value".to_string())?;
            bpm = parse_bpm_value(&value)?;
            continue;
        }

        if arg.starts_with("--") {
            return Err(format!("Unknown argument: {arg}").into());
        }

        // Keep supporting the legacy positional form for compatibility.
        bpm = parse_bpm_value(&arg)?;
    }

    Ok(CliArgs {
        bpm,
        headless,
        mute,
        omit,
    })
}

fn parse_bpm_value(value: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let bpm = value
        .parse::<f32>()
        .map_err(|_| format!("Invalid BPM argument: {value}"))?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return Err(format!("BPM must be a positive finite number, got: {value}").into());
    }
    Ok(bpm)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OmittedAnimal {
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    Bassist,
    Drummer,
}

impl OmittedAnimal {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "lead-vocalist" => Some(Self::LeadVocalist),
            "lead-guitarist" => Some(Self::LeadGuitarist),
            "rhythm-guitarist" => Some(Self::RhythmGuitarist),
            "bassist" | "bass" => Some(Self::Bassist),
            "drummer" | "drums" => Some(Self::Drummer),
            _ => None,
        }
    }

    fn as_cli_name(self) -> &'static str {
        match self {
            Self::LeadVocalist => "lead-vocalist",
            Self::LeadGuitarist => "lead-guitarist",
            Self::RhythmGuitarist => "rhythm-guitarist",
            Self::Bassist => "bassist",
            Self::Drummer => "drummer",
        }
    }
}

fn parse_omit_list(
    value: &str,
    omit: &mut BTreeSet<OmittedAnimal>,
) -> Result<(), Box<dyn std::error::Error>> {
    if value.is_empty() {
        return Err("--omit requires a comma-delimited list of animals".into());
    }

    for token in value.split(',') {
        if token.is_empty() {
            return Err(format!("invalid --omit list '{value}': contains an empty entry").into());
        }
        if token.chars().any(|ch| ch.is_ascii_uppercase()) {
            return Err(
                format!("invalid --omit entry '{token}': expected lowercase animal names").into(),
            );
        }
        let Some(animal) = OmittedAnimal::parse(token) else {
            return Err(format!(
                "unknown --omit entry '{token}'; supported values: lead-vocalist, lead-guitarist, rhythm-guitarist, bassist, drummer"
            )
            .into());
        };
        omit.insert(animal);
    }

    Ok(())
}
