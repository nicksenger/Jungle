use jungle_sdk::prelude::*;

use crate::{
    action::{MergeUnit, Rest as GenericRest},
    effect::{Sound, SoundInput},
};

use super::{ElectricGuitar, ElectricGuitarArticulation};

type PostMergeRest<const TICKS: u32, const LANE_ID: u8> =
    GenericRest<ElectricGuitarArticulation, TICKS, LANE_ID>;

pub struct Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Action
    for Pick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<ElectricGuitar>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: *state,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: LANE_ID,
        }
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct Pluck<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u8,
>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0, LANE_ID>>, Step<Pick<NOTE_2, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit<ElectricGuitarArticulation>>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);

#[derive(Flow)]
pub struct StrumPair<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const LANE_ID: u8>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0, LANE_ID>>, Step<Pick<NOTE_2, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit<ElectricGuitarArticulation>>,
);

#[derive(Flow)]
pub struct Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u8,
>(
    Join<StrumPair<NOTE_1, NOTE_2, NOTE_TICK, LANE_ID>, Step<Pick<NOTE_3, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit<ElectricGuitarArticulation>>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);
