use std::sync::Arc;

use super::Note;

pub mod backup;
pub mod formant;

#[derive(Debug, Clone, Copy)]
pub enum VocalsArticulation {
    GroupHarmony,
    Formant([Option<crate::vocals::Phoneme>; 12]),
}

#[derive(Clone, Copy)]
pub struct PlaybackLayer {
    pub pan: f32,
    pub gain_scale: f32,
    pub playback_rate_scale: f32,
    pub delay_seconds: f32,
}

pub fn articulation_layers(articulation: VocalsArticulation) -> &'static [PlaybackLayer] {
    match articulation {
        VocalsArticulation::GroupHarmony => backup::articulation_layers(),
        VocalsArticulation::Formant(_) => formant::articulation_layers(),
    }
}

pub fn synthesize_vocals(note: &Note<VocalsArticulation>) -> (Arc<[f32]>, f32, f32) {
    match note.articulation {
        VocalsArticulation::GroupHarmony => backup::synthesize_vocals(note),
        VocalsArticulation::Formant(phonemes) => formant::synthesize_vocals(note, phonemes),
    }
}
