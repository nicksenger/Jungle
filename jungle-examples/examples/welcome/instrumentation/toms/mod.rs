use super::{Instrument, Note, SynthHandle};

pub(super) mod audio;

pub struct Toms {
    audio: crate::audio::AudioHandle,
    synth: SynthHandle,
}

impl Toms {
    pub fn new(audio: crate::audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TomsArticulation {
    /// A clean, resonant strike to the center of the tom.
    StandardHit,
}

impl Instrument for Toms {
    type Articulation = TomsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, &self.synth, note).await
    }
}
