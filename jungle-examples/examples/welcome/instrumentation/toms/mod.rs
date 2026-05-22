use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct Toms {
    audio: crate::audio::AudioHandle,
}

impl Toms {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TomsArticulation {
    /// A clean, resonant strike to the center of the tom.
    StandardHit,
    /// An extra-powerful strike maximizing shell resonance.
    AccentedHit,
    /// Striking two different toms simultaneously (e.g., Rack Tom 2 and Floor Tom).
    /// Used for the massive downbeat punctuation marks in the breakdown.
    DoubleHit,
}

impl Instrument for Toms {
    type Articulation = TomsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Tom<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32> Act
    for Tom<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<Toms, TomsArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(state: &TomsArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut TomsArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
