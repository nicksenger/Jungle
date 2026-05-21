use jungle_sdk::effect;
use jungle_sdk::prelude::*;
use num::U255;
use std::time::Duration;

use crate::flow;
use crate::score::Kind;

pub type RhythmGuitaristState = flow::RhythmGuitarIntroState;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = flow::RhythmGuitaristJourney;
}

pub struct Pick<const NOTE: u8, const TICKS: u8>;

//#[effect(id = 500)]
//impl<const NOTE: u8, const TICKS: u8> Effect<WelcomeEcosystem> for Pick<NOTE, TICKS> {
//    type In = ();
//    type Out = ();
//    type Err = String;
//
//    async fn effect(jungle: &WelcomeEcosystem, note: Self::In) -> Result<Self::Out, Self::Err> {
//        let bpm = jungle.bpm();
//        let playable_note = Note {
//            n_midi: NOTE,
//            amplitude_multiplier: 0.5,
//            pan: 0.5,
//            duration: std::time::Duration::from_secs_f32(duration_beats * seconds_per_beat),
//            velocity: 37.0 / 127.0,
//            expression: None,
//            articulation: ElectricGuitarArticulation::RhythmSustained,
//        };
//        jungle
//            .rhythm_guitar()
//            .play(playable_note)
//            .await
//            .map_err(|err| err.to_string())
//    }
//}
