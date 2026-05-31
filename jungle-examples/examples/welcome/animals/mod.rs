use std::{marker::PhantomData, time::Duration};

use crate::flow::loop2::*;
use crate::instrumentation::{
    phonemes_from_text, BassArticulation, ElectricGuitarArticulation, Lyrics, VocalsArticulation,
};
use jungle_sdk::prelude::*;

#[cfg(feature = "bass")]
mod bassist;
#[cfg(feature = "drums")]
mod drummer;
#[cfg(feature = "leadguitar")]
mod lead_guitarist;
#[cfg(feature = "vocals")]
mod lead_vocalist;
#[cfg(feature = "rhythmguitar")]
mod rhythm_guitarist;
#[cfg(feature = "bass")]
pub use bassist::*;
#[cfg(feature = "drums")]
pub use drummer::*;
#[cfg(feature = "leadguitar")]
pub use lead_guitarist::*;
#[cfg(feature = "vocals")]
pub use lead_vocalist::*;
#[cfg(feature = "rhythmguitar")]
pub use rhythm_guitarist::*;

#[cfg(not(test))]
#[derive(Animals)]
pub struct WelcomeAnimals(LeadVocalist, RhythmGuitarist, LeadGuitarist, Bass, Drums);

pub struct LeadGuitarist;
#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LeadGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
    transition_loops_remaining: u8,
    sustain_loops_remaining: u8,
}
impl Default for LeadGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::default(),
            riff_loops_remaining: 1,
            transition_loops_remaining: 3,
            sustain_loops_remaining: 1,
        }
    }
}
#[jungle::animal(id = 2, generation = 0)]
impl Animal for LeadGuitarist {
    type State = LeadGuitaristState;
    type Seed = ();
    #[cfg(feature = "leadguitar")]
    type Journey = LeadGuitarFlow;
    #[cfg(not(feature = "leadguitar"))]
    type Journey = StubFlow<(), LeadGuitaristState>;
}

pub struct LeadVocalist;
#[derive(Optic, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeadVocalistState {
    articulation: VocalsArticulation,
    intro_pickup_remaining: u8,
    pub lyrics: Lyrics,
}
impl Default for LeadVocalistState {
    fn default() -> Self {
        Self {
            articulation: VocalsArticulation::Clean,
            intro_pickup_remaining: 1,
            lyrics: Lyrics {
                phonemes: [
                    "ha",
                    "down",
                    "you",
                    "bring",
                    "na",
                    "gon",
                    "its",
                    "your",
                    "to",
                    "you",
                    "bring",
                    "it",
                    "Watch",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "knees",
                    "knees",
                    "knees",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "shuh",
                    "your",
                    "to",
                    "you",
                    "bring",
                    "it",
                    "Watch",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "teen",
                    "pen",
                    "ser",
                    "my",
                    "my",
                    "my",
                    "my",
                    "my",
                    "Feel",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "In",
                    "you're",
                    "knees",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "shuh",
                    "your",
                    "to",
                    "you",
                    "bring",
                    "it",
                    "Watch",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "here",
                    "eye",
                    "die",
                    "na",
                    "gon",
                    "you're",
                    "bee",
                    "bay",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "you're",
                    "are?",
                    "you",
                    "where",
                    "know",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "ha",
                    "yeah",
                    "heh",
                    "heh",
                    "heh",
                    "heh",
                    "yeah",
                    "down",
                    "down",
                    "so",
                    "down",
                    "down",
                    "So",
                    "down",
                    "down",
                    "so",
                    "down",
                    "come",
                    "to",
                    "want",
                    "ver",
                    "ne",
                    "want",
                    "er",
                    "nev",
                    "you",
                    "eye",
                    "high",
                    "you're",
                    "when",
                    "and",
                    "bleed",
                    "you",
                    "watch",
                    "na",
                    "wan",
                    "I",
                    "knees",
                    "knees",
                    "knees",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "shuh",
                    "your",
                    "to",
                    "you",
                    "bring",
                    "it",
                    "Watch",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "me",
                    "from",
                    "it",
                    "take",
                    "not",
                    "better",
                    "you",
                    "but",
                    "want",
                    "you",
                    "everything",
                    "have",
                    "can",
                    "you",
                    "and",
                    "lee",
                    "tual",
                    "even",
                    "it",
                    "take",
                    "youll",
                    "see",
                    "you",
                    "what",
                    "for",
                    "ger",
                    "hun",
                    "you",
                    "if",
                    "and",
                    "play",
                    "we",
                    "where",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "here",
                    "mal",
                    "i",
                    "an",
                    "an",
                    "like",
                    "live",
                    "to",
                    "learn",
                    "you",
                    "day",
                    "ree",
                    "ev",
                    "here",
                    "worse",
                    "gets",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "Wel",
                    "scream",
                    "you",
                    "hear",
                    "na",
                    "wan",
                    "I",
                    "teen",
                    "pen",
                    "ser",
                    "my",
                    "my",
                    "my",
                    "my",
                    "Feel",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "oh",
                    "free",
                    "for",
                    "there",
                    "get",
                    "wont",
                    "you",
                    "but",
                    "lights",
                    "bright",
                    "the",
                    "taste",
                    "can",
                    "You",
                    "please",
                    "to",
                    "hard",
                    "eee",
                    "vair",
                    "thats",
                    "girl",
                    "eee",
                    "sex",
                    "eee",
                    "vair",
                    "a",
                    "youre",
                    "And",
                    "pay",
                    "to",
                    "price",
                    "the",
                    "its",
                    "but",
                    "bleed",
                    "na",
                    "gon",
                    "youre",
                    "it",
                    "want",
                    "you",
                    "If",
                    "day",
                    "by",
                    "day",
                    "it",
                    "take",
                    "we",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "Wel",
                    "bleed",
                    "you",
                    "watch",
                    "na",
                    "wan",
                    "I",
                    "I",
                    "knees",
                    "knees",
                    "knees",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "nuh",
                    "shuh",
                    "your",
                    "to",
                    "you",
                    "bring",
                    "it",
                    "watch",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "wel",
                    "gol",
                    "jun",
                    "the",
                    "in",
                    "here",
                    "oh",
                    "ease",
                    "dis",
                    "your",
                    "got",
                    "we",
                    "neee",
                    "huh",
                    "ney",
                    "muh",
                    "the",
                    "got",
                    "you",
                    "If",
                    "need",
                    "may",
                    "you",
                    "er",
                    "ev",
                    "what",
                    "find",
                    "can",
                    "that",
                    "ple",
                    "peo",
                    "the",
                    "are",
                    "We",
                    "names",
                    "the",
                    "know",
                    "we",
                    "ney",
                    "huh",
                    "want",
                    "you",
                    "thing",
                    "ree",
                    "ev",
                    "got",
                    "We",
                    "games",
                    "games",
                    "and",
                    "fun",
                    "got",
                    "we",
                    "gol",
                    "jun",
                    "the",
                    "to",
                    "come",
                    "Wel",
                    "ha",
                ]
                .into_iter()
                .map(phonemes_from_text)
                .collect(),
            },
        }
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LeadVocalistSeed {
    pub lyrics: Option<Vec<String>>,
}
#[jungle::animal(id = 0, generation = 0)]
impl Animal for LeadVocalist {
    type State = LeadVocalistState;
    type Seed = LeadVocalistSeed;
    #[cfg(feature = "vocals")]
    type Journey = LeadVocalIntro;
    #[cfg(not(feature = "vocals"))]
    type Journey = StubFlow<LeadVocalistSeed, LeadVocalistState>;
}

pub struct RhythmGuitarist;
#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RhythmGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
}
impl Default for RhythmGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::Sustained,
            riff_loops_remaining: 1,
        }
    }
}
#[jungle::animal(id = 1, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = ();
    #[cfg(feature = "rhythmguitar")]
    type Journey = RhythmGuitarIntro;
    #[cfg(not(feature = "rhythmguitar"))]
    type Journey = StubFlow<(), RhythmGuitaristState>;
}

pub struct Bass;
#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BassistState {
    #[jungle(focus)]
    articulation: BassArticulation,
    ostinato_loops_remaining: u8,
    riff_loops_remaining: u8,
    #[jungle(focus)]
    loop2: Loop2Container<BassArticulation>,
}
impl Default for BassistState {
    fn default() -> Self {
        Self {
            articulation: BassArticulation::Picked,
            ostinato_loops_remaining: 1,
            riff_loops_remaining: 1,
            loop2: Loop2Container::new(BassArticulation::Picked),
        }
    }
}
#[jungle::animal(id = 3, generation = 0)]
impl Animal for Bass {
    type State = BassistState;
    type Seed = ();
    #[cfg(feature = "bass")]
    type Journey = BassIntro;
    #[cfg(not(feature = "bass"))]
    type Journey = StubFlow<(), BassistState>;
}

pub struct Drums;
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct DrummerState {
    groove_variant_is_46: bool,
}
impl Default for DrummerState {
    fn default() -> Self {
        Self {
            groove_variant_is_46: true,
        }
    }
}
#[jungle::animal(id = 4, generation = 0)]
impl Animal for Drums {
    type State = DrummerState;
    type Seed = ();
    #[cfg(feature = "drums")]
    type Journey = DrummerIntro;
    #[cfg(not(feature = "drums"))]
    type Journey = StubFlow<(), DrummerState>;
}

#[allow(unused)]
pub struct DecrementCounter<Focus>(core::marker::PhantomData<fn() -> Focus>);
#[jungle::action(aspect = Focus)]
impl<Focus> Action for DecrementCounter<Focus> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_view: &u8, _input: Self::Input) -> Self::Input {}

    fn absorb(
        view: &mut u8,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = (|| {
            output.expect("counter decrement should succeed");
            *view = view.saturating_sub(1);
        })();
        Ok(__absorb_out_1)
    }
}

pub struct StubStepSpec;
#[jungle::action]
impl Action for StubStepSpec {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(
        _state: &mut (),
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = (|| {})();
        Ok(__absorb_out_2)
    }
}

pub struct SleepFiveMinutesSpec;
#[jungle::action]
impl Action for SleepFiveMinutesSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        Duration::from_secs(5 * 60)
    }

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = (|| {
            output.expect("sleep step should complete after worker wakeup");
        })();
        Ok(__absorb_out_3)
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

#[derive(Flow)]
pub struct StubFlow<T, S>(Step<Stub<T, S>>);
pub struct Stub<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::action]
impl<T, S> Action for Stub<T, S> {
    type Effect = Noop;
    type Input = T;
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}
    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = (|| {})();
        Ok(__absorb_out_4)
    }
}

#[cfg(test)]
#[derive(Animals)]
pub struct WelcomeAnimals(
    BassJoinSound100Animal,
    RhythmJoinSound100Animal,
    ConditionalJoinSound100Animal,
);
