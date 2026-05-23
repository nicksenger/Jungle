use jungle_sdk::prelude::*;

use crate::effect::{Dyad, Monad, Triad};

use super::{Instrument, Note};

mod audio;

pub struct ElectricGuitar {
    audio: crate::audio::AudioHandle,
}

impl ElectricGuitar {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }

    pub fn audio(&self) -> &crate::audio::AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ElectricGuitarArticulation {
    /// Standard picked note with normal sustain and release.
    Sustained,
    /// A sustained rhythm-guitar chord voice.
    RhythmSustained,
}

impl Default for ElectricGuitarArticulation {
    fn default() -> Self {
        Self::RhythmSustained
    }
}

impl Instrument for ElectricGuitar {
    type Articulation = ElectricGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32 = 0>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u32> Act
    for Pick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect =
        Monad<ElectricGuitar, ElectricGuitarArticulation, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct Pluck<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u32 = 0,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
        const LANE_ID: u32,
    > Act for Pluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Dyad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u32 = 0,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
        const LANE_ID: u32,
    > Act for Strum<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Triad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_3,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
