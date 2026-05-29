use std::{sync::Arc, time::Duration};

use super::{duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, Note, SAMPLE_RATE};

pub fn synthesize_kick_drum(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(48.0, 74.0);
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.72);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(base_hz, phase, t, velocity);
        let env = articulation_envelope(phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 0.35;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape() -> (f32, f32) {
    (1.28, 1.0)
}

fn articulation_sample(base_hz: f32, phase: f32, t: f32, velocity: f32) -> f32 {
    let pitch_env = 1.0 - smoothstep((phase * 5.8).clamp(0.0, 1.0));
    let sweep_hz = base_hz * (1.0 + pitch_env * 1.85);
    let sub = sine(sweep_hz, t);
    let punch = sine(base_hz * (1.95 + pitch_env * 0.45), t) * (-phase * 14.0).exp();
    let ring = sine(base_hz * 2.8, t) * (-phase * 10.5).exp();
    let beater_noise = (hash_noise(t * 14_600.0) - hash_noise(t * 2_300.0) * 0.55)
        * (1.0 - smoothstep((phase * 18.0).clamp(0.0, 1.0)));
    let beater_tone =
        sine(1_900.0 + pitch_env * 640.0, t) * (1.0 - smoothstep((phase * 26.0).clamp(0.0, 1.0)));
    let click = (beater_noise * 0.9 + beater_tone * 0.55) * (0.65 + velocity * 0.5);

    sub * 0.88 + punch * 0.42 + ring * 0.18 + click * 0.34
}

fn articulation_envelope(phase: f32) -> f32 {
    let attack = 0.0012;
    let (body_decay, tail_decay) = (1.15, 2.3);
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let body = (-phase * body_decay * 4.2).exp();
    let tail = (-phase * tail_decay * 9.4).exp();
    (attack_env * (body * 0.74 + tail * 0.26)).clamp(0.0, 1.0)
}
