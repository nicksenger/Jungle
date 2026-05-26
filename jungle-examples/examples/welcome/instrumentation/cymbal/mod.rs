use super::{Instrument, Note, SynthHandle};

pub(super) mod audio;

pub struct Cymbal {
    audio: crate::audio::AudioHandle,
    synth: SynthHandle,
}

impl Cymbal {
    pub fn new(audio: crate::audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CymbalArticulation {
    /// A standard, explosive crash on the edge of the cymbal.
    StandardCrash,
}

impl Instrument for Cymbal {
    type Articulation = CymbalArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, &self.synth, note).await
    }
}
