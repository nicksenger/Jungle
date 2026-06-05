use std::{f32::consts::TAU, sync::Arc, time::Duration};

pub use self::speech_synthesis::singer::{phonemes_from_text, synthesize_formant_vocals, Phoneme};

use super::{PlaybackLayer, VocalsArticulation};
use crate::dsp::{
    duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, Expression, Note,
    SAMPLE_RATE,
};

pub mod speech_synthesis;

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
