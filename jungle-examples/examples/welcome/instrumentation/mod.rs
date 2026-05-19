use std::time::Duration;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
mod bass;
#[allow(dead_code)]
mod backup_vocals;
#[allow(dead_code)]
mod cymbal;
#[allow(dead_code)]
mod hihat;
#[allow(dead_code)]
mod kick_drum;
mod lead_guitar;
#[allow(dead_code)]
mod rhythm_guitar;
#[allow(dead_code)]
mod snare_drum;
#[allow(dead_code)]
mod synthesis;
#[allow(dead_code)]
mod toms;
#[allow(dead_code)]
mod vocals;

pub use lead_guitar::{LeadGuitar, LeadGuitarArticulation};

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
