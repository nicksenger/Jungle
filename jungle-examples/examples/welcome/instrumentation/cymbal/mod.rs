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
    /// Grabbing the cymbal with the hand immediately after striking to choke the sound.
    ChokedCrash,
    /// Striking the flat surface of the ride cymbal with the tip of the stick for a clear ping.
    RideTip,
    /// Striking the dome/bell of the ride cymbal.
    /// Adds distinct, bright, metallic punctuation to specific grooves.
    RideBell,
}

impl Instrument for Cymbal {
    type Articulation = CymbalArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}
