use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct Vocals {
    audio: crate::audio::AudioHandle,
}

impl Vocals {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum VocalsArticulation {
    /// Clean, melodic singing with standard resonance (e.g., the lower register parts of the verses).
    Clean,
    /// Pushing the voice into a distorted, high-register rock belt.
    /// This is Axl's signature sound for the choruses.
    GritRasp,
    /// The chest-voice, semi-spoken, low-register delivery.
    /// Essential for the "Do you know where you are?" breakdown.
    SpokenBreakdown,
    /// Ultra-high, piercing falsetto screams (like the legendary "Welcome to the Jungle!" intro howl).
    SirenScream,
    /// Rapid, rhythmic, percussive vocal sound effects (e.g., the stuttering "nn-nn-nn-nn-nn-nn-nn-f-f-freee").
    StutterStab,
    /// Clean, unified group harmony backing up a lead line.
    GroupHarmony,
    /// Aggressive, chanted, or shouted backing lines (e.g., shouting "Jungle!" in response to Axl).
    ShoutResponse,
    /// Sustained, open-vowel vocal beds ("Ahhs" or "Ohhs") used for atmospheric backing texture.
    VocalBed,
}

impl Instrument for Vocals {
    type Articulation = VocalsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Sing<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Sing<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Vocals, VocalsArticulation, NOTE, NOTE_TICK, REST_TICK>;
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
