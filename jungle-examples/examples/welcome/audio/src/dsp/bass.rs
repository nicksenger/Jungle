use std::{sync::Arc, time::Duration};

use super::{
    duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, Expression, Note,
    SAMPLE_RATE,
};

pub fn synthesize_bass(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(30.0, 200.0);
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

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 1.12;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape() -> (f32, f32) {
    (0.93, 1.0)
}

fn articulation_sample(base_hz: f32, phase: f32, t: f32, expression: Expression) -> f32 {
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.14;
    let vibrato =
        (std::f32::consts::TAU * 5.2 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let freq = base_hz * (1.0 + bend + vibrato);

    // Enhanced transient for sharper attack and better spectral definition
    let transient = hash_noise(t * 18_000.0) * (1.0 - smoothstep(phase * 15.0));

    // Adjusted harmonics to match target energy distribution
    // Use sawtooth for rich harmonics and sine for clean sub/fundamental
    let growl = saw(freq, t) * 0.30 + saw(freq * 2.0, t) * 0.15;
    let low_body = sine(freq, t) * 0.40 + sine(freq * 0.5, t) * 0.25;

    // Boost transient for sharper attack and better spectral definition
    ((growl + low_body + transient * 0.20) * 1.1).tanh()
}

fn articulation_envelope(phase: f32) -> f32 {
    let attack = 0.006;
    let decay = 0.45;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
