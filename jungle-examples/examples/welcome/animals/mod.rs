#![allow(dead_code)]

use jungle_sdk::prelude::*;
use num::U255;
use std::time::Duration;

use crate::flow;

mod rhythm_guitarist;
pub use rhythm_guitarist::*;

pub type LeadVocalistState = ();
pub type LeadVocalistSeed = ();
pub type LeadGuitaristState = ();
pub type LeadGuitaristSeed = ();
pub type BassState = ();
pub type BassSeed = ();
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
    type Journey = BandStubFlow;
}

pub struct LeadGuitarist;

#[jungle::animal(id = 1, generation = 0)]
impl Animal for LeadGuitarist {
    type State = LeadGuitaristState;
    type Seed = LeadGuitaristSeed;
    type Journey = BandStubFlow;
}

pub struct Bass;

#[jungle::animal(id = 3, generation = 0)]
impl Animal for Bass {
    type State = BassState;
    type Seed = BassSeed;
    type Journey = BandStubFlow;
}

pub struct Drums;

#[jungle::animal(id = 4, generation = 0)]
impl Animal for Drums {
    type State = DrumsState;
    type Seed = DrumsSeed;
    type Journey = BandStubFlow;
}
