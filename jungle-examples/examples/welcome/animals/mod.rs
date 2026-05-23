#![allow(dead_code)]

use jungle_sdk::prelude::*;
use std::time::Duration;

mod bassist;
mod drummer;
mod lead_guitarist;
mod lead_vocalist;
mod rhythm_guitarist;
pub use bassist::*;
pub use drummer::*;
pub use lead_guitarist::*;
pub use lead_vocalist::*;
pub use rhythm_guitarist::*;

#[derive(Animals)]
pub struct WelcomeAnimals(LeadVocalist, LeadGuitarist, RhythmGuitarist, Bass, Drums);

pub struct DecrementCounter<Focus>(core::marker::PhantomData<fn() -> Focus>);

#[jungle::act(aspect = Focus)]
impl<Focus> Act for DecrementCounter<Focus> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_view: &u8, _input: Self::Input) -> Self::Input {}

    fn absorb(view: &mut u8, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("counter decrement should succeed");
        *view = view.saturating_sub(1);
    }
}

pub struct StubStepSpec;

#[jungle::act]
impl Act for StubStepSpec {
    type Effect = Noop;
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

#[derive(Flow)]
pub struct Double<T>(T, T);

#[derive(Flow)]
pub struct Quad<T>(Double<T>, Double<T>);

#[derive(Flow)]
pub struct Octa<T>(Quad<T>, Quad<T>);

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
    type State = DrummerState;
    type Seed = DrummerSeed;
    type Journey = DrummerIntro;
}
