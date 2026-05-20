use std::time::Duration;

use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    animals::RhythmGuitarist,
    effects,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RhythmGuitarIntroNoteSpec {
    pub bar: u8,
    pub beat: u8,
    pub beat_offset_num: u8,
    pub beat_offset_den: u8,
    pub duration_num: u8,
    pub duration_den: u8,
    pub midi: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmGuitarIntroBarSpec {
    pub bar: u8,
    pub notes: Vec<RhythmGuitarIntroNoteSpec>,
}

pub struct RhythmGuitarIntroState {
    intro_bar_index: usize,
    intro_bars: Vec<RhythmGuitarIntroBarSpec>,
}

impl Default for RhythmGuitarIntroState {
    fn default() -> Self {
        Self {
            intro_bar_index: 0,
            intro_bars: build_intro_bars(),
        }
    }
}

pub struct RhythmGuitarIntroRemaining;
impl LoopCondition<RhythmGuitarIntroState> for RhythmGuitarIntroRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitarIntroState) -> bool {
        state.intro_bar_index < state.intro_bars.len()
    }
}

pub struct PlayRhythmGuitarIntroBar;
impl BoundAct<RhythmGuitarist> for PlayRhythmGuitarIntroBar {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Aspect = Identity;
    type Input = ();
    type Output = RhythmGuitarIntroBarSpec;

    fn emit(
        state: &RhythmGuitarIntroState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        state.intro_bars[state.intro_bar_index].clone()
    }

    fn absorb(
        state: &mut RhythmGuitarIntroState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let played = output.expect("rhythm guitar intro bar should play successfully");
        state.intro_bar_index = state.intro_bar_index.saturating_add(1);
        played
    }
}

pub struct PlayRhythmGuitarIntroBarSpec;
#[jungle::act(bind = PlayRhythmGuitarIntroBar)]
impl Act for PlayRhythmGuitarIntroBarSpec {
    type Effect = effects::PlayRhythmGuitarIntroNote;
    type Input = ();
    type Output = RhythmGuitarIntroBarSpec;
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

pub type RhythmGuitarIntroLoopBody = Step<PlayRhythmGuitarIntroBarSpec>;

#[derive(Flow)]
pub struct RhythmGuitarIntroFlow(While<RhythmGuitarIntroRemaining, RhythmGuitarIntroLoopBody>);

#[derive(Flow)]
pub struct RhythmGuitaristJourney(RhythmGuitarIntroFlow, Step<SleepFiveMinutesAfterIntroSpec>);

fn build_intro_bars() -> Vec<RhythmGuitarIntroBarSpec> {
    let mut bars: Vec<RhythmGuitarIntroBarSpec> = Vec::new();

    for note in crate::score::rhythm_guitar_intro_grid() {
        let bar_number = note.position.bar as u8;
        if bars.last().is_none_or(|bar| bar.bar != bar_number) {
            bars.push(RhythmGuitarIntroBarSpec {
                bar: bar_number,
                notes: Vec::new(),
            });
        }

        let (duration_num, duration_den) = duration_fraction(note.beats);
        let next_note = RhythmGuitarIntroNoteSpec {
            bar: bar_number,
            beat: note.position.beat as u8,
            beat_offset_num: note.position.beat_offset_num as u8,
            beat_offset_den: note.position.beat_offset_den as u8,
            duration_num,
            duration_den,
            midi: note.midi,
        };
        bars.last_mut()
            .expect("intro bars should contain an active bar before inserting note")
            .notes
            .push(next_note);
    }

    bars
}

fn duration_fraction(beats: f32) -> (u8, u8) {
    if (beats - 0.25).abs() < f32::EPSILON {
        return (1, 4);
    }
    if (beats - 0.5).abs() < f32::EPSILON {
        return (1, 2);
    }
    if (beats - 1.0).abs() < f32::EPSILON {
        return (1, 1);
    }
    if (beats - 2.0).abs() < f32::EPSILON {
        return (2, 1);
    }
    (1, 4)
}
