use super::{Instrument, Note};

mod audio;

pub struct Toms {
    audio: crate::audio::AudioHandle,
}

impl Toms {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
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
        audio::play(&self.audio, note).await
    }
}
