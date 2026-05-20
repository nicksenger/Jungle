mod animals;
mod assets;
mod audio;
mod effects;
mod flow;
mod instrumentation;
mod metronome;
mod score;
mod ui;

#[cfg(feature = "transport")]
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};
use std::{
    sync::{Arc, Mutex},
    time::Instant as StdInstant,
};

use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
#[cfg(feature = "transport")]
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::JungleClient;
#[cfg(not(feature = "transport"))]
use jungle_sdk::LocalClient;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
#[cfg(feature = "redb")]
use uuid::Uuid;
use tokio::sync::Notify;

#[cfg(feature = "postgres")]
use testcontainers::runners::AsyncRunner;
#[cfg(feature = "postgres")]
use testcontainers_modules::postgres::Postgres;

use crate::{
    animals::{Bass as BassAnimal, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist},
    audio::{AudioEngine, AudioHandle},
    instrumentation::{
        Bass, BassArticulation, Cymbal, CymbalArticulation, ElectricGuitar,
        ElectricGuitarArticulation, Error as InstrumentError, HiHat, HiHatArticulation, Instrument,
        KickDrum, KickDrumArticulation, Note, SnareDrum, SnareDrumArticulation, Toms,
        TomsArticulation, Vocals, VocalsArticulation,
    },
    metronome::{Metronome, MetronomeSync},
    score::{rhythm_guitar_intro_score, ScheduledNote},
};

const DEFAULT_BPM: f32 = 123.0;
const UI_MIN_UPTIME_BEFORE_SHUTDOWN: Duration = Duration::from_secs(5 * 60);
const MIN_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(120);
const MAX_SUBMISSION_ATTEMPTS: u32 = 6;

#[derive(Animals)]
struct WelcomeAnimals(
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    BassAnimal,
    Drums,
);

#[derive(Clone, Default)]
struct PlaybackClock {
    started_at: Arc<Mutex<Option<StdInstant>>>,
    started_notify: Arc<Notify>,
}

impl PlaybackClock {
    fn start_now(&self) -> StdInstant {
        let now = StdInstant::now();
        let mut started_at = self
            .started_at
            .lock()
            .expect("playback clock mutex should not be poisoned");
        if started_at.is_none() {
            *started_at = Some(now);
            self.started_notify.notify_waiters();
        }
        started_at.unwrap_or(now)
    }

    async fn wait_started(&self) -> tokio::time::Instant {
        loop {
            let started_at = {
                *self
                    .started_at
                    .lock()
                    .expect("playback clock mutex should not be poisoned")
            };
            if let Some(started_at) = started_at {
                let elapsed = StdInstant::now().saturating_duration_since(started_at);
                return tokio::time::Instant::now()
                    .checked_sub(elapsed)
                    .unwrap_or_else(tokio::time::Instant::now);
            }

            self.started_notify.notified().await;
        }
    }
}

struct WelcomeEcosystem {
    rhythm_guitar: ElectricGuitar,
    rhythm_guitar_intro: Arc<[ScheduledNote<ElectricGuitarArticulation>]>,
    playback_clock: PlaybackClock,
}

impl WelcomeEcosystem {
    fn new(audio_handle: AudioHandle, bpm: f32, playback_clock: PlaybackClock) -> Self {
        let rhythm_guitar_intro: Arc<[ScheduledNote<ElectricGuitarArticulation>]> =
            rhythm_guitar_intro_score(bpm).into();

        Self {
            rhythm_guitar: ElectricGuitar::new(audio_handle),
            rhythm_guitar_intro,
            playback_clock,
        }
    }

    fn rhythm_guitar(&self) -> &ElectricGuitar {
        &self.rhythm_guitar
    }

    fn rhythm_guitar_intro_note(
        &self,
        index: u16,
    ) -> Option<ScheduledNote<ElectricGuitarArticulation>> {
        self.rhythm_guitar_intro
            .get(index as usize)
            .copied()
            .map(|note| with_articulation(note, ElectricGuitarArticulation::RhythmSustained))
    }

    fn playback_clock(&self) -> &PlaybackClock {
        &self.playback_clock
    }
}

impl Ecosystem for WelcomeEcosystem {
    const NAME: &'static str = "welcome";
    type Animals = WelcomeAnimals;
}

#[cfg(feature = "transport")]
pub(crate) type RuntimeClient = jungle_sdk::Client<WelcomeEcosystem>;
#[cfg(not(feature = "transport"))]
pub(crate) type RuntimeClient = LocalClient;

#[cfg(feature = "postgres")]
type PostgresContainer = testcontainers::ContainerAsync<Postgres>;

#[derive(Default)]
struct RuntimeKeepAlive {
    audio_engine: Option<AudioEngine>,
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
    let bpm = parse_bpm_arg()?;

    let (setup_tx, setup_rx) = std::sync::mpsc::sync_channel::<Result<UiSetup, String>>(1);
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Instant>(1);
    let ui_shutdown = ui::ShutdownFlag::new();
    let shutdown_for_runtime = ui_shutdown.clone();
    let runtime_thread = std::thread::spawn(move || {
        run_runtime_thread(bpm, shutdown_for_runtime, setup_tx, started_rx)
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
        let audio_engine = AudioEngine::start_default().await.map_err(|err| {
            error!(error = %err, "failed starting audio engine");
            err.to_string()
        })?;
        let audio_handle = audio_engine.handle();
        let playback_clock = PlaybackClock::default();
        let ecosystem = WelcomeEcosystem::new(audio_handle.clone(), bpm, playback_clock.clone());
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
        let journeys = ui::JourneyIds {
            lead_vocalist: client
                .start_journey::<LeadVocalist>(seed.clone())
                .await
                .map_err(|err| {
                    error!(error = %err, "failed starting lead vocalist journey");
                    err.to_string()
                })?,
            lead_guitarist: client
                .start_journey::<LeadGuitarist>(seed.clone())
                .await
                .map_err(|err| {
                    error!(error = %err, "failed starting lead guitarist journey");
                    err.to_string()
                })?,
            rhythm_guitarist: client
                .start_journey::<RhythmGuitarist>(seed.clone())
                .await
                .map_err(|err| {
                    error!(error = %err, "failed starting rhythm guitarist journey");
                    err.to_string()
                })?,
            bass: client
                .start_journey::<BassAnimal>(seed.clone())
                .await
                .map_err(|err| {
                    error!(error = %err, "failed starting bass journey");
                    err.to_string()
                })?,
            drums: client.start_journey::<Drums>(seed).await.map_err(|err| {
                error!(error = %err, "failed starting drums journey");
                err.to_string()
            })?,
        };

        keep_alive.audio_engine = Some(audio_engine);
        Ok::<(UiSetup, RuntimeKeepAlive, PlaybackClock), String>((
            UiSetup { client, journeys },
            keep_alive,
            playback_clock,
        ))
    });

    let (setup, mut keep_alive, playback_clock) = match setup {
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
        playback_clock,
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
            .ecosystem::<WelcomeEcosystem>()
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
    playback_clock: PlaybackClock,
) -> Result<(), String> {
    let _ = playback_clock.start_now();
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

fn parse_bpm_arg() -> Result<f32, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut bpm = DEFAULT_BPM;

    while let Some(arg) = args.next() {
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

        // Keep supporting the legacy positional form for compatibility.
        bpm = parse_bpm_value(&arg)?;
    }

    Ok(bpm)
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

fn score_duration(notes: &[ScheduledNote<ElectricGuitarArticulation>]) -> Duration {
    notes
        .iter()
        .map(|note| note.at.saturating_add(note.note.duration))
        .max()
        .unwrap_or(Duration::ZERO)
}

async fn play_lead_guitar_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let lead_guitar = ElectricGuitar::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(&lead_guitar, note, &mut metronome_sync).await?;
    }
    Ok(())
}

async fn play_backup_vocals_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let backup_vocals = Vocals::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &backup_vocals,
            with_articulation(note, VocalsArticulation::GroupHarmony),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_vocals_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let vocals = Vocals::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &vocals,
            with_articulation(note, VocalsArticulation::Clean),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_bass_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let bass = Bass::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &bass,
            with_articulation(note, BassArticulation::Picked),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_kick_drum_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let kick_drum = KickDrum::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &kick_drum,
            with_articulation(note, KickDrumArticulation::StandardHit),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_hi_hat_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let hi_hat = HiHat::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &hi_hat,
            with_articulation(note, HiHatArticulation::ClosedTip),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_cymbal_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let cymbal = Cymbal::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &cymbal,
            with_articulation(note, CymbalArticulation::StandardCrash),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_snare_drum_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let snare_drum = SnareDrum::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &snare_drum,
            with_articulation(note, SnareDrumArticulation::CenterHit),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

async fn play_toms_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let toms = Toms::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &toms,
            with_articulation(note, TomsArticulation::StandardHit),
            &mut metronome_sync,
        )
        .await?;
    }
    Ok(())
}

fn with_articulation<Articulation>(
    note: ScheduledNote<ElectricGuitarArticulation>,
    articulation: Articulation,
) -> ScheduledNote<Articulation> {
    ScheduledNote {
        at: note.at,
        note: Note {
            n_midi: note.note.n_midi,
            amplitude_multiplier: note.note.amplitude_multiplier,
            pan: note.note.pan,
            duration: note.note.duration,
            velocity: note.note.velocity,
            expression: note.note.expression,
            articulation,
        },
    }
}

async fn play_with_retry<I>(
    instrument: &I,
    scheduled_note: ScheduledNote<I::Articulation>,
    metronome_sync: &mut MetronomeSync,
) -> Result<(), InstrumentError>
where
    I: Instrument,
    I::Articulation: Copy,
{
    let requested_offset = scheduled_note.at;
    let note = scheduled_note.note;
    let note_midi = note.n_midi;
    let note_duration = note.duration;
    let beat_duration = metronome_sync.beat_duration();
    let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
    let target_instant = metronome_sync.target_instant(requested_offset);
    if target_instant > tokio::time::Instant::now() {
        tokio::time::sleep_until(target_instant).await;
    }
    let mut retry_count = 0_u32;
    let mut lateness = metronome_sync.elapsed().saturating_sub(requested_offset);

    if lateness > late_note_drop_threshold {
        debug!(
            midi = note_midi,
            lateness_ms = lateness.as_millis(),
            threshold_ms = late_note_drop_threshold.as_millis(),
            requested_offset_ms = requested_offset.as_millis(),
            duration_ms = note_duration.as_millis(),
            "dropping note that is too late to keep instrument phase aligned"
        );
        return Ok(());
    }

    // Bound retries to prevent queue backpressure from creating a long late-note backlog.
    loop {
        let play_started = Instant::now();
        let play_result = instrument.play(note).await;
        let play_elapsed = play_started.elapsed();
        if play_elapsed >= Duration::from_millis(250) {
            warn!(
                retries = retry_count,
                midi = note_midi,
                play_elapsed_ms = play_elapsed.as_millis(),
                requested_offset_ms = requested_offset.as_millis(),
                lateness_ms = lateness.as_millis(),
                duration_ms = note_duration.as_millis(),
                "instrument.play is slow; possible compute pressure or enqueue backpressure"
            );
        } else if play_elapsed >= Duration::from_millis(50) {
            debug!(
                retries = retry_count,
                midi = note_midi,
                play_elapsed_ms = play_elapsed.as_millis(),
                "instrument.play latency spike observed"
            );
        }

        match play_result {
            Ok(()) => {
                if retry_count > 0 {
                    debug!(
                        retries = retry_count,
                        midi = note_midi,
                        requested_offset_ms = requested_offset.as_millis(),
                        lateness_ms = lateness.as_millis(),
                        duration_ms = note_duration.as_millis(),
                        "note playback submission eventually succeeded after retries"
                    );
                }
                return Ok(());
            }
            Err(InstrumentError::Submission) => {
                retry_count = retry_count.saturating_add(1);

                lateness = metronome_sync.elapsed().saturating_sub(requested_offset);
                if retry_count >= MAX_SUBMISSION_ATTEMPTS || lateness > late_note_drop_threshold {
                    warn!(
                        retries = retry_count,
                        midi = note_midi,
                        play_elapsed_ms = play_elapsed.as_millis(),
                        requested_offset_ms = requested_offset.as_millis(),
                        lateness_ms = lateness.as_millis(),
                        threshold_ms = late_note_drop_threshold.as_millis(),
                        duration_ms = note_duration.as_millis(),
                        "dropping note after bounded submission retries to avoid runtime starvation"
                    );
                    return Ok(());
                }

                debug!(
                    retries = retry_count,
                    midi = note_midi,
                    play_elapsed_ms = play_elapsed.as_millis(),
                    requested_offset_ms = requested_offset.as_millis(),
                    lateness_ms = lateness.as_millis(),
                    duration_ms = note_duration.as_millis(),
                    "note playback submission failed; retrying with bounded backoff"
                );
                let backoff_ms = (1_u64 << retry_count.min(4)).min(16);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
            Err(err) => {
                error!(
                    error = %err,
                    midi = note_midi,
                    requested_offset_ms = requested_offset.as_millis(),
                    lateness_ms = lateness.as_millis(),
                    duration_ms = note_duration.as_millis(),
                    "non-retryable instrument playback error"
                );
                return Err(err);
            }
        }
    }
}

fn late_note_drop_threshold(beat: Duration) -> Duration {
    beat.div_f32(4.0)
        .clamp(MIN_LATE_NOTE_DROP_THRESHOLD, MAX_LATE_NOTE_DROP_THRESHOLD)
}
