use crate::{AudioHandle, PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct ElectricGuitar {
    audio: AudioHandle,
    synth: SynthHandle,
}

impl ElectricGuitar {
    pub fn new(audio: AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }

    pub fn audio(&self) -> &AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ElectricGuitarArticulation {
    /// Standard picked note with normal sustain and release.
    Sustained,
    /// A sustained lead-guitar chord voice.
    RhythmSustained,
}

impl Default for ElectricGuitarArticulation {
    fn default() -> Self {
        Self::RhythmSustained
    }
}

impl Instrument for ElectricGuitar {
    type Articulation = ElectricGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate, pan) = self.synth.electric_guitar(note).await?;

        let mut request = PlayRequest::new(pcm, 1, crate::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = pan;
        request.priority = PlayPriority::Low;

        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}
