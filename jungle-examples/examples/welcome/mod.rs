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
        toms_snare_score, vocals_score,
    },
};

const DEFAULT_BPM: f32 = 123.0;
const BEATS_PER_BAR: u32 = 4;
const UI_MIN_UPTIME_BEFORE_SHUTDOWN: Duration = Duration::from_secs(5 * 60);

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bpm = parse_bpm_arg()?;
    let client = LocalClient::builder().build().await?;
    let worker_client = client.clone();
    let _worker_task = tokio::spawn(async move {
        let worker = JungleWorker::new(WelcomeEcosystem, worker_client);
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&())?;
    let journeys = ui::JourneyIds {
        lead_vocalist: client.start_journey::<LeadVocalist>(seed.clone()).await?,
        lead_guitarist: client.start_journey::<LeadGuitarist>(seed.clone()).await?,
        rhythm_guitarist: client
            .start_journey::<RhythmGuitarist>(seed.clone())
            .await?,
        bass: client.start_journey::<BassAnimal>(seed.clone()).await?,
        drums: client.start_journey::<Drums>(seed).await?,
    };
    let ui_shutdown = ui::ShutdownFlag::new();
    let ui_started_at = Instant::now();
    let ui_thread = ui::spawn_ui(client.clone(), journeys, ui_shutdown.clone());

    let audio_engine = AudioEngine::start_default().await?;
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

    let mut tasks = Vec::with_capacity(10);
    tasks.push(tokio::spawn(play_lead_guitar_score(
        audio_engine.handle(),
        lead_guitar,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_rhythm_guitar_score(
        audio_engine.handle(),
        rhythm_guitar,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_backup_vocals_score(
        audio_engine.handle(),
        backup_vocals,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_vocals_score(
        audio_engine.handle(),
        vocals,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_bass_score(
        audio_engine.handle(),
        bass,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_kick_drum_score(
        audio_engine.handle(),
        kick_drum,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_hi_hat_score(
        audio_engine.handle(),
        hi_hat,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_cymbal_score(
        audio_engine.handle(),
        cymbal,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_snare_drum_score(
        audio_engine.handle(),
        snare_drum,
        metronome.clone(),
    )));
    tasks.push(tokio::spawn(play_toms_score(
        audio_engine.handle(),
        toms,
        metronome,
    )));

    for task in tasks {
        task.await??;
    }

    tokio::time::sleep(total_duration.saturating_add(Duration::from_secs(1))).await;
    let elapsed_since_ui_start = ui_started_at.elapsed();
    if elapsed_since_ui_start < UI_MIN_UPTIME_BEFORE_SHUTDOWN {
        tokio::time::sleep(UI_MIN_UPTIME_BEFORE_SHUTDOWN - elapsed_since_ui_start).await;
    }
    ui_shutdown.request_shutdown();
    let _ = ui_thread.join();
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

fn score_duration(notes: &[Note<ElectricGuitarArticulation>]) -> Duration {
    notes
        .iter()
        .map(|note| note.offset.saturating_add(note.duration))
        .max()
        .unwrap_or(Duration::ZERO)
}

async fn play_lead_guitar_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    notes: Vec<Note<ElectricGuitarArticulation>>,
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
    note: Note<ElectricGuitarArticulation>,
    articulation: Articulation,
) -> Note<Articulation> {
    Note {
        n_midi: note.n_midi,
        amplitude_multiplier: note.amplitude_multiplier,
        pan: note.pan,
        duration: note.duration,
        velocity: note.velocity,
        expression: note.expression,
        offset: note.offset,
        articulation,
    }
}

async fn play_with_retry<I>(
    instrument: &I,
    mut note: Note<I::Articulation>,
    metronome_sync: &mut MetronomeSync,
) -> Result<(), InstrumentError>
where
    I: Instrument,
    I::Articulation: Copy,
{
    note.offset = metronome_sync.synchronize(note.offset).await;

    // Submitting a dense score can temporarily saturate the mixer queue.
    // Retry with a brief backoff instead of dropping notes.
    loop {
        match instrument.play(note).await {
            Ok(()) => return Ok(()),
            Err(InstrumentError::Submission) => tokio::time::sleep(Duration::from_millis(1)).await,
            Err(err) => return Err(err),
        }
    }
}
