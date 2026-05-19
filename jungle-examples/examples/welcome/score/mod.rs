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
    pub score: &'static str,
    pub midi: u8,
    pub start_tick: u32,
    pub duration_tick: u32,
    pub kind: Kind,
    pub beats: f32,
    pub seconds: f32,
    pub position: Position,
}
