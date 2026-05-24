use super::{Instrument, Note};

mod audio;

pub struct SnareDrum {
    audio: crate::audio::AudioHandle,
}

impl SnareDrum {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SnareDrumArticulation {
    /// Striking the center of the head and the metal rim simultaneously.
    /// This is the primary articulation for the massive verse/chorus backbeats.
    Rimshot,
}

impl Instrument for SnareDrum {
    type Articulation = SnareDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}
