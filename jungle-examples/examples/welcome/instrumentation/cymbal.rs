use welcome_audio::{PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Cymbal {
    audio: welcome_audio::AudioHandle,
    synth: SynthHandle,
}

impl Cymbal {
    pub fn new(audio: welcome_audio::AudioHandle, synth: SynthHandle) -> Self {
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

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = self.synth.cymbal(note).await?;

        let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.25;
        request.priority = PlayPriority::Normal;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}
