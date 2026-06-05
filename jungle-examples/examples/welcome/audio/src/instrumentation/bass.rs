use crate::{AudioHandle, PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Bass {
    audio: AudioHandle,
    synth: SynthHandle,
}

impl Bass {
    pub fn new(audio: AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BassArticulation {
    /// A hard, aggressive pick strike with normal sustain.
    Picked,
}

impl Default for BassArticulation {
    fn default() -> Self {
        Self::Picked
    }
}

impl Instrument for Bass {
    type Articulation = BassArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = self.synth.bass(note).await?;

        let mut request = PlayRequest::new(pcm, 1, crate::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        request.priority = PlayPriority::Normal;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}
