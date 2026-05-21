use jungle_sdk::prelude::*;
use num::U255;
use std::time::Duration;

use crate::flow;

pub type RhythmGuitaristState = flow::RhythmGuitarIntroState;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = flow::RhythmGuitaristJourney;
}
