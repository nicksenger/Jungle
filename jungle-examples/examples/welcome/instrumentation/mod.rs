use std::time::Duration;

use serde::{Deserialize, Serialize};

mod backup_vocals;
mod bass;
mod cymbal;
mod hihat;
mod kick_drum;
mod lead_guitar;
mod rhythm_guitar;
mod snare_drum;
mod synthesis;
mod toms;
mod vocals;

pub use backup_vocals::{BackupVocals, BackupVocalsArticulation};
pub use bass::{Bass, BassArticulation};
pub use cymbal::{Cymbal, CymbalArticulation};
pub use hihat::{HiHat, HiHatArticulation};
pub use kick_drum::{KickDrum, KickDrumArticulation};
pub use lead_guitar::{LeadGuitar, LeadGuitarArticulation};
pub use rhythm_guitar::{RhythmGuitar, RhythmGuitarArticulation};
pub use snare_drum::{SnareDrum, SnareDrumArticulation};
pub use toms::{Toms, TomsArticulation};
pub use vocals::{Vocals, VocalsArticulation};

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
    /// `0.5` is neutral loudness. Lower values are quieter, higher values are louder.
    pub amplitude_multiplier: f32,
    pub duration: Duration,
    pub velocity: f32,
    pub expression: Option<Expression>,
    pub offset: Duration,
    pub articulation: Articulation,
}

pub const NORMAL_AMPLITUDE_MULTIPLIER: f32 = 0.5;

pub fn amplitude_gain<Articulation>(note: &Note<Articulation>) -> f32 {
    let amplitude = if note.amplitude_multiplier.is_finite() {
        note.amplitude_multiplier.clamp(0.0, 1.0)
    } else {
        NORMAL_AMPLITUDE_MULTIPLIER
    };
    amplitude * 2.0
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
