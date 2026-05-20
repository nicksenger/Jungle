mod animals;
mod assets;
mod audio;
mod effects;
mod instrumentation;
mod metronome;
mod score;
mod ui;

use std::time::{Duration, Instant};

use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{JungleClient, LocalClient};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

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
    score::{
        backup_vocals_score, bass_drum_score, bass_guitar_score, closed_hi_hat_cymbal_score,
        crash_cymbal_score, lead_guitar_score, rhythm_guitar_score, snare_drum_score,
        toms_snare_score, vocals_score, ScheduledNote,
    },
};

const DEFAULT_BPM: f32 = 123.0;
const BEATS_PER_BAR: u32 = 4;
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

struct WelcomeEcosystem;
impl Ecosystem for WelcomeEcosystem {
    const NAME: &'static str = "welcome";
    type Animals = WelcomeAnimals;
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
    client: LocalClient,
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
        let client = LocalClient::builder().build().await.map_err(|err| {
            error!(error = %err, "failed building local client");
            err.to_string()
        })?;
        let worker_client = client.clone();
        let _worker_task = tokio::spawn(async move {
            let worker = JungleWorker::new(WelcomeEcosystem, worker_client);
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

        Ok::<UiSetup, String>(UiSetup { client, journeys })
    });

    if let Err(err) = setup_tx.send(setup) {
        error!(error = %err, "failed sending UI setup to main thread");
        return Err(format!("failed to send UI setup to main thread: {err}"));
    }

    let ui_started_at = started_rx.recv().map_err(|err| {
        error!(
            error = %err,
            "failed receiving UI start signal from main thread"
        );
        format!("failed to receive UI start signal from main thread: {err}")
    })?;

    runtime.block_on(play_audio_and_schedule_shutdown(
        bpm,
        ui_shutdown,
        ui_started_at,
    ))
}

async fn play_audio_and_schedule_shutdown(
    bpm: f32,
    ui_shutdown: ui::ShutdownFlag,
    ui_started_at: Instant,
) -> Result<(), String> {
    let audio_engine = AudioEngine::start_default().await.map_err(|err| {
        error!(error = %err, "failed starting audio engine");
        err.to_string()
    })?;
    let metronome = Metronome::spawn(bpm, BEATS_PER_BAR);

    let lead_guitar = lead_guitar_score(bpm);
    let rhythm_guitar = rhythm_guitar_score(bpm);
    let backup_vocals = backup_vocals_score(bpm);
    let vocals = vocals_score(bpm);
    let bass = bass_guitar_score(bpm);
    let kick_drum = bass_drum_score(bpm);
    let hi_hat = closed_hi_hat_cymbal_score(bpm);
    let cymbal = crash_cymbal_score(bpm);
    let snare_drum = snare_drum_score(bpm);
    let toms = toms_snare_score(bpm);

    let total_duration = [
        lead_guitar.as_slice(),
        rhythm_guitar.as_slice(),
        backup_vocals.as_slice(),
        vocals.as_slice(),
        bass.as_slice(),
        kick_drum.as_slice(),
        hi_hat.as_slice(),
        cymbal.as_slice(),
        snare_drum.as_slice(),
        toms.as_slice(),
    ]
    .into_iter()
    .map(score_duration)
    .max()
    .unwrap_or(Duration::ZERO);

    info!(bpm, ?total_duration, "starting welcome score playback");
    let mut tasks = Vec::with_capacity(10);
    tasks.push((
        "lead_guitar",
        tokio::spawn(play_lead_guitar_score(
            audio_engine.handle(),
            lead_guitar,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "rhythm_guitar",
        tokio::spawn(play_rhythm_guitar_score(
            audio_engine.handle(),
            rhythm_guitar,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "backup_vocals",
        tokio::spawn(play_backup_vocals_score(
            audio_engine.handle(),
            backup_vocals,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "vocals",
        tokio::spawn(play_vocals_score(
            audio_engine.handle(),
            vocals,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "bass",
        tokio::spawn(play_bass_score(
            audio_engine.handle(),
            bass,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "kick_drum",
        tokio::spawn(play_kick_drum_score(
            audio_engine.handle(),
            kick_drum,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "hi_hat",
        tokio::spawn(play_hi_hat_score(
            audio_engine.handle(),
            hi_hat,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "cymbal",
        tokio::spawn(play_cymbal_score(
            audio_engine.handle(),
            cymbal,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "snare_drum",
        tokio::spawn(play_snare_drum_score(
            audio_engine.handle(),
            snare_drum,
            metronome.clone(),
        )),
    ));
    tasks.push((
        "toms",
        tokio::spawn(play_toms_score(audio_engine.handle(), toms, metronome)),
    ));

    for (name, task) in tasks {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                error!(instrument = name, error = %err, "instrument playback task failed");
                return Err(err.to_string());
            }
            Err(err) => {
                error!(instrument = name, error = %err, "instrument playback task panicked");
                return Err(err.to_string());
            }
        }
    }

    tokio::time::sleep(total_duration.saturating_add(Duration::from_secs(1))).await;
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

async fn play_rhythm_guitar_score(
    audio_handle: AudioHandle,
    notes: Vec<ScheduledNote<ElectricGuitarArticulation>>,
    metronome: Metronome,
) -> Result<(), InstrumentError> {
    let rhythm_guitar = ElectricGuitar::new(audio_handle);
    let mut metronome_sync = metronome.subscribe();
    for note in notes {
        play_with_retry(
            &rhythm_guitar,
            with_articulation(note, ElectricGuitarArticulation::RhythmSustained),
            &mut metronome_sync,
        )
        .await?;
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
    if target_instant > Instant::now() {
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
