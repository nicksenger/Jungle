use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct AudioSlaveState;

pub type AudioSlaveSeed = AudioSlaveState;

pub struct AudioSlaveIdle;
#[jungle::action]
impl Action for AudioSlaveIdle {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &AudioSlaveState, _input: Self::Input) -> Duration {
        Duration::from_millis(0)
    }

    fn absorb(
        _state: &mut AudioSlaveState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("audioslave idle stub should succeed"))?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct AudioSlaveJourney(Step<AudioSlaveIdle>);

pub struct AudioSlave;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for AudioSlave {
    type State = AudioSlaveState;
    type Seed = AudioSlaveSeed;
    type Flow = AudioSlaveJourney;
}

#[derive(Animals)]
pub struct PcmLandAnimals(AudioSlave);

pub struct PcmLand;
impl Ecosystem for PcmLand {
    const NAME: &'static str = "pcm-land";
    type Animals = PcmLandAnimals;
}

fn main() {
    eprintln!("audioslave example stub: not implemented yet");
}
