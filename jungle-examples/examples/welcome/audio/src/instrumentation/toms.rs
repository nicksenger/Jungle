use crate::{AudioHandle, PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Toms {
    audio: AudioHandle,
    synth: SynthHandle,
}

impl Toms {
    pub fn new(audio: AudioHandle, synth: SynthHandle) -> Self {
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

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, mut gain, mut playback_rate) = self.synth.toms(note).await?;

        let velocity = note.velocity.clamp(0.0, 1.0);
        gain *= 0.86 + velocity * 0.42;
        playback_rate *= 0.985 + velocity * 0.045;

        let mut request = PlayRequest::new(pcm, 1, crate::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = -0.14 + (velocity - 0.5) * 0.08;
        request.priority = PlayPriority::Normal;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}
