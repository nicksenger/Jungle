use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod effect;
pub mod tokens;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState;

pub type MockingBirdSeed = MockingBirdState;

pub struct MockingBirdIdle;
#[jungle::action]
impl Action for MockingBirdIdle {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &MockingBirdState, _input: Self::Input) -> Duration {
        Duration::from_millis(0)
    }

    fn absorb(
        _state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("mockingbird idle stub should succeed"))?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct MockingBirdJourney(Step<MockingBirdIdle>);

pub struct MockingBird;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for MockingBird {
    type State = MockingBirdState;
    type Seed = MockingBirdSeed;
    type Flow = MockingBirdJourney;
}

#[derive(Animals)]
pub struct PcmParadiseAnimals(MockingBird);

pub struct PcmParadise;
impl Ecosystem for PcmParadise {
    const NAME: &'static str = "pcm-paradise";
    type Animals = PcmParadiseAnimals;
}

fn main() {
    eprintln!("mockingbird example stub: not implemented yet");
}
