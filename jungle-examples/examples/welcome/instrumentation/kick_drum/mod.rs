use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct KickDrum {
    audio: crate::audio::AudioHandle,
}

impl KickDrum {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum KickDrumArticulation {
    /// A standard, powerful kick where the beater strikes and bounces off.
    StandardHit,
    /// Burying the beater into the head, dampening the sustain for a tighter, punchier thud.
    BuriedBeater,
    /// A soft, unaccented hit used in quick double-stroke patterns.
    GhostHit,
}

impl Instrument for KickDrum {
    type Articulation = KickDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Kick<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32> Act
    for Kick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<KickDrum, KickDrumArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &KickDrumArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut KickDrumArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
