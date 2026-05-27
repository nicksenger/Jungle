use jungle_sdk::prelude::*;

use crate::effect::{AtomicDualHit, AtomicTriHit, Monad};

use super::{Instrument, Note, SynthHandle};

pub(super) mod audio;

pub struct ElectricGuitar {
    audio: crate::audio::AudioHandle,
    synth: SynthHandle,
}

impl ElectricGuitar {
    pub fn new(audio: crate::audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
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
        audio::play(&self.audio, &self.synth, note).await
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
    type Effect = AtomicDualHit<
        ElectricGuitar,
        ElectricGuitar,
        ElectricGuitarArticulation,
        ElectricGuitarArticulation,
        LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_TICK,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        (*state, *state)
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
    type Effect = AtomicTriHit<
        ElectricGuitar,
        ElectricGuitar,
        ElectricGuitar,
        ElectricGuitarArticulation,
        ElectricGuitarArticulation,
        ElectricGuitarArticulation,
        LANE_ID,
        NOTE_1,
        NOTE_2,
        NOTE_3,
        NOTE_TICK,
        NOTE_TICK,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        (*state, *state, *state)
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
