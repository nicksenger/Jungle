use std::{sync::Arc, time::Duration};

use crate::{AudioHandle, PlayPriority, PlayRequest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Vocals {
    audio: AudioHandle,
    synth: SynthHandle,
}

impl Vocals {
    pub fn new(audio: AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum VocalsArticulation {
    /// Unified group harmony backing up a lead line.
    GroupHarmony,
    /// Formant
    Formant([Option<Phoneme>; 12]),
}

impl Default for VocalsArticulation {
    fn default() -> Self {
        Self::GroupHarmony
    }
}

impl Instrument for Vocals {
    type Articulation = VocalsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = self.synth.vocals(note).await?;

        for layer in crate::dsp::vocals::articulation_layers(to_dsp_articulation(note.articulation))
        {
            if layer.delay_seconds > 0.0 {
                tokio::time::sleep(Duration::from_secs_f32(layer.delay_seconds)).await;
            }
            let mut request = PlayRequest::new(Arc::clone(&pcm), 1, crate::dsp::SAMPLE_RATE);
            request.gain = gain * layer.gain_scale * amplitude_gain(&note);
            request.playback_rate = playback_rate * layer.playback_rate_scale;
            request.pan = layer.pan;
            request.priority = PlayPriority::Low;
            self.audio
                .play(request)
                .await
                .map_err(|_| Error::Submission)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lyrics {
    pub phonemes: Vec<[Option<Phoneme>; 12]>,
}

pub type Phoneme = crate::vocals::Phoneme;

pub fn phonemes_from_text(text: &str) -> [Option<Phoneme>; 12] {
    crate::vocals::phonemes_from_text(text)
}

pub(crate) fn to_dsp_articulation(
    articulation: VocalsArticulation,
) -> crate::dsp::vocals::VocalsArticulation {
    match articulation {
        VocalsArticulation::GroupHarmony => crate::dsp::vocals::VocalsArticulation::GroupHarmony,
        VocalsArticulation::Formant(phonemes) => {
            crate::dsp::vocals::VocalsArticulation::Formant(phonemes)
        }
    }
}
