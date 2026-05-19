mod animals;
mod assets;
mod audio;
mod effects;
mod flow;
mod instrumentation;
mod score;
mod ui;

use std::time::Duration;

use crate::{
    audio::{AudioEngine, AudioHandle},
    instrumentation::{
        BackupVocals, BackupVocalsArticulation, Bass, BassArticulation, Cymbal, CymbalArticulation,
        Error as InstrumentError, HiHat, HiHatArticulation, Instrument, KickDrum,
        KickDrumArticulation, LeadGuitar, LeadGuitarArticulation, Note, RhythmGuitar,
        RhythmGuitarArticulation, SnareDrum, SnareDrumArticulation, Toms, TomsArticulation, Vocals,
        VocalsArticulation,
    },
    score::{
        backup_vocals_score, bass_drum_score, bass_guitar_score, closed_hi_hat_cymbal_score,
        crash_cymbal_score, lead_guitar_score, rhythm_guitar_score, snare_drum_score,
        toms_snare_score, vocals_score,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");
    let audio_engine = AudioEngine::start_default().await?;

    let lead_guitar = lead_guitar_score();
    let rhythm_guitar = rhythm_guitar_score();
    let backup_vocals = backup_vocals_score();
    let vocals = vocals_score();
    let bass = bass_guitar_score();
    let kick_drum = bass_drum_score();
    let hi_hat = closed_hi_hat_cymbal_score();
    let cymbal = crash_cymbal_score();
    let snare_drum = snare_drum_score();
    let toms = toms_snare_score();

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
    )));
    tasks.push(tokio::spawn(play_rhythm_guitar_score(
        audio_engine.handle(),
        rhythm_guitar,
    )));
    tasks.push(tokio::spawn(play_backup_vocals_score(
        audio_engine.handle(),
        backup_vocals,
    )));
    tasks.push(tokio::spawn(play_vocals_score(
        audio_engine.handle(),
        vocals,
    )));
    tasks.push(tokio::spawn(play_bass_score(audio_engine.handle(), bass)));
    tasks.push(tokio::spawn(play_kick_drum_score(
        audio_engine.handle(),
        kick_drum,
    )));
    tasks.push(tokio::spawn(play_hi_hat_score(
        audio_engine.handle(),
        hi_hat,
    )));
    tasks.push(tokio::spawn(play_cymbal_score(
        audio_engine.handle(),
        cymbal,
    )));
    tasks.push(tokio::spawn(play_snare_drum_score(
        audio_engine.handle(),
        snare_drum,
    )));
    tasks.push(tokio::spawn(play_toms_score(audio_engine.handle(), toms)));

    for task in tasks {
        task.await??;
    }

    tokio::time::sleep(total_duration.saturating_add(Duration::from_secs(1))).await;
    Ok(())
}

fn score_duration(notes: &[Note<LeadGuitarArticulation>]) -> Duration {
    notes
        .iter()
        .map(|note| note.offset.saturating_add(note.duration))
        .max()
        .unwrap_or(Duration::ZERO)
}

async fn play_lead_guitar_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let lead_guitar = LeadGuitar::new(audio_handle);
    for note in notes {
        play_with_retry(&lead_guitar, note).await?;
    }
    Ok(())
}

async fn play_rhythm_guitar_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let rhythm_guitar = RhythmGuitar::new(audio_handle);
    for note in notes {
        play_with_retry(
            &rhythm_guitar,
            with_articulation(note, RhythmGuitarArticulation::Sustained),
        )
        .await?;
    }
    Ok(())
}

async fn play_backup_vocals_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let backup_vocals = BackupVocals::new(audio_handle);
    for note in notes {
        play_with_retry(
            &backup_vocals,
            with_articulation(note, BackupVocalsArticulation::GroupHarmony),
        )
        .await?;
    }
    Ok(())
}

async fn play_vocals_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let vocals = Vocals::new(audio_handle);
    for note in notes {
        play_with_retry(&vocals, with_articulation(note, VocalsArticulation::Clean)).await?;
    }
    Ok(())
}

async fn play_bass_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let bass = Bass::new(audio_handle);
    for note in notes {
        play_with_retry(&bass, with_articulation(note, BassArticulation::Picked)).await?;
    }
    Ok(())
}

async fn play_kick_drum_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let kick_drum = KickDrum::new(audio_handle);
    for note in notes {
        play_with_retry(
            &kick_drum,
            with_articulation(note, KickDrumArticulation::StandardHit),
        )
        .await?;
    }
    Ok(())
}

async fn play_hi_hat_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let hi_hat = HiHat::new(audio_handle);
    for note in notes {
        play_with_retry(
            &hi_hat,
            with_articulation(note, HiHatArticulation::ClosedTip),
        )
        .await?;
    }
    Ok(())
}

async fn play_cymbal_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let cymbal = Cymbal::new(audio_handle);
    for note in notes {
        play_with_retry(
            &cymbal,
            with_articulation(note, CymbalArticulation::StandardCrash),
        )
        .await?;
    }
    Ok(())
}

async fn play_snare_drum_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let snare_drum = SnareDrum::new(audio_handle);
    for note in notes {
        play_with_retry(
            &snare_drum,
            with_articulation(note, SnareDrumArticulation::CenterHit),
        )
        .await?;
    }
    Ok(())
}

async fn play_toms_score(
    audio_handle: AudioHandle,
    notes: Vec<Note<LeadGuitarArticulation>>,
) -> Result<(), InstrumentError> {
    let toms = Toms::new(audio_handle);
    for note in notes {
        play_with_retry(
            &toms,
            with_articulation(note, TomsArticulation::StandardHit),
        )
        .await?;
    }
    Ok(())
}

fn with_articulation<Articulation>(
    note: Note<LeadGuitarArticulation>,
    articulation: Articulation,
) -> Note<Articulation> {
    Note {
        n_midi: note.n_midi,
        duration: note.duration,
        velocity: note.velocity,
        expression: note.expression,
        offset: note.offset,
        articulation,
    }
}

async fn play_with_retry<I>(
    instrument: &I,
    note: Note<I::Articulation>,
) -> Result<(), InstrumentError>
where
    I: Instrument,
    I::Articulation: Copy,
{
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
