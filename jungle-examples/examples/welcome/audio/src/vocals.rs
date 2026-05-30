use std::{sync::Arc, time::Duration};

use crate::speech_synthesis::{
    parser, reciter,
    singer::{render_vocal_note, LyricInput, VocalNote, VoiceParams},
};
use tracing::warn;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Phoneme {
    pub length: u8,
    pub index: usize,
    pub stress: u8,
}

pub fn phonemes_from_text(text: &str) -> [Option<Phoneme>; 12] {
    let mut output = [None; 12];

    let parsed_text = match reciter::text_to_phonemes(text) {
        Ok(parsed_text) => parsed_text,
        Err(err) => {
            warn!(word = text, error = %err, "failed to recite text into rustsam phonemes");
            return output;
        }
    };

    let parsed_phonemes = match parser::parse_phonemes(&parsed_text) {
        Ok(parsed_phonemes) => parsed_phonemes,
        Err(err) => {
            warn!(word = text, error = %err, "failed to parse rustsam phoneme string");
            return output;
        }
    };

    if parsed_phonemes.len() > output.len() {
        warn!(
            word = text,
            parsed_count = parsed_phonemes.len(),
            max_count = output.len(),
            "truncating parsed rustsam phonemes to fit vocals articulation capacity"
        );
    }

    for (slot, phoneme) in output.iter_mut().zip(parsed_phonemes.into_iter()) {
        *slot = Some(Phoneme {
            length: phoneme.length,
            index: phoneme.index,
            stress: phoneme.stress,
        });
    }

    output
}

pub fn synthesize_formant_vocals(
    midi_note: u8,
    duration: Duration,
    phonemes: [Option<Phoneme>; 12],
    velocity: f32,
    sample_rate: u32,
) -> Option<Arc<[f32]>> {
    const FORMANT_TARGET_RMS: f32 = 0.22;
    const FORMANT_TARGET_PEAK: f32 = 0.92;
    const FORMANT_MAX_MAKEUP_GAIN: f32 = 4.0;

    let phonemes: Vec<parser::Phoneme> = phonemes
        .into_iter()
        .flatten()
        .map(|phoneme| parser::Phoneme {
            length: phoneme.length,
            index: phoneme.index,
            stress: phoneme.stress,
        })
        .collect();

    if phonemes.is_empty() {
        return None;
    }

    let voice = VoiceParams::default();
    let rendered = render_vocal_note(
        VocalNote {
            midi_note,
            lyric: LyricInput::Phonemes(phonemes),
            duration,
        },
        sample_rate,
        voice,
    );

    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(err) => {
            warn!(error = %err, "failed to render rustsam formant vocals; using procedural fallback");
            return None;
        }
    };

    let mut normalized: Vec<f32> = rendered
        .into_iter()
        .map(|sample| sample as f32 / 127.5 - 1.0)
        .collect();

    let peak_abs = normalized.iter().fold(0.0_f32, |acc, sample| {
        if sample.abs() > acc {
            sample.abs()
        } else {
            acc
        }
    });
    let rms = if normalized.is_empty() {
        0.0
    } else {
        (normalized.iter().map(|sample| sample * sample).sum::<f32>() / normalized.len() as f32)
            .sqrt()
    };

    if peak_abs > 1.0e-4 && rms > 1.0e-4 {
        let rms_gain = FORMANT_TARGET_RMS / rms;
        let peak_gain = FORMANT_TARGET_PEAK / peak_abs;
        let makeup_gain = rms_gain.min(peak_gain).clamp(1.0, FORMANT_MAX_MAKEUP_GAIN);
        if makeup_gain > 1.0 {
            for sample in &mut normalized {
                *sample = (*sample * makeup_gain).clamp(-1.0, 1.0);
            }
        }
    }

    let velocity = velocity.clamp(0.0, 1.0);
    let pcm: Vec<f32> = normalized
        .into_iter()
        .map(|sample| sample * velocity)
        .collect();
    Some(Arc::from(pcm))
}
