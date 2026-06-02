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
    let f = base_hz.clamp(90.0, 880.0);
    let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
    unison + sine(f * 2.0, t) * 0.15 + hash_noise(t * 4_500.0) * 0.08
}

fn articulation_envelope(phase: f32) -> f32 {
    let attack = 0.04;
    let decay = 0.45;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
