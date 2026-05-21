use jungle_sdk::effect;
use std::marker::PhantomData;

use crate::ecosystem::WelcomeEcosystem;
use crate::instrumentation::{ElectricGuitarArticulation, Instrument, Note};

const TICKS_PER_SECOND: f32 = 787.2;

pub struct Monad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE: u8,
    const D_TICK: u8,
>(PhantomData<(I, A)>);

pub trait RhythmArticulation: Copy + Into<ElectricGuitarArticulation> {
    fn rhythm_sustained() -> Self;
}

impl RhythmArticulation for ElectricGuitarArticulation {
    fn rhythm_sustained() -> Self {
        Self::RhythmSustained
    }
}

#[effect(id = 500)]
impl<I, A, const NOTE: u8, const D_TICK: u8> jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Monad<I, A, NOTE, D_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let playable_note = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: std::time::Duration::from_secs_f32((D_TICK as f32) / TICKS_PER_SECOND),
            velocity: 37.0 / 127.0,
            expression: None,
            articulation: A::rhythm_sustained().into(),
        };
        jungle
            .rhythm_guitar()
            .play(playable_note)
            .await
            .map_err(|err| err.to_string())
    }
}
