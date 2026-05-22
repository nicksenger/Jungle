use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct HiHat {
    audio: crate::audio::AudioHandle,
}

impl HiHat {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum HiHatArticulation {
    /// Fully pressed closed, yielding a tight, crisp "chick" sound.
    ClosedTip,
    /// Striking the edge of a closed hi-hat with the shoulder of the stick for more bite.
    ClosedEdge,
    /// Slightly releasing foot pressure so the cymbals sizzle against each other.
    /// Essential for building tension in the pre-chorus.
    HalfOpen,
    /// Completely open, creating a loud, aggressive, sloshy wash. Used in the choruses.
    FullOpen,
    /// Closing the hats purely with the foot pedal, creating a soft "chick" with no stick attack.
    FootSplash,
}

impl Instrument for HiHat {
    type Articulation = HiHatArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Chick<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32> Act
    for Chick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<HiHat, HiHatArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(state: &HiHatArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut HiHatArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
