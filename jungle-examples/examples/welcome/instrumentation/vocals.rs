use jungle_sdk::prelude::*;
use std::{sync::Arc, time::Duration};
use welcome_audio::{PlayPriority, PlayRequest};

use crate::{animals::LeadVocalistState, effect::Monad};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Vocals {
    audio: welcome_audio::AudioHandle,
    synth: SynthHandle,
}

impl Vocals {
    pub fn new(audio: welcome_audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum VocalsArticulation {
    /// Clean, melodic singing with standard resonance (e.g., the lower register parts of the verses).
    Clean,
    /// Clean, unified group harmony backing up a lead line.
    GroupHarmony,
    /// Formant
    Formant([Option<Phoneme>; 12]),
}

impl Default for VocalsArticulation {
    fn default() -> Self {
        Self::Clean
    }
}

impl Instrument for Vocals {
    type Articulation = VocalsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = self.synth.vocals(note).await?;

        for layer in
            welcome_audio::dsp::vocals::articulation_layers(to_dsp_articulation(note.articulation))
        {
            if layer.delay_seconds > 0.0 {
                tokio::time::sleep(Duration::from_secs_f32(layer.delay_seconds)).await;
            }
            let mut request =
                PlayRequest::new(Arc::clone(&pcm), 1, welcome_audio::dsp::SAMPLE_RATE);
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

pub type Phoneme = welcome_audio::vocals::Phoneme;

pub fn phonemes_from_text(text: &str) -> [Option<Phoneme>; 12] {
    welcome_audio::vocals::phonemes_from_text(text)
}

fn to_dsp_articulation(
    articulation: VocalsArticulation,
) -> welcome_audio::dsp::vocals::VocalsArticulation {
    match articulation {
        VocalsArticulation::Clean => welcome_audio::dsp::vocals::VocalsArticulation::Clean,
        VocalsArticulation::GroupHarmony => {
            welcome_audio::dsp::vocals::VocalsArticulation::GroupHarmony
        }
        VocalsArticulation::Formant(phonemes) => {
            welcome_audio::dsp::vocals::VocalsArticulation::Formant(phonemes)
        }
    }
}

pub struct Generate<
    const NOTE: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u8 = 0,
>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Act
    for Generate<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<Vocals, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        VocalsArticulation::Formant(
            state
                .lyrics
                .phonemes
                .last()
                .copied()
                .unwrap_or_else(|| [None; 12]),
        )
    }

    fn absorb(
        state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
        let _ = state.lyrics.phonemes.pop();
    }
}
