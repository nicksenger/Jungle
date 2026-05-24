use super::{Instrument, Note, SynthHandle};

pub(super) mod audio;

pub struct KickDrum {
    audio: crate::audio::AudioHandle,
    synth: SynthHandle,
}

impl KickDrum {
    pub fn new(audio: crate::audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum KickDrumArticulation {
    /// A standard, powerful kick where the beater strikes and bounces off.
    StandardHit,
}

impl Instrument for KickDrum {
    type Articulation = KickDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, &self.synth, note).await
    }
}
