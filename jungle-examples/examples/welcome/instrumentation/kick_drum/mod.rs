use super::{Instrument, Note};

mod audio;

pub struct KickDrum {
    audio: crate::audio::AudioHandle,
}

impl KickDrum {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum KickDrumArticulation {
    /// A standard, powerful kick where the beater strikes and bounces off.
    StandardHit,
    /// Burying the beater into the head, dampening the sustain for a tighter, punchier thud.
    BuriedBeater,
    /// A soft, unaccented hit used in quick double-stroke patterns.
    GhostHit,
}

impl Instrument for KickDrum {
    type Articulation = KickDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}
