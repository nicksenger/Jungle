use jungle_sdk::effect;
use jungle_sdk::prelude::*;
use num::U255;
use std::time::Duration;

use crate::ecosystem::WelcomeEcosystem;
use crate::flow;
use crate::instrumentation::{ElectricGuitarArticulation, Instrument, Note};
use crate::score::Kind;

const TICKS_PER_SECOND: f32 = 787.2;

pub type RhythmGuitaristState = flow::RhythmGuitarIntroState;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = flow::RhythmGuitaristJourney;
}

//pub struct Pick<const NOTE: u8, const D_TICK: u8>;
//#[jungle::act]
//impl<const NOTE: u8, const D_TICK: u8> Act for Pick<NOTE, D_TICK> {
//    type Effect = Monad<NOTE, D_TICK>;
//    type Input = ();
//    type Output = ();
//
//    fn emit(_state: &(), _input: Self::Input) -> Self::Input {}
//
//    fn absorb(_state: &mut (), output: EffectCompletion<Self::Effect>) -> Self::Output {
//        output.expect("note playback should succeed");
//    }
//}

pub struct Monad<const NOTE: u8, const D_TICK: u8>;
#[effect(id = 500)]
impl<const NOTE: u8, const D_TICK: u8> Effect<WelcomeEcosystem> for Monad<NOTE, D_TICK> {
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, note: Self::In) -> Result<Self::Out, Self::Err> {
        let bpm = jungle.bpm();
        let playable_note = Note {
            n_midi: NOTE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: std::time::Duration::from_secs_f32((D_TICK as f32) * TICKS_PER_SECOND),
            velocity: 37.0 / 127.0,
            expression: None,
            articulation: ElectricGuitarArticulation::RhythmSustained,
        };
        jungle
            .rhythm_guitar()
            .play(playable_note)
            .await
            .map_err(|err| err.to_string())
    }
}
