use std::time::Duration;

use crate::instrumentation::{ElectricGuitarArticulation, Note};

mod backup_vocals;
mod bass_drum;
mod bass_guitar;
mod closed_hi_hat_cymbal;
mod crash_cymbal;
mod lead_guitar;
mod rhythm_guitar;
mod snare_drum;
mod toms_snare;
mod vocals;

pub use backup_vocals::backup_vocals_score;
pub use bass_drum::bass_drum_score;
pub use bass_guitar::bass_guitar_score;
pub use closed_hi_hat_cymbal::closed_hi_hat_cymbal_score;
pub use crash_cymbal::crash_cymbal_score;
pub use lead_guitar::lead_guitar_score;
pub use rhythm_guitar::rhythm_guitar_score;
pub use snare_drum::snare_drum_score;
pub use toms_snare::toms_snare_score;
pub use vocals::vocals_score;

const BEATS_PER_BAR: f32 = 4.0;

/// Musical duration bucket for notes extracted from tick-based score events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Whole,
    DottedHalf,
    Half,
    DottedQuarter,
    Quarter,
    DottedEighth,
    Eighth,
    DottedSixteenth,
    Sixteenth,
    ThirtySecond,
    Fractional { numerator: u32, denominator: u32 },
}

/// Position of a note relative to a meter grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub bar: u32,
    pub beat: u32,
    pub beat_offset_num: u32,
    pub beat_offset_den: u32,
}

/// Extracted note timing data from the legacy absolute-tick score representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridNote {
    pub midi: u8,
    pub kind: Kind,
    pub beats: f32,
    pub d_sec: f32,
    t_sec: f32,
    pub position: Position,
}

pub(super) fn grid_note_to_note(note: GridNote, bpm: f32) -> Note<ElectricGuitarArticulation> {
    let seconds_per_beat = 60.0_f32 / bpm;
    let beat_offset = if note.position.beat_offset_den == 0 {
        0.0
    } else {
        note.position.beat_offset_num as f32 / note.position.beat_offset_den as f32
    };
    let absolute_beats = ((note.position.bar - 1) as f32 * BEATS_PER_BAR)
        + (note.position.beat - 1) as f32
        + beat_offset;
    let offset = Duration::from_secs_f32(absolute_beats * seconds_per_beat);

    Note {
        n_midi: note.midi,
        duration: Duration::from_secs_f32(note.beats * seconds_per_beat),
        velocity: 37.0 / 127.0,
        amplitude_multiplier: 0.5,
        pan: 0.5,
        expression: None,
        offset,
        articulation: ElectricGuitarArticulation::Sustained,
    }
}

pub(super) fn collect_score_from_sections(
    sections: &[&[GridNote]],
    bpm: f32,
) -> Vec<Note<ElectricGuitarArticulation>> {
    let total_notes = sections.iter().map(|section| section.len()).sum();
    let mut notes = Vec::with_capacity(total_notes);

    for section in sections {
        for &grid_note in *section {
            notes.push(grid_note_to_note(grid_note, bpm));
        }
    }

    debug_assert!(
        notes.windows(2).all(|pair| pair[0].offset <= pair[1].offset),
        "welcome score notes are not monotonic by offset"
    );

    notes
}
