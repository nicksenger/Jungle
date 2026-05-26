use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{animals::LeadVocalistState, effect::Monad};

use super::{Instrument, Note, SynthHandle};

pub(super) mod audio;

pub struct Vocals {
    audio: crate::audio::AudioHandle,
    synth: SynthHandle,
}

impl Vocals {
    pub fn new(audio: crate::audio::AudioHandle, synth: SynthHandle) -> Self {
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

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, &self.synth, note).await
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lyrics {
    pub phonemes: Vec<[Option<Phoneme>; 12]>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Phoneme {
    pub length: u8,
    pub index: usize,
    pub stress: u8,
}

pub fn phonemes_from_text(text: &str) -> [Option<Phoneme>; 12] {
    let mut output = [None; 12];

    let parsed_text = audio::text_to_phonemes(text);

    let parsed_phonemes = match audio::parse_phonemes(&parsed_text) {
        Ok(parsed_phonemes) => parsed_phonemes,
        Err(err) => {
            warn!(word = text, error = %err, "failed to parse rustsam phoneme string");
            return output;
        }
    };

    if parsed_phonemes.len() > output.len() {
        warn!(
            word = text,
            parsed_count = parsed_phonemes.len(),
            max_count = output.len(),
            "truncating parsed rustsam phonemes to fit vocals articulation capacity"
        );
    }

    for (slot, phoneme) in output.iter_mut().zip(parsed_phonemes.into_iter()) {
        *slot = Some(Phoneme {
            length: phoneme.length,
            index: phoneme.index,
            stress: phoneme.stress,
        });
    }

    output
}

pub struct Generate<
    const NOTE: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u32 = 0,
>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32> Act
    for Generate<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<Vocals, VocalsArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
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
