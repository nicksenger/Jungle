use std::time::Duration;

use serde::{Deserialize, Serialize};

mod bass;
mod kick_drum;
mod lead_guitar;
mod rhythm_guitar;
mod snare_drum;
mod synth;
mod tambourine;
mod vocals;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Note playback submission failed.")]
    Submission,
    #[error("Note playback failed.")]
    Playback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Note<Articulation> {
    pub n_midi: u8,
    pub duration: Duration,
    pub velocity: f32,
    pub expression: Option<Expression>,
    pub offset: Duration,
    pub articulation: Articulation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Expression {
    pub bend: f32,
    pub vibrato: f32,
}

pub trait Instrument {
    type Articulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error>;
}
