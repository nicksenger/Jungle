use std::{sync::Arc, time::Duration};

use super::{PlaybackLayer, VocalsArticulation};
use crate::dsp::{duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, Note, SAMPLE_RATE};

const BACKUP_PLAYBACK_LAYERS: [PlaybackLayer; 3] = [
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
];

pub fn articulation_layers() -> &'static [PlaybackLayer] {
    &BACKUP_PLAYBACK_LAYERS
}

pub fn synthesize_vocals(note: &Note<VocalsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(85.0, 1_300.0);
    let velocity = note.velocity.clamp(0.0, 1.0);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(base_hz, t);
        let env = articulation_envelope(phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    Duration::from_secs_f32(base.as_secs_f32().max(0.03))
}

fn articulation_output_shape() -> (f32, f32) {
    (0.78, 1.0)
}

fn articulation_sample(base_hz: f32, t: f32) -> f32 {
    // Use base frequency to create stronger harmonics at target bands
    let f = base_hz.clamp(90.0, 880.0);

    // Enhanced unison with wider detuning for better harmonic spread
    let unison = sine(f * 0.993, t) * 0.35 + sine(f, t) * 0.3 + sine(f * 1.005, t) * 0.35;

    // Formant-like resonances for vocal character - adjusted for target bands
    let formant_1 = sine(f * 1.5, t) * 0.12 + sine(f * 2.0, t) * 0.08;
    let formant_2 = sine(f * 3.0, t) * 0.15 + sine(f * 3.5, t) * 0.10;
    let formant_3 = sine(f * 4.0, t) * 0.08 + sine(f * 4.5, t) * 0.05;

    // Boosted mid harmonics to raise spectral centroid toward 1713 Hz target
    let harmonic_2 = sine(f * 2.0, t) * 0.22;
    let harmonic_3 = sine(f * 3.0, t) * 0.16;
    let harmonic_4 = sine(f * 4.0, t) * 0.10;
    let harmonic_5 = sine(f * 5.0, t) * 0.06;
    let harmonic_6 = sine(f * 6.0, t) * 0.03;
    let harmonic_7 = sine(f * 7.0, t) * 0.015;
    let harmonic_8 = sine(f * 8.0, t) * 0.008;
    let harmonic_9 = sine(f * 9.0, t) * 0.004;
    let harmonic_10 = sine(f * 10.0, t) * 0.002;

    // Reduced sub-harmonic to avoid lowering spectral centroid too much
    let sub_harmonic = sine(f * 0.5, t) * 0.10;

    // Add noise for spectral flatness and texture
    let noise = hash_noise(t * 4_500.0) * 0.04;

    let total = unison
        + formant_1
        + formant_2
        + formant_3
        + harmonic_2
        + harmonic_3
        + harmonic_4
        + harmonic_5
        + harmonic_6
        + harmonic_7
        + harmonic_8
        + harmonic_9
        + harmonic_10
        + sub_harmonic
        + noise;

    total
}

fn articulation_envelope(phase: f32) -> f32 {
    // Slightly faster attack for better spectral definition
    let attack = 0.03;
    let decay = 0.5;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 3.5).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
