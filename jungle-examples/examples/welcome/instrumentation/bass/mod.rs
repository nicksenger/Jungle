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
    /// Forcing the string down so hard it clanks against the frets on attack.
    /// Used to accent the downbeats of the chorus.
    AccentedClank,
    /// Muting the string immediately with the fretting hand.
    /// Essential for keeping the fast-moving basslines crisp and preventing mud.
    StaccatoMute,
    /// Sliding from one note down into the next, a classic Duff transition tool.
    SlideDown,
    /// Striking a completely dead string for a purely percussive thud.
    GhostNote,
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

pub struct Thump<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Thump<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Bass, BassArticulation, NOTE, NOTE_TICK, REST_TICK>;
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
