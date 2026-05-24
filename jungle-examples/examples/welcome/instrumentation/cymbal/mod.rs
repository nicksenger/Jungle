use super::{Instrument, Note};

mod audio;

pub struct Cymbal {
    audio: crate::audio::AudioHandle,
}

impl Cymbal {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
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
        audio::play(&self.audio, note).await
    }
}
