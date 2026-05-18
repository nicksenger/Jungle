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
    score::{distortion_guitar_score, electric_guitar_score},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");
    let audio_engine = AudioEngine::start_default().await?;
    let electric_notes = electric_guitar_score();
    let distortion_notes = distortion_guitar_score();

    let total_duration = score_duration(&electric_notes).max(score_duration(&distortion_notes));

    let electric_task = tokio::spawn(play_lead_guitar_score(audio_engine.handle(), electric_notes));
    let distortion_task =
        tokio::spawn(play_lead_guitar_score(audio_engine.handle(), distortion_notes));

    electric_task.await??;
    distortion_task.await??;

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
