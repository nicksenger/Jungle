use jungle_sdk::prelude::*;

use crate::{
    animals::LeadVocalistState,
    effect::{Sound, SoundInput},
};

use super::{Vocals, VocalsArticulation};

#[allow(unused)]
pub struct Generate<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Action
    for Generate<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<Vocals>;
    type Input = ();
    type Output = ();

    fn emit(state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: VocalsArticulation::Formant(
                state
                    .lyrics
                    .phonemes
                    .last()
                    .copied()
                    .unwrap_or_else(|| [None; 12]),
            ),
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: LANE_ID,
        }
    }

    fn absorb(
        state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        let _ = state.lyrics.phonemes.pop();
        Ok(())
    }
}
