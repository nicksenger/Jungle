use std::{marker::PhantomData, time::Duration};

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{animals::RhythmGuitarist, effects};

const INTRO_FIRST_BAR: u8 = 1;
const INTRO_LAST_BAR: u8 = 17;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlayNoteCommand {
    pub bar: u8,
    pub beat: u8,
    pub beat_offset_num: u8,
    pub beat_offset_den: u8,
    pub duration_num: u8,
    pub duration_den: u8,
    pub midi: u8,
}

#[derive(Clone, Default)]
pub struct RhythmGuitarIntroState {
    current_bar: u8,
    bar_root_midi: u8,
}

impl RhythmGuitarIntroState {
    fn root_for_current_bar(&self) -> u8 {
        match self.current_bar {
            1..=2 => 58,
            3..=4 => 53,
            5..=6 => 49,
            7..=8 => 46,
            9..=10 => 44,
            11..=12 => 46,
            13..=14 => 49,
            15..=16 => 53,
            _ => 58,
        }
    }
}

pub struct IntroBarsRemaining;
impl LoopCondition<RhythmGuitarIntroState> for IntroBarsRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitarIntroState) -> bool {
        let bar = if state.current_bar == 0 {
            INTRO_FIRST_BAR
        } else {
            state.current_bar
        };
        bar <= INTRO_LAST_BAR
    }
}

pub struct IntroBarIsTurnaround;
impl Condition<(RhythmGuitarIntroState, ())> for IntroBarIsTurnaround {
    fn choose((state, _): &(RhythmGuitarIntroState, ())) -> bool {
        let bar = state.current_bar;
        bar == 4 || bar == 8 || bar == 12 || bar == 16 || bar == INTRO_LAST_BAR
    }
}

pub struct IntroBarIsLowRegister;
impl Condition<(RhythmGuitarIntroState, ())> for IntroBarIsLowRegister {
    fn choose((state, _): &(RhythmGuitarIntroState, ())) -> bool {
        (9..=12).contains(&state.current_bar)
    }
}

pub struct SetBarProfile;
impl BoundAct<RhythmGuitarist> for SetBarProfile {
    type Effect = effects::SetRhythmGuitarBarProfile;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitarIntroState, _input: Self::Input) -> () {
    }

    fn absorb(state: &mut RhythmGuitarIntroState, output: EffectCompletion<Self::Effect>) {
        output.expect("bar profile step should succeed");
        if state.current_bar == 0 {
            state.current_bar = INTRO_FIRST_BAR;
        }
        state.bar_root_midi = state.root_for_current_bar();
    }
}

pub struct SetBarProfileSpec;
#[jungle::act(bind = SetBarProfile)]
impl Act for SetBarProfileSpec {
    type Effect = effects::SetRhythmGuitarBarProfile;
    type Input = ();
    type Output = ();
}

pub struct PlaySlot<
    A,
    const BEAT: u8,
    const OFFSET_NUM: u8,
    const OFFSET_DEN: u8,
    const DUR_NUM: u8,
    const DUR_DEN: u8,
    const INTERVAL: i8,
>(PhantomData<fn() -> A>);

impl<
    A,
    const BEAT: u8,
    const OFFSET_NUM: u8,
    const OFFSET_DEN: u8,
    const DUR_NUM: u8,
    const DUR_DEN: u8,
    const INTERVAL: i8,
> BoundAct<A> for PlaySlot<A, BEAT, OFFSET_NUM, OFFSET_DEN, DUR_NUM, DUR_DEN, INTERVAL>
where
    A: Animal<State = RhythmGuitarIntroState>,
{
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &RhythmGuitarIntroState, _input: Self::Input) -> PlayNoteCommand {
        let root = state.bar_root_midi as i16;
        let midi = (root + INTERVAL as i16).clamp(0, 127) as u8;
        PlayNoteCommand {
            bar: state.current_bar,
            beat: BEAT,
            beat_offset_num: OFFSET_NUM,
            beat_offset_den: OFFSET_DEN,
            duration_num: DUR_NUM,
            duration_den: DUR_DEN,
            midi,
        }
    }

    fn absorb(_state: &mut RhythmGuitarIntroState, output: EffectCompletion<Self::Effect>) {
        output.expect("rhythm guitar intro note should play successfully");
    }
}

pub struct PlayBasePickupSpec;
#[jungle::act(bind = PlaySlot<A, 1, 0, 1, 1, 4, 0>)]
impl Act for PlayBasePickupSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct PlayTurnaroundPickupSpec;
#[jungle::act(bind = PlaySlot<A, 1, 0, 1, 1, 4, -2>)]
impl Act for PlayTurnaroundPickupSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct PlayPulseASpec;
#[jungle::act(bind = PlaySlot<A, 2, 0, 1, 1, 4, 0>)]
impl Act for PlayPulseASpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct PlayLowPulseSpec;
#[jungle::act(bind = PlaySlot<A, 3, 1, 2, 1, 4, 5>)]
impl Act for PlayLowPulseSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct PlayHighPulseSpec;
#[jungle::act(bind = PlaySlot<A, 3, 1, 2, 1, 4, 7>)]
impl Act for PlayHighPulseSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct PlayResolveSpec;
#[jungle::act(bind = PlaySlot<A, 4, 3, 4, 1, 4, 0>)]
impl Act for PlayResolveSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = ();
}

pub struct MergeEither;
impl BoundAct<RhythmGuitarist> for MergeEither {
    type Effect = effects::MergeRhythmEither;
    type Aspect = Identity;
    type Input = Either<(), ()>;
    type Output = ();

    fn emit(_state: &RhythmGuitarIntroState, _input: Self::Input) {}

    fn absorb(_state: &mut RhythmGuitarIntroState, output: EffectCompletion<Self::Effect>) {
        output.expect("either merge should succeed");
    }
}

pub struct MergeEitherSpec;
#[jungle::act(bind = MergeEither)]
impl Act for MergeEitherSpec {
    type Effect = effects::MergeRhythmEither;
    type Input = Either<(), ()>;
    type Output = ();
}

pub struct AdvanceBar;
impl BoundAct<RhythmGuitarist> for AdvanceBar {
    type Effect = effects::AdvanceRhythmGuitarBar;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(state: &RhythmGuitarIntroState, _input: Self::Input) -> u8 {
        state.current_bar
    }

    fn absorb(state: &mut RhythmGuitarIntroState, output: EffectCompletion<Self::Effect>) {
        state.current_bar = output.expect("bar advancement should succeed");
    }
}

pub struct AdvanceBarSpec;
#[jungle::act(bind = AdvanceBar)]
impl Act for AdvanceBarSpec {
    type Effect = effects::AdvanceRhythmGuitarBar;
    type Input = ();
    type Output = ();
}

#[derive(Flow)]
pub struct IntroBarFlow(
    Step<SetBarProfileSpec>,
    Conditional<IntroBarIsTurnaround, Step<PlayTurnaroundPickupSpec>, Step<PlayBasePickupSpec>>,
    Step<MergeEitherSpec>,
    Step<PlayPulseASpec>,
    Conditional<IntroBarIsLowRegister, Step<PlayLowPulseSpec>, Step<PlayHighPulseSpec>>,
    Step<MergeEitherSpec>,
    Step<PlayResolveSpec>,
    Step<AdvanceBarSpec>,
);

#[derive(Flow)]
pub struct RhythmGuitarIntroFlow(While<IntroBarsRemaining, IntroBarFlow>);

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

#[derive(Flow)]
pub struct RhythmGuitaristJourney(RhythmGuitarIntroFlow, Step<SleepFiveMinutesAfterIntroSpec>);
