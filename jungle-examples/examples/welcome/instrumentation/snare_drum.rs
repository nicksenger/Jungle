use welcome_audio::{PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct SnareDrum {
    audio: welcome_audio::AudioHandle,
    synth: SynthHandle,
}

impl SnareDrum {
    pub fn new(audio: welcome_audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
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

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, mut gain, mut playback_rate) = self.synth.snare_drum(note).await?;

        let velocity = note.velocity.clamp(0.0, 1.0);
        gain *= 0.88 + velocity * 0.52;
        playback_rate *= 0.98 + velocity * 0.06;

        let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.08 + (velocity - 0.5) * 0.06;
        request.priority = PlayPriority::Critical;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}
