use jungle_sdk::prelude::*;

use crate::effect::Monad;

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

pub struct Sing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32> Act
    for Sing<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<Vocals, VocalsArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(state: &VocalsArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut VocalsArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
