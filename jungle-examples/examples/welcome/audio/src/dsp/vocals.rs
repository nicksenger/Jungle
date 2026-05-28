use std::{f32::consts::TAU, sync::Arc, time::Duration};

use super::{
    duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, Expression, Note,
    SAMPLE_RATE,
};

#[derive(Debug, Clone, Copy)]
pub enum VocalsArticulation {
    Clean,
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
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => &[
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
        ],
        VocalsArticulation::GroupHarmony => &[
            PlaybackLayer {
                pan: -0.42,
                gain_scale: 0.7,
                playback_rate_scale: 0.992,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 0.9,
                playback_rate_scale: 1.0,
                delay_seconds: 0.011,
            },
            PlaybackLayer {
                pan: 0.38,
                gain_scale: 0.68,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
    }
}

pub fn synthesize_vocals(
    note: &Note<VocalsArticulation>,
) -> (Arc<[f32]>, f32, f32) {
    if let VocalsArticulation::Formant(phonemes) = note.articulation {
        if let Some(formant) = synthesize_formant_vocals(note, phonemes) {
            return formant;
        }
    }

    let duration = articulation_duration(note.duration, note.articulation);
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
        let sample = articulation_sample(note.articulation, base_hz, phase, t, expression);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn synthesize_formant_vocals(
    note: &Note<VocalsArticulation>,
    phonemes: [Option<crate::vocals::Phoneme>; 12],
) -> Option<(Arc<[f32]>, f32, f32)> {
    let duration = articulation_duration(note.duration, note.articulation);
    let pcm = crate::vocals::synthesize_formant_vocals(
        note.n_midi,
        duration,
        phonemes,
        note.velocity,
        SAMPLE_RATE,
    )?;
    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    Some((pcm, gain, playback_rate))
}

fn articulation_duration(base: Duration, articulation: VocalsArticulation) -> Duration {
    let scale = match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => 1.05,
        VocalsArticulation::GroupHarmony => 1.0,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: VocalsArticulation) -> (f32, f32) {
    match articulation {
        VocalsArticulation::Clean => (0.83, 1.0),
        VocalsArticulation::Formant(_) => (1.66, 1.0),
        VocalsArticulation::GroupHarmony => (0.78, 1.0),
    }
}

fn articulation_sample(
    articulation: VocalsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let vibrato = (TAU * 5.7 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.02;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.15;

    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => {
            let f = base_hz * (1.0 + bend + vibrato);
            reed_formant(f, t, phase)
        }
        VocalsArticulation::GroupHarmony => {
            let f = base_hz.clamp(90.0, 880.0);
            let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
            unison + sine(f * 2.0, t) * 0.15 + hash_noise(t * 4_500.0) * 0.08
        }
    }
}

fn reed_formant(frequency_hz: f32, t: f32, phase: f32) -> f32 {
    let mouthpiece = smoothstep((phase / 0.12).clamp(0.0, 1.0));
    let body = sine(frequency_hz, t) * 0.62 + saw(frequency_hz, t) * 0.17;
    let warmth = sine(frequency_hz * 2.0, t) * 0.18 + sine(frequency_hz * 3.0, t) * 0.08;
    let breath = hash_noise(t * 6_100.0) * (0.03 + 0.03 * (1.0 - mouthpiece));
    (body + warmth + breath).tanh()
}

fn articulation_envelope(articulation: VocalsArticulation, phase: f32) -> f32 {
    match articulation {
        VocalsArticulation::Clean | VocalsArticulation::Formant(_) => {
            let attack = 0.03;
            let body = 0.9;
            let release_start = 0.8;
            let release = 0.18;
            let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
            let release_phase = ((phase - release_start) / release).clamp(0.0, 1.0);
            let release_env = 1.0 - smoothstep(release_phase);
            (attack_env * body * release_env).clamp(0.0, 1.0)
        }
        VocalsArticulation::GroupHarmony => {
            let attack = 0.04;
            let decay = 0.45;
            let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
            let decay_env = (-phase * decay * 4.0).exp();
            (attack_env * decay_env).clamp(0.0, 1.0)
        }
    }
}
