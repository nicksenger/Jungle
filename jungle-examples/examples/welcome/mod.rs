mod animals;
mod assets;
mod audio;
mod effects;
mod flow;
mod instrumentation;
mod ui;

use std::time::Duration;

use crate::{
    audio::AudioEngine,
    instrumentation::{Instrument, LeadGuitar, LeadGuitarArticulation, Note},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _viewer = jungle_viewer::JungleViewerBuilder::new().title("Welcome Example");
    let audio_engine = AudioEngine::start_default().await?;
    let lead_guitar = LeadGuitar::new(audio_engine.handle());

    let notes = [
        (
            58,
            LeadGuitarArticulation::Sustained,
            Duration::from_millis(850),
            0.8,
        ),
        (
            56,
            LeadGuitarArticulation::PalmMuted,
            Duration::from_millis(420),
            0.75,
        ),
        (
            53,
            LeadGuitarArticulation::HammerOn,
            Duration::from_millis(560),
            0.78,
        ),
        (
            51,
            LeadGuitarArticulation::PullOff,
            Duration::from_millis(500),
            0.72,
        ),
        (
            49,
            LeadGuitarArticulation::Slide,
            Duration::from_millis(780),
            0.84,
        ),
        (
            46,
            LeadGuitarArticulation::PinchHarmonic,
            Duration::from_millis(620),
            0.9,
        ),
    ];

    for (n_midi, articulation, duration, velocity) in notes {
        let note = Note {
            n_midi,
            duration,
            velocity,
            expression: None,
            offset: Duration::ZERO,
            articulation,
        };
        lead_guitar.play(note).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
