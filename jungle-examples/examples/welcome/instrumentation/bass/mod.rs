use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct Bass {
    audio: crate::audio::AudioHandle,
}

impl Bass {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
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

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32> Act
    for Thump<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<Bass, BassArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
