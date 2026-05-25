#![recursion_limit = "16384"]

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

use clap::{Parser, ValueEnum};
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
    instrumentation::SynthHandle,
};

const DEFAULT_BPM: f32 = 123.0;
const DEFAULT_WORKERS: usize = 2;
const DEFAULT_SYNTH_WORKERS: usize = 9;
const DEFAULT_SYNTH_QUEUE_SIZE: usize = 128;
const DEFAULT_PLAYBACK_DELAY_MS: u64 = 1_000;
const DEFAULT_EVENT_LEAD_TIME_MS: u64 = 128;
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
        return run_headless(
            args.bpm,
            args.mute,
            args.workers,
            args.synth_workers,
            args.synth_queue_size,
            args.playback_delay_ms,
            args.event_lead_time_ms,
            args.enabled_animals,
        );
    }
    run_with_ui(
        args.bpm,
        args.mute,
        args.workers,
        args.synth_workers,
        args.synth_queue_size,
        args.playback_delay_ms,
        args.event_lead_time_ms,
        args.enabled_animals,
    )
}

struct CliArgs {
    bpm: f32,
    headless: bool,
    mute: bool,
    workers: usize,
    synth_workers: usize,
    synth_queue_size: usize,
    playback_delay_ms: u64,
    event_lead_time_ms: u64,
    enabled_animals: BTreeSet<SelectedAnimal>,
}

#[derive(Debug, Parser)]
#[clap(name = "Welcome to the Jungle")]
struct WelcomeCliArgs {
    /// Tempo in BPM.
    #[clap(long = "bpm", value_parser = parse_bpm_value, default_value_t = DEFAULT_BPM)]
    bpm: f32,
    /// Run without the viewer UI and exit after playback.
    #[clap(long = "headless")]
    headless: bool,
    /// Disable live audio output and run with a stub audio clock.
    #[clap(long = "mute")]
    mute: bool,
    /// Number of runtime workers to spawn.
    #[clap(
        long = "workers",
        default_value_t = DEFAULT_WORKERS,
        value_parser = parse_workers_value
    )]
    workers: usize,
    /// Number of synth workers to spawn.
    #[clap(
        long = "synth-workers",
        default_value_t = DEFAULT_SYNTH_WORKERS,
        value_parser = parse_synth_workers_value
    )]
    synth_workers: usize,
    /// Queue capacity per synth worker.
    #[clap(
        long = "synth-queue-size",
        default_value_t = DEFAULT_SYNTH_QUEUE_SIZE,
        value_parser = parse_synth_queue_size_value
    )]
    synth_queue_size: usize,
    /// Playback delay in milliseconds.
    #[clap(
        long = "playback-delay-ms",
        default_value_t = DEFAULT_PLAYBACK_DELAY_MS
    )]
    playback_delay_ms: u64,
    /// Event lead time in milliseconds.
    #[clap(
        long = "event-lead-time",
        default_value_t = DEFAULT_EVENT_LEAD_TIME_MS
    )]
    event_lead_time_ms: u64,
    /// Comma-delimited or repeatable list of enabled animals.
    #[clap(long = "animals", value_enum, value_delimiter = ',')]
    animals: Vec<SelectedAnimalCli>,
}

fn run_with_ui(
    bpm: f32,
    mute: bool,
    workers: usize,
    synth_workers: usize,
    synth_queue_size: usize,
    playback_delay_ms: u64,
    event_lead_time_ms: u64,
    enabled_animals: BTreeSet<SelectedAnimal>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (setup_tx, setup_rx) = std::sync::mpsc::sync_channel::<Result<UiSetup, String>>(1);
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Instant>(1);
    let ui_shutdown = ui::ShutdownFlag::new();
    let shutdown_for_runtime = ui_shutdown.clone();
    let runtime_thread = std::thread::spawn(move || {
        run_runtime_thread(
            bpm,
            mute,
            workers,
            synth_workers,
            synth_queue_size,
            playback_delay_ms,
            event_lead_time_ms,
            enabled_animals,
            shutdown_for_runtime,
            setup_tx,
            started_rx,
        )
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
    workers: usize,
    synth_workers: usize,
    synth_queue_size: usize,
    playback_delay_ms: u64,
    event_lead_time_ms: u64,
    enabled_animals: BTreeSet<SelectedAnimal>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        bpm,
        mute,
        workers,
        synth_workers,
        synth_queue_size,
        playback_delay_ms,
        event_lead_time_ms,
        "running welcome example in headless mode"
    );

    let (setup_tx, setup_rx) = std::sync::mpsc::sync_channel::<Result<UiSetup, String>>(1);
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Instant>(1);
    let ui_shutdown = ui::ShutdownFlag::new();
    let shutdown_for_runtime = ui_shutdown.clone();
    let runtime_thread = std::thread::spawn(move || {
        run_runtime_thread(
            bpm,
            mute,
            workers,
            synth_workers,
            synth_queue_size,
            playback_delay_ms,
            event_lead_time_ms,
            enabled_animals,
            shutdown_for_runtime,
            setup_tx,
            started_rx,
        )
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
    let env_filter = env_filter
        .add_directive("iced_sugiyama=off".parse().unwrap())
        .add_directive("iced_winit=off".parse().unwrap());
    let _ = fmt().with_env_filter(env_filter).compact().try_init();
}

struct UiSetup {
    client: ui::DeferredJungleClient<RuntimeClient>,
    journeys: ui::JourneyIds,
}

fn run_runtime_thread(
    bpm: f32,
    mute: bool,
    workers: usize,
    synth_workers: usize,
    synth_queue_size: usize,
    playback_delay_ms: u64,
    event_lead_time_ms: u64,
    enabled_animals: BTreeSet<SelectedAnimal>,
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
            let audio_handle =
                audio_handle.with_playback_delay(Duration::from_millis(playback_delay_ms));
            (audio_handle, None, Some(stub_audio))
        } else {
            let audio_engine = AudioEngine::start_default().await.map_err(|err| {
                error!(error = %err, "failed starting audio engine");
                err.to_string()
            })?;
            let audio_handle = audio_engine
                .handle()
                .with_playback_delay(Duration::from_millis(playback_delay_ms));
            (audio_handle, Some(audio_engine), None)
        };
        let synth_handle = SynthHandle::new_with_config(synth_workers, synth_queue_size);
        let mut worker_metronomes = Vec::with_capacity(workers);
        for worker_index in 0..workers {
            let metronome = metronome::Metronome::spawn(bpm);
            let ecosystem = TheJungle::new_with_metronome_and_synth(
                audio_handle.clone(),
                synth_handle.clone(),
                bpm,
                metronome,
            );
            let metronome = ecosystem.metronome().clone();
            metronome.arm_start_barrier();
            worker_metronomes.push(metronome);
            let worker_client = client.clone();
            tokio::spawn(async move {
                let worker = JungleWorker::new(ecosystem, worker_client);
                if let Err(err) = worker.spawn().await {
                    error!(
                        error = %err,
                        worker_index,
                        "welcome worker task exited with error"
                    );
                }
            });
        }
        info!(
            workers,
            synth_workers, synth_queue_size, "started welcome workers"
        );

        let seed = postcard::to_allocvec(&()).map_err(|err| {
            error!(error = %err, "failed serializing journey seed");
            err.to_string()
        })?;
        if enabled_animals.len() < SelectedAnimal::all().len() {
            let selected = enabled_animals
                .iter()
                .map(|animal| animal.as_cli_name())
                .collect::<Vec<_>>();
            info!(animals = ?selected, "running welcome with selected animals");
        }
        let lead_vocalist_fut = async {
            if enabled_animals.contains(&SelectedAnimal::LeadVocalist) {
                client
                    .start_journey::<LeadVocalist>(seed.clone())
                    .await
                    .map(Some)
                    .map_err(|err| {
                        error!(error = %err, "failed starting lead vocalist journey");
                        err.to_string()
                    })
            } else {
                Ok(None)
            }
        };
        let lead_guitarist_fut = async {
            if enabled_animals.contains(&SelectedAnimal::LeadGuitarist) {
                client
                    .start_journey::<LeadGuitarist>(seed.clone())
                    .await
                    .map(Some)
                    .map_err(|err| {
                        error!(error = %err, "failed starting lead guitarist journey");
                        err.to_string()
                    })
            } else {
                Ok(None)
            }
        };
        let rhythm_guitarist_fut = async {
            if enabled_animals.contains(&SelectedAnimal::RhythmGuitarist) {
                client
                    .start_journey::<RhythmGuitarist>(seed.clone())
                    .await
                    .map(Some)
                    .map_err(|err| {
                        error!(error = %err, "failed starting rhythm guitarist journey");
                        err.to_string()
                    })
            } else {
                Ok(None)
            }
        };
        let bass_fut = async {
            if enabled_animals.contains(&SelectedAnimal::Bassist) {
                client
                    .start_journey::<BassAnimal>(seed.clone())
                    .await
                    .map(Some)
                    .map_err(|err| {
                        error!(error = %err, "failed starting bass journey");
                        err.to_string()
                    })
            } else {
                Ok(None)
            }
        };
        let drums_fut = async {
            if enabled_animals.contains(&SelectedAnimal::Drummer) {
                client
                    .start_journey::<Drums>(seed.clone())
                    .await
                    .map(Some)
                    .map_err(|err| {
                        error!(error = %err, "failed starting drums journey");
                        err.to_string()
                    })
            } else {
                Ok(None)
            }
        };
        let (lead_vocalist, lead_guitarist, rhythm_guitarist, bass, drums) = tokio::join!(
            lead_vocalist_fut,
            lead_guitarist_fut,
            rhythm_guitarist_fut,
            bass_fut,
            drums_fut,
        );
        let journeys = ui::JourneyIds {
            lead_vocalist: lead_vocalist?,
            lead_guitarist: lead_guitarist?,
            rhythm_guitarist: rhythm_guitarist?,
            bass: bass?,
            drums: drums?,
        };
        futures::future::join_all(
            worker_metronomes
                .iter()
                .map(|metronome| metronome.release_start_barrier_on_downbeat()),
        )
        .await;

        keep_alive.audio_engine = audio_engine;
        keep_alive.stub_audio = stub_audio;
        let ui_client = ui::DeferredJungleClient::new(
            client.clone(),
            Duration::from_millis(playback_delay_ms),
            Duration::from_millis(event_lead_time_ms),
        );
        Ok::<(UiSetup, RuntimeKeepAlive), String>((
            UiSetup {
                client: ui_client,
                journeys,
            },
            keep_alive,
        ))
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
    let parsed = WelcomeCliArgs::parse();
    let enabled_animals = if parsed.animals.is_empty() {
        SelectedAnimal::all()
    } else {
        parsed
            .animals
            .into_iter()
            .map(SelectedAnimal::from)
            .collect::<BTreeSet<_>>()
    };
    Ok(CliArgs {
        bpm: parsed.bpm,
        headless: parsed.headless,
        mute: parsed.mute,
        workers: parsed.workers,
        synth_workers: parsed.synth_workers,
        synth_queue_size: parsed.synth_queue_size,
        playback_delay_ms: parsed.playback_delay_ms,
        event_lead_time_ms: parsed.event_lead_time_ms,
        enabled_animals,
    })
}

fn parse_bpm_value(value: &str) -> Result<f32, String> {
    let bpm = value
        .parse::<f32>()
        .map_err(|_| format!("invalid BPM argument: {value}"))?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return Err(format!(
            "BPM must be a positive finite number, got: {value}"
        ));
    }
    Ok(bpm)
}

fn parse_workers_value(value: &str) -> Result<usize, String> {
    let workers = value
        .parse::<usize>()
        .map_err(|_| format!("invalid workers argument: {value}"))?;
    if workers == 0 {
        return Err("workers must be at least 1".to_string());
    }
    Ok(workers)
}

fn parse_synth_workers_value(value: &str) -> Result<usize, String> {
    let synth_workers = value
        .parse::<usize>()
        .map_err(|_| format!("invalid synth workers argument: {value}"))?;
    if synth_workers == 0 {
        return Err("synth workers must be at least 1".to_string());
    }
    Ok(synth_workers)
}

fn parse_synth_queue_size_value(value: &str) -> Result<usize, String> {
    let synth_queue_size = value
        .parse::<usize>()
        .map_err(|_| format!("invalid synth queue size argument: {value}"))?;
    if synth_queue_size == 0 {
        return Err("synth queue size must be at least 1".to_string());
    }
    Ok(synth_queue_size)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SelectedAnimal {
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    Bassist,
    Drummer,
}

impl SelectedAnimal {
    fn all() -> BTreeSet<Self> {
        [
            Self::LeadVocalist,
            Self::LeadGuitarist,
            Self::RhythmGuitarist,
            Self::Bassist,
            Self::Drummer,
        ]
        .into_iter()
        .collect()
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

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum SelectedAnimalCli {
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    #[value(alias = "bass")]
    Bassist,
    #[value(alias = "drums")]
    Drummer,
}

impl From<SelectedAnimalCli> for SelectedAnimal {
    fn from(value: SelectedAnimalCli) -> Self {
        match value {
            SelectedAnimalCli::LeadVocalist => Self::LeadVocalist,
            SelectedAnimalCli::LeadGuitarist => Self::LeadGuitarist,
            SelectedAnimalCli::RhythmGuitarist => Self::RhythmGuitarist,
            SelectedAnimalCli::Bassist => Self::Bassist,
            SelectedAnimalCli::Drummer => Self::Drummer,
        }
    }
}
