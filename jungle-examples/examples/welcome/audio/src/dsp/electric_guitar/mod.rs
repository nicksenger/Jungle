use std::sync::Arc;

use super::{Expression, Note, SAMPLE_RATE};

mod lead;
mod rhythm;

#[derive(Debug, Clone, Copy)]
pub enum ElectricGuitarArticulation {
    Sustained,
    RhythmSustained,
}
impl ElectricGuitarArticulation {
    fn is_rhythm_voice(self) -> bool {
        matches!(self, Self::RhythmSustained)
    }
}

pub fn synthesize_electric_guitar(
    note: &Note<ElectricGuitarArticulation>,
) -> (Arc<[f32]>, f32, f32, f32) {
    if note.articulation.is_rhythm_voice() {
        let (pcm, gain, playback_rate) = lead::synthesize_lead_guitar(note);
        (pcm, gain, playback_rate, -0.25)
    } else {
        let (pcm, gain, playback_rate) = rhythm::synthesize_rhythm_guitar(note);
        (pcm, gain, playback_rate, 0.12)
    }
}
