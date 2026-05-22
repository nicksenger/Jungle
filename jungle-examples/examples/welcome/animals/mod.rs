#![allow(dead_code)]

use jungle_sdk::prelude::*;
use num::U255;
use std::time::Duration;

mod bassist;
mod counter;
mod lead_guitarist;
mod lead_vocalist;
mod rhythm_guitarist;
pub use bassist::*;
pub use lead_guitarist::*;
pub use lead_vocalist::*;
pub use rhythm_guitarist::*;

#[derive(Animals)]
pub struct WelcomeAnimals(LeadVocalist, LeadGuitarist, RhythmGuitarist, Bass, Drums);

pub type DrumsState = ();
pub type DrumsSeed = ();

pub struct StubEffect;

impl EffectSchema for StubEffect {
    type Id = Id<U255>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J> Effect<J> for StubEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

pub struct StubStepSpec;

#[jungle::act]
impl Act for StubStepSpec {
    type Effect = StubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(_state: &mut (), _output: EffectCompletion<Self::Effect>) -> Self::Output {}
}

pub struct SleepFiveMinutesSpec;

#[jungle::act]
impl Act for SleepFiveMinutesSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        Duration::from_secs(5 * 60)
    }

    fn absorb(_state: &mut (), output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("sleep step should complete after worker wakeup");
    }
}

#[derive(Flow)]
pub struct BandStubFlow(Step<StubStepSpec>, Step<SleepFiveMinutesSpec>);

pub struct LeadVocalist;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for LeadVocalist {
    type State = LeadVocalistState;
    type Seed = LeadVocalistSeed;
    type Journey = LeadVocalIntro;
}

pub struct LeadGuitarist;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for LeadGuitarist {
    type State = LeadGuitaristState;
    type Seed = LeadGuitaristSeed;
    type Journey = LeadGuitarIntro;
}

pub struct Bass;

#[jungle::animal(id = 3, generation = 0)]
impl Animal for Bass {
    type State = BassistState;
    type Seed = BassistSeed;
    type Journey = BassIntro;
}

pub struct Drums;

#[jungle::animal(id = 4, generation = 0)]
impl Animal for Drums {
    type State = DrumsState;
    type Seed = DrumsSeed;
    type Journey = BandStubFlow;
}
