use jungle_sdk::prelude::*;

use crate::effect::{Sound, SoundInput};

use super::{Bass, BassArticulation};

#[allow(unused)]
pub struct Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Action
    for Thump<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<Bass>;
    type Input = ();
    type Output = ();

    fn emit(state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: *state,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: LANE_ID,
        }
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        Ok(())
    }
}
