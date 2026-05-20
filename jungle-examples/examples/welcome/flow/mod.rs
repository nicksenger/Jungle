use std::time::Duration;

use jungle_sdk::prelude::*;

use crate::{
    animals::RhythmGuitarist,
    effects,
};

#[derive(Default)]
pub struct RhythmGuitarIntroState {
    intro_note_index: u16,
}

pub struct RhythmGuitarIntroRemaining;
impl LoopCondition<RhythmGuitarIntroState> for RhythmGuitarIntroRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitarIntroState) -> bool {
        state.intro_note_index < crate::score::rhythm_guitar_intro_len()
    }
}

pub struct PlayRhythmGuitarIntroNote;
impl BoundAct<RhythmGuitarist> for PlayRhythmGuitarIntroNote {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(
        state: &RhythmGuitarIntroState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        state.intro_note_index
    }

    fn absorb(
        state: &mut RhythmGuitarIntroState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("rhythm guitar intro note should play successfully");
        state.intro_note_index = state.intro_note_index.saturating_add(1);
    }
}

pub struct PlayRhythmGuitarIntroNoteSpec;
#[jungle::act(bind = PlayRhythmGuitarIntroNote)]
impl Act for PlayRhythmGuitarIntroNoteSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct SleepFiveMinutesAfterIntroSpec;
#[jungle::act]
impl Act for SleepFiveMinutesAfterIntroSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitarIntroState, _input: Self::Input) -> Duration {
        Duration::from_secs(5 * 60)
    }

    fn absorb(_state: &mut RhythmGuitarIntroState, output: EffectCompletion<Self::Effect>) {
        output.expect("sleep step should complete after worker wakeup");
    }
}

pub type RhythmGuitarIntroLoopBody = Step<PlayRhythmGuitarIntroNoteSpec>;

#[derive(Flow)]
pub struct RhythmGuitarIntroFlow(While<RhythmGuitarIntroRemaining, RhythmGuitarIntroLoopBody>);

#[derive(Flow)]
pub struct RhythmGuitaristJourney(RhythmGuitarIntroFlow, Step<SleepFiveMinutesAfterIntroSpec>);
