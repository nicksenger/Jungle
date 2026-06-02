use std::{f32::consts::TAU, sync::Arc, time::Duration};

use self::speech_synthesis::{
    parser, reciter,
    singer::{render_vocal_note, LyricInput, VocalNote, VoiceParams},
};
use tracing::warn;

use super::{PlaybackLayer, VocalsArticulation};
use crate::dsp::{
    duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, Expression, Note,
    SAMPLE_RATE,
};

pub mod speech_synthesis;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Phoneme {
    pub length: u8,
    pub index: usize,
    pub stress: u8,
}

const FORMANT_PLAYBACK_LAYERS: [PlaybackLayer; 3] = [
    PlaybackLayer {
        pan: 0.02,
        gain_scale: 1.0,
        playback_rate_scale: 1.0,
        delay_seconds: 0.0,
    },
    PlaybackLayer {
        pan: -0.17,
        gain_scale: 0.26,
        playback_rate_scale: 0.998,
        delay_seconds: 0.008,
    },
    PlaybackLayer {
        pan: 0.21,
        gain_scale: 0.21,
        playback_rate_scale: 1.004,
        delay_seconds: 0.015,
    },
];

pub fn articulation_layers() -> &'static [PlaybackLayer] {
    &FORMANT_PLAYBACK_LAYERS
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

pub fn synthesize_vocals(
    note: &Note<VocalsArticulation>,
    phonemes: [Option<Phoneme>; 12],
) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let pcm =
        synthesize_formant_vocals(note.n_midi, duration, phonemes, note.velocity, SAMPLE_RATE)
            .unwrap_or_else(|| synthesize_procedural_fallback(note, duration));
    let (gain, playback_rate) = articulation_output_shape();
    (pcm, gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    Duration::from_secs_f32((base.as_secs_f32() * 1.05).max(0.03))
}

fn articulation_output_shape() -> (f32, f32) {
    (1.66, 1.0)
}

fn synthesize_procedural_fallback(
    note: &Note<VocalsArticulation>,
    duration: Duration,
) -> Arc<[f32]> {
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(85.0, 1_300.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(base_hz, phase, t, expression);
        let env = articulation_envelope(phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    Arc::from(pcm)
}

fn articulation_sample(base_hz: f32, phase: f32, t: f32, expression: Expression) -> f32 {
    let vibrato = (TAU * 5.7 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.02;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.15;
    let f = base_hz * (1.0 + bend + vibrato);
    reed_formant(f, t, phase)
}

fn reed_formant(frequency_hz: f32, t: f32, phase: f32) -> f32 {
    let mouthpiece = smoothstep((phase / 0.12).clamp(0.0, 1.0));
    let body = sine(frequency_hz, t) * 0.62 + saw(frequency_hz, t) * 0.17;
    let warmth = sine(frequency_hz * 2.0, t) * 0.18 + sine(frequency_hz * 3.0, t) * 0.08;
    let breath = hash_noise(t * 6_100.0) * (0.03 + 0.03 * (1.0 - mouthpiece));
    (body + warmth + breath).tanh()
}

fn articulation_envelope(phase: f32) -> f32 {
    let attack = 0.03;
    let body = 0.9;
    let release_start = 0.8;
    let release = 0.18;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let release_phase = ((phase - release_start) / release).clamp(0.0, 1.0);
    let release_env = 1.0 - smoothstep(release_phase);
    (attack_env * body * release_env).clamp(0.0, 1.0)
}
