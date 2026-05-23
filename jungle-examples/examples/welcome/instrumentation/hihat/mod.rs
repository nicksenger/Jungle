use super::{Instrument, Note};

mod audio;

pub struct HiHat {
    audio: crate::audio::AudioHandle,
}

impl HiHat {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum HiHatArticulation {
    /// Fully pressed closed, yielding a tight, crisp "chick" sound.
    ClosedTip,
}

impl Instrument for HiHat {
    type Articulation = HiHatArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}
