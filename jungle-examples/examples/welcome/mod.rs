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
        Error as InstrumentError, Instrument, LeadGuitar, LeadGuitarArticulation, Note,
    },
    score::{
        bass_drum_score, bass_guitar_score, closed_hi_hat_cymbal_score, crash_cymbal_score,
        distortion_guitar_score, electric_guitar_score, flute_score, saxophone_score,
        snare_drum_score, toms_snare_score,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");
    let audio_engine = AudioEngine::start_default().await?;
    let instrument_scores = vec![
        electric_guitar_score(),
        distortion_guitar_score(),
        saxophone_score(),
        flute_score(),
        bass_guitar_score(),
        bass_drum_score(),
        closed_hi_hat_cymbal_score(),
        crash_cymbal_score(),
        snare_drum_score(),
        toms_snare_score(),
    ];

    let total_duration = instrument_scores
        .iter()
        .map(|notes| score_duration(notes))
        .max()
        .unwrap_or(Duration::ZERO);

    let mut tasks = Vec::with_capacity(instrument_scores.len());
    for notes in instrument_scores {
        tasks.push(tokio::spawn(play_lead_guitar_score(audio_engine.handle(), notes)));
    }

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
        // Submitting a dense score can temporarily saturate the mixer queue.
        // Retry with a brief backoff instead of dropping notes.
        loop {
            match lead_guitar.play(note).await {
                Ok(()) => break,
                Err(InstrumentError::Submission) => tokio::time::sleep(Duration::from_millis(1)).await,
                Err(err) => return Err(err),
            }
        }
    }
    Ok(())
}
