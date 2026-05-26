use jungle_sdk::prelude::*;

use crate::effect::{Passthrough, Rest};
use crate::instrumentation::{
    phonemes_from_text, Generate as LaneGenerate, Lyrics, VocalsArticulation,
};

use super::{Double, LeadVocalist};

const LEAD_VOCALS_LANE_ID: u32 = <<LeadVocalist as Animal>::Id as AnimalIdValue>::U32;
type Generate<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneGenerate<NOTE, NOTE_TICK, REST_TICK, LEAD_VOCALS_LANE_ID>;

type Generate68Tick = Step<Generate<68, 96, 96>>;
type Sing68Tick = Step<Generate<68, 96, 96>>;
type Generate68Hold = Step<Generate<68, 192, 192>>;
type Sing68Hold = Step<Generate<68, 192, 192>>;
type Sing63Hold = Step<Generate<63, 192, 192>>;

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
                phonemes: vec![
                    phonemes_from_text("ha"),
                    phonemes_from_text("down"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("na"),
                    phonemes_from_text("gon"),
                    phonemes_from_text("its"),
                    phonemes_from_text("your"),
                    phonemes_from_text("to"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("it"),
                    phonemes_from_text("Watch"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("shu"),
                    phonemes_from_text("your"),
                    phonemes_from_text("to"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("it"),
                    phonemes_from_text("Watch"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("teen"),
                    phonemes_from_text("pen"),
                    phonemes_from_text("ser"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("Feel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("In"),
                    phonemes_from_text("you're"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("shu"),
                    phonemes_from_text("your"),
                    phonemes_from_text("to"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("it"),
                    phonemes_from_text("Watch"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("here"),
                    phonemes_from_text("eee"),
                    phonemes_from_text("die"),
                    phonemes_from_text("na"),
                    phonemes_from_text("gon"),
                    phonemes_from_text("you're"),
                    phonemes_from_text("bee"),
                    phonemes_from_text("bay"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("you're"),
                    phonemes_from_text("are?"),
                    phonemes_from_text("you"),
                    phonemes_from_text("where"),
                    phonemes_from_text("know"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("la"),
                    phonemes_from_text("yeah"),
                    phonemes_from_text("heh"),
                    phonemes_from_text("heh"),
                    phonemes_from_text("heh"),
                    phonemes_from_text("heh"),
                    phonemes_from_text("yeah"),
                    phonemes_from_text("down"),
                    phonemes_from_text("down"),
                    phonemes_from_text("so"),
                    phonemes_from_text("down"),
                    phonemes_from_text("down"),
                    phonemes_from_text("So"),
                    phonemes_from_text("down"),
                    phonemes_from_text("down"),
                    phonemes_from_text("so"),
                    phonemes_from_text("down"),
                    phonemes_from_text("come"),
                    phonemes_from_text("to"),
                    phonemes_from_text("want"),
                    phonemes_from_text("ver"),
                    phonemes_from_text("ne"),
                    phonemes_from_text("want"),
                    phonemes_from_text("er"),
                    phonemes_from_text("nev"),
                    phonemes_from_text("you"),
                    phonemes_from_text("iigh"),
                    phonemes_from_text("high"),
                    phonemes_from_text("you're"),
                    phonemes_from_text("when"),
                    phonemes_from_text("and"),
                    phonemes_from_text("bleed"),
                    phonemes_from_text("you"),
                    phonemes_from_text("watch"),
                    phonemes_from_text("na"),
                    phonemes_from_text("wan"),
                    phonemes_from_text("I"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("your"),
                    phonemes_from_text("to"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("it"),
                    phonemes_from_text("Watch"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("In"),
                    phonemes_from_text("me"),
                    phonemes_from_text("from"),
                    phonemes_from_text("it"),
                    phonemes_from_text("take"),
                    phonemes_from_text("not"),
                    phonemes_from_text("better"),
                    phonemes_from_text("you"),
                    phonemes_from_text("but"),
                    phonemes_from_text("want"),
                    phonemes_from_text("you"),
                    phonemes_from_text("thing"),
                    phonemes_from_text("every"),
                    phonemes_from_text("have"),
                    phonemes_from_text("can"),
                    phonemes_from_text("you"),
                    phonemes_from_text("and"),
                    phonemes_from_text("lee"),
                    phonemes_from_text("tual"),
                    phonemes_from_text("en"),
                    phonemes_from_text("eve"),
                    phonemes_from_text("it"),
                    phonemes_from_text("take"),
                    phonemes_from_text("youll"),
                    phonemes_from_text("see"),
                    phonemes_from_text("you"),
                    phonemes_from_text("what"),
                    phonemes_from_text("for"),
                    phonemes_from_text("ger"),
                    phonemes_from_text("hun"),
                    phonemes_from_text("you"),
                    phonemes_from_text("if"),
                    phonemes_from_text("and"),
                    phonemes_from_text("play"),
                    phonemes_from_text("we"),
                    phonemes_from_text("where"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("here"),
                    phonemes_from_text("mal"),
                    phonemes_from_text("i"),
                    phonemes_from_text("an"),
                    phonemes_from_text("an"),
                    phonemes_from_text("like"),
                    phonemes_from_text("live"),
                    phonemes_from_text("to"),
                    phonemes_from_text("learn"),
                    phonemes_from_text("you"),
                    phonemes_from_text("day"),
                    phonemes_from_text("ree"),
                    phonemes_from_text("ev"),
                    phonemes_from_text("here"),
                    phonemes_from_text("worse"),
                    phonemes_from_text("gets"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("Wel"),
                    phonemes_from_text("scream"),
                    phonemes_from_text("you"),
                    phonemes_from_text("hear"),
                    phonemes_from_text("na"),
                    phonemes_from_text("wan"),
                    phonemes_from_text("I"),
                    phonemes_from_text("teen"),
                    phonemes_from_text("pen"),
                    phonemes_from_text("ser"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("my"),
                    phonemes_from_text("Feel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("oh"),
                    phonemes_from_text("free"),
                    phonemes_from_text("for"),
                    phonemes_from_text("there"),
                    phonemes_from_text("get"),
                    phonemes_from_text("wont"),
                    phonemes_from_text("you"),
                    phonemes_from_text("but"),
                    phonemes_from_text("lights"),
                    phonemes_from_text("bright"),
                    phonemes_from_text("the"),
                    phonemes_from_text("taste"),
                    phonemes_from_text("can"),
                    phonemes_from_text("You"),
                    phonemes_from_text("please"),
                    phonemes_from_text("to"),
                    phonemes_from_text("hard"),
                    phonemes_from_text("eee"),
                    phonemes_from_text("vair"),
                    phonemes_from_text("thats"),
                    phonemes_from_text("girl"),
                    phonemes_from_text("eee"),
                    phonemes_from_text("sex"),
                    phonemes_from_text("eee"),
                    phonemes_from_text("vair"),
                    phonemes_from_text("a"),
                    phonemes_from_text("youre"),
                    phonemes_from_text("And"),
                    phonemes_from_text("pay"),
                    phonemes_from_text("to"),
                    phonemes_from_text("price"),
                    phonemes_from_text("the"),
                    phonemes_from_text("its"),
                    phonemes_from_text("but"),
                    phonemes_from_text("bleed"),
                    phonemes_from_text("na"),
                    phonemes_from_text("gon"),
                    phonemes_from_text("youre"),
                    phonemes_from_text("it"),
                    phonemes_from_text("want"),
                    phonemes_from_text("you"),
                    phonemes_from_text("If"),
                    phonemes_from_text("day"),
                    phonemes_from_text("by"),
                    phonemes_from_text("day"),
                    phonemes_from_text("it"),
                    phonemes_from_text("take"),
                    phonemes_from_text("we"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("Wel"),
                    phonemes_from_text("bleed"),
                    phonemes_from_text("you"),
                    phonemes_from_text("watch"),
                    phonemes_from_text("na"),
                    phonemes_from_text("wan"),
                    phonemes_from_text("I"),
                    phonemes_from_text("I"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("knees"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("nu"),
                    phonemes_from_text("your"),
                    phonemes_from_text("to"),
                    phonemes_from_text("you"),
                    phonemes_from_text("bring"),
                    phonemes_from_text("it"),
                    phonemes_from_text("watch"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("wel"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("in"),
                    phonemes_from_text("here"),
                    phonemes_from_text("oh"),
                    phonemes_from_text("ease"),
                    phonemes_from_text("dis"),
                    phonemes_from_text("your"),
                    phonemes_from_text("got"),
                    phonemes_from_text("we"),
                    phonemes_from_text("neee"),
                    phonemes_from_text("huh"),
                    phonemes_from_text("ney"),
                    phonemes_from_text("muh"),
                    phonemes_from_text("the"),
                    phonemes_from_text("got"),
                    phonemes_from_text("you"),
                    phonemes_from_text("If"),
                    phonemes_from_text("need"),
                    phonemes_from_text("may"),
                    phonemes_from_text("you"),
                    phonemes_from_text("er"),
                    phonemes_from_text("ev"),
                    phonemes_from_text("what"),
                    phonemes_from_text("find"),
                    phonemes_from_text("can"),
                    phonemes_from_text("that"),
                    phonemes_from_text("ple"),
                    phonemes_from_text("peo"),
                    phonemes_from_text("the"),
                    phonemes_from_text("are"),
                    phonemes_from_text("We"),
                    phonemes_from_text("names"),
                    phonemes_from_text("the"),
                    phonemes_from_text("know"),
                    phonemes_from_text("we"),
                    phonemes_from_text("ney"),
                    phonemes_from_text("ho"),
                    phonemes_from_text("want"),
                    phonemes_from_text("you"),
                    phonemes_from_text("thing"),
                    phonemes_from_text("ry"),
                    phonemes_from_text("ev"),
                    phonemes_from_text("got"),
                    phonemes_from_text("We"),
                    phonemes_from_text("games"),
                    phonemes_from_text("games"),
                    phonemes_from_text("and"),
                    phonemes_from_text("fun"),
                    phonemes_from_text("got"),
                    phonemes_from_text("we"),
                    phonemes_from_text("gol"),
                    phonemes_from_text("jun"),
                    phonemes_from_text("the"),
                    phonemes_from_text("to"),
                    phonemes_from_text("come"),
                    phonemes_from_text("Wel"),
                    phonemes_from_text("huh"),
                ],
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LeadVocalistSeed {
    pub lyrics: Option<Vec<String>>,
}

const INTRO_START_DELAY_TICKS: u32 = 20_352;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct ApplyLeadVocalistSeed;
#[jungle::act]
impl Act for ApplyLeadVocalistSeed {
    type Effect = Passthrough<LeadVocalistSeed>;
    type Input = LeadVocalistSeed;
    type Output = ();

    fn emit(_state: &LeadVocalistState, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let seed = output.expect("lead vocalist seed step should complete");
        if let Some(lyrics) = seed.lyrics {
            let phonemes = lyrics
                .iter()
                .rev()
                .map(|word| phonemes_from_text(word))
                .collect::<Vec<_>>();
            if !phonemes.is_empty() {
                state.lyrics.phonemes = phonemes;
            }
        }
    }
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<LEAD_VOCALS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct UseLeadVocalPickup;
impl Condition<(LeadVocalistState, ())> for UseLeadVocalPickup {
    fn choose((state, _): &(LeadVocalistState, ())) -> bool {
        state.intro_pickup_remaining > 0
    }
}

pub struct ConsumeLeadVocalPickup;
#[jungle::act]
impl Act for ConsumeLeadVocalPickup {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead vocal pickup consume should complete");
        state.intro_pickup_remaining = state.intro_pickup_remaining.saturating_sub(1);
    }
}

pub struct MergeLeadVocalPickupChoice;
#[jungle::act]
impl Act for MergeLeadVocalPickupChoice {
    type Effect = Noop;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
    }

    fn absorb(
        _state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("lead vocal pickup branch merge should complete");
    }
}

#[derive(Flow)]
pub struct LeadVocalPickupBranch(
    Transparent<IntroSectionMeta, LeadVocalSection01>,
    Step<ConsumeLeadVocalPickup>,
);

#[derive(Flow)]
pub struct LeadVocalMainBranch(Step<ConsumeLeadVocalPickup>);

#[derive(Flow)]
pub struct LeadVocalIntro(
    Step<ApplyLeadVocalistSeed>,
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Conditional<UseLeadVocalPickup, LeadVocalPickupBranch, LeadVocalMainBranch>,
    Step<MergeLeadVocalPickupChoice>,
    Transparent<IntroSectionMeta, LeadVocalSection02>,
    Transparent<IntroSectionMeta, LeadVocalSection03>,
);

#[derive(Flow)]
pub struct LeadVocalSection01(
    Transparent<IntroSectionMeta, LeadVocalPart01>,
    Transparent<IntroSectionMeta, LeadVocalPart02>,
    Transparent<IntroSectionMeta, LeadVocalPart03>,
    Transparent<IntroSectionMeta, LeadVocalPart04>,
    Transparent<IntroSectionMeta, LeadVocalPart05>,
    Transparent<IntroSectionMeta, LeadVocalPart06>,
);

#[derive(Flow)]
pub struct LeadVocalSection02(
    Transparent<IntroSectionMeta, LeadVocalPart07>,
    Transparent<IntroSectionMeta, LeadVocalPart08>,
    Transparent<IntroSectionMeta, LeadVocalPart09>,
    Transparent<IntroSectionMeta, LeadVocalPart10>,
    Transparent<IntroSectionMeta, LeadVocalPart11>,
    Transparent<IntroSectionMeta, LeadVocalPart12>,
);

#[derive(Flow)]
pub struct LeadVocalSection03(
    Transparent<IntroSectionMeta, LeadVocalPart13>,
    Transparent<IntroSectionMeta, LeadVocalPart14>,
    Transparent<IntroSectionMeta, LeadVocalPart15>,
    Transparent<IntroSectionMeta, LeadVocalPart16>,
    Transparent<IntroSectionMeta, LeadVocalPart17>,
    Transparent<IntroSectionMeta, LeadVocalPart18>,
);

#[derive(Flow)]
pub struct LeadVocalPart01(
    Step<Generate<58, 192, 6528>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<71, 384, 384>>,
    Step<Generate<68, 192, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<71, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<71, 480, 480>>,
    Step<Generate<66, 192, 192>>,
    Transparent<IntroSectionMeta, LeadVocalPart01Cadence>,
);

#[derive(Flow)]
pub struct LeadVocalPart02(
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Transparent<IntroSectionMeta, LeadVocalTriple68Hold>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 480, 768>>,
    Step<Generate<58, 96, 96>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<61, 288, 672>>,
    Step<Generate<61, 192, 192>>,
    Transparent<IntroSectionMeta, LeadVocalTriple63Hold>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<66, 96, 96>>,
);

#[derive(Flow)]
pub struct LeadVocalPart03(
    Step<Generate<66, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 96, 96>>,
    Step<Generate<63, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 576>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 96, 576>>,
    Step<Generate<68, 96, 96>>,
);

#[derive(Flow)]
pub struct Sing68Triplet(
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
);

#[derive(Flow)]
pub struct Sing73Triplet(
    Step<Generate<73, 96, 96>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<73, 96, 96>>,
);

#[derive(Flow)]
pub struct TripleSing68Triplet(Double<Sing68Triplet>, Sing68Triplet);

#[derive(Flow)]
pub struct LeadVocalPart01Cadence(Double<Generate68Tick>, Generate68Hold);

#[derive(Flow)]
pub struct LeadVocalTriple68Hold(Double<Sing68Hold>, Sing68Hold);

#[derive(Flow)]
pub struct LeadVocalTriple63Hold(Double<Sing63Hold>, Sing63Hold);

#[derive(Flow)]
pub struct LeadVocalPart04(
    TripleSing68Triplet,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<70, 384, 672>>,
    Step<Generate<70, 480, 1056>>,
    Step<Generate<70, 384, 768>>,
    Step<Generate<70, 384, 672>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<73, 576, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
);

#[derive(Flow)]
pub struct LeadVocalPart05(
    Step<Generate<66, 96, 96>>,
    Step<Generate<71, 384, 384>>,
    Step<Generate<68, 192, 384>>,
    Step<Generate<59, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<71, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<71, 480, 480>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart06(
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 480, 768>>,
    Step<Generate<58, 96, 96>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 384, 768>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart07(
    Step<Generate<61, 96, 96>>,
    Step<Generate<63, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 576>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 384>>,
    Step<Generate<68, 384, 384>>,
    Step<Generate<66, 192, 864>>,
    Step<Generate<68, 96, 768>>,
    Step<Generate<68, 96, 480>>,
    Step<Generate<70, 384, 672>>,
    Step<Generate<70, 480, 1056>>,
    Step<Generate<70, 384, 768>>,
    Step<Generate<70, 384, 672>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<70, 96, 96>>,
);

#[derive(Flow)]
pub struct LeadVocalPart08(
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<73, 384, 12864>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<71, 384, 384>>,
    Step<Generate<68, 192, 384>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 384>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<71, 192, 192>>,
    Step<Generate<68, 96, 96>>,
);

#[derive(Flow)]
pub struct LeadVocalPart09(
    Step<Generate<71, 288, 288>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 128, 128>>,
    Step<Generate<68, 257, 129>>,
    Step<Generate<68, 0, 128>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 480, 768>>,
    Step<Generate<58, 96, 96>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 384, 768>>,
);

#[derive(Flow)]
pub struct LeadVocalPart10(
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 96, 96>>,
    Step<Generate<63, 288, 288>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<66, 288, 288>>,
    Step<Generate<63, 192, 576>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 384>>,
);

#[derive(Flow)]
pub struct LeadVocalPart11(
    Step<Generate<68, 384, 384>>,
    Step<Generate<66, 192, 960>>,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Step<Generate<70, 384, 672>>,
    Step<Generate<70, 480, 1056>>,
    Step<Generate<73, 576, 1344>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<70, 384, 7488>>,
    Step<Generate<60, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart12(
    Step<Generate<65, 192, 192>>,
    Step<Generate<68, 384, 384>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<63, 384, 384>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<63, 96, 96>>,
    Step<Generate<61, 288, 288>>,
    Step<Generate<61, 384, 960>>,
    Step<Generate<65, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<65, 192, 192>>,
    Step<Generate<63, 192, 192>>,
    Step<Generate<61, 192, 192>>,
    Step<Generate<61, 960, 1344>>,
    Step<Generate<59, 192, 384>>,
    Step<Generate<59, 192, 192>>,
    Step<Generate<58, 576, 960>>,
    Step<Generate<59, 192, 384>>,
    Step<Generate<59, 192, 192>>,
    Step<Generate<58, 576, 960>>,
    Step<Generate<59, 192, 384>>,
    Step<Generate<63, 768, 768>>,
    Step<Generate<68, 384, 768>>,
    Step<Generate<75, 1536, 1536>>,
);

#[derive(Flow)]
pub struct LeadVocalPart13(
    Step<Generate<75, 768, 768>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 192, 384>>,
    Step<Generate<70, 384, 40512>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<73, 257, 128>>,
    Step<Generate<73, 0, 129>>,
    Step<Generate<73, 512, 128>>,
    Step<Generate<73, 0, 1152>>,
);

#[derive(Flow)]
pub struct LeadVocalPart14(
    Step<Generate<71, 128, 128>>,
    Step<Generate<73, 129, 129>>,
    Step<Generate<70, 128, 128>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<73, 192, 1152>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<73, 384, 384>>,
    Step<Generate<66, 3456, 3456>>,
    Step<Generate<73, 384, 384>>,
    Step<Generate<71, 96, 96>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<73, 288, 288>>,
    Step<Generate<71, 192, 576>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<71, 96, 96>>,
    Step<Generate<75, 384, 384>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<73, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart15(
    Step<Generate<72, 192, 192>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<72, 192, 192>>,
    Step<Generate<73, 192, 192>>,
    Step<Generate<72, 96, 576>>,
    Sing73Triplet,
    Sing73Triplet,
    Sing73Triplet,
    Sing73Triplet,
    Step<Generate<75, 384, 672>>,
    Step<Generate<75, 480, 480>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<70, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart16(
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 384>>,
    Step<Generate<68, 384, 384>>,
    Step<Generate<66, 192, 672>>,
    Step<Generate<70, 288, 288>>,
    Step<Generate<70, 192, 672>>,
    Step<Generate<70, 288, 672>>,
    Step<Generate<69, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<70, 480, 864>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart17(
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 576>>,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Sing68Triplet,
    Step<Generate<70, 384, 672>>,
    Step<Generate<70, 288, 288>>,
    Step<Generate<70, 192, 192>>,
    Step<Generate<68, 96, 96>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 288, 288>>,
    Step<Generate<66, 192, 576>>,
    Step<Generate<70, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadVocalPart18(
    Step<Generate<70, 192, 192>>,
    Step<Generate<70, 96, 96>>,
    Step<Generate<73, 96, 96>>,
    Step<Generate<70, 384, 384>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 192>>,
    Step<Generate<68, 192, 192>>,
    Step<Generate<66, 192, 480>>,
    Step<Generate<75, 192, 192>>,
    Step<Generate<75, 96, 96>>,
    Step<Generate<70, 288, 288>>,
    Step<Generate<73, 288, 288>>,
    Step<Generate<70, 384, 480>>,
    Step<Generate<72, 288, 864>>,
    Step<Generate<73, 192, 192>>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::LeadVocalist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn full_song_journey_starts_and_stays_alive() {
        let client = LocalClient::builder()
            .namespace("welcome-lead-vocal-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&super::LeadVocalistSeed::default())
            .expect("seed should serialize");
        let journey_id = client
            .start_journey::<LeadVocalist>(seed)
            .await
            .expect("journey should start");

        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = client
            .journey_details(journey_id)
            .await
            .expect("journey details should be available");
        match status {
            JourneyStatus::Dead | JourneyStatus::Stopped => {
                panic!("journey reached terminal non-complete status: {status:?}");
            }
            JourneyStatus::Created | JourneyStatus::Alive | JourneyStatus::Completed => {}
        }

        worker_handle.abort();
        let _ = worker_handle.await;
    }
}
