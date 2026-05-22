use jungle_sdk::prelude::*;

use crate::effect::Monad;

use super::{Instrument, Note};

mod audio;

pub struct SnareDrum {
    audio: crate::audio::AudioHandle,
}

impl SnareDrum {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SnareDrumArticulation {
    /// A standard, clean strike to the center of the drum head.
    CenterHit,
    /// Striking the center of the head and the metal rim simultaneously.
    /// This is the primary articulation for the massive verse/chorus backbeats.
    Rimshot,
    /// Laying the stick across the head and striking the rim for a woody click.
    /// Useful for low-energy dynamic drops.
    Sidestick,
    /// A very soft, low-velocity hit. Adler uses these to fill the space between backbeats.
    GhostNote,
    /// Two rapid, almost overlapping strikes (one hand trailing the other) to add weight.
    Flam,
}

impl Instrument for SnareDrum {
    type Articulation = SnareDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Crack<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8, const LANE_ID: u32> Act
    for Crack<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Monad<SnareDrum, SnareDrumArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &SnareDrumArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut SnareDrumArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
