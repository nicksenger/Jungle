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
    audio::AudioEngine,
    instrumentation::{Error as InstrumentError, Instrument, LeadGuitar},
    score::electric_guitar_score,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");
    let audio_engine = AudioEngine::start_default().await?;
    let lead_guitar = LeadGuitar::new(audio_engine.handle());
    let notes = electric_guitar_score();
    let total_duration = notes
        .iter()
        .map(|note| note.offset.saturating_add(note.duration))
        .max()
        .unwrap_or(Duration::ZERO);

    for note in notes {
        // Submitting the full score can temporarily saturate the mixer queue.
        // Retry with brief backoff instead of dropping notes.
        loop {
            match lead_guitar.play(note).await {
                Ok(()) => break,
                Err(InstrumentError::Submission) => tokio::time::sleep(Duration::from_millis(1)).await,
                Err(err) => return Err(err.into()),
            }
        }
    }

    tokio::time::sleep(total_duration.saturating_add(Duration::from_secs(1))).await;
    Ok(())
}
