use std::{sync::Arc, time::Duration};

use crate::audio::{PlayPriority, PlayRequest};
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Note,
};

use super::SnareDrumArticulation;

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<SnareDrumArticulation>,
) -> Result<(), Error> {
    let (pcm, mut gain, mut playback_rate) = synth.snare_drum(note).await?;

    let velocity = note.velocity.clamp(0.0, 1.0);
    gain *= 0.88 + velocity * 0.52;
    playback_rate *= 0.98 + velocity * 0.06;

    let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.08 + (velocity - 0.5) * 0.06;
    request.priority = PlayPriority::Critical;
    audio.play(request).await.map_err(|_| Error::Submission)
}

pub(in crate::instrumentation) fn synthesize_snare_drum(
    note: &Note<SnareDrumArticulation>,
) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let body_hz = midi_to_hz(note.n_midi).clamp(145.0, 262.0);
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.55);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(body_hz, phase, t, velocity);
        let env = articulation_envelope(phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 0.4;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.035))
}

fn articulation_output_shape() -> (f32, f32) {
    (1.22, 1.0)
}

fn articulation_sample(body_hz: f32, phase: f32, t: f32, velocity: f32) -> f32 {
    let pitch_env = 1.0 - smoothstep((phase * 7.0).clamp(0.0, 1.0));
    let body_fund = sine(body_hz * (1.0 + pitch_env * 0.18), t) * 0.58;
    let body_ring = sine(body_hz * 2.05, t) * (-phase * 7.8).exp() * 0.3;
    let shell = triangle(body_hz * 3.15, t) * (-phase * 8.8).exp() * 0.14;

    let crack_tone =
        sine(1_860.0 + pitch_env * 1_050.0, t) * (1.0 - smoothstep((phase * 24.0).clamp(0.0, 1.0)));
    let stick_noise = hash_noise(t * 22_800.0) * (1.0 - smoothstep((phase * 28.0).clamp(0.0, 1.0)));

    let wire_white = hash_noise(t * 15_600.0);
    let wire_dark = hash_noise(t * 6_300.0);
    let wire = (wire_white - wire_dark * 0.58)
        * (0.22 + 0.78 * (1.0 - smoothstep((phase * 3.3).clamp(0.0, 1.0))));

    let base = body_fund + body_ring + shell;
    let attack = (stick_noise * 0.92 + crack_tone * 0.72) * (0.6 + velocity * 0.55);

    let rim = triangle(2_450.0 + velocity * 280.0, t) * 0.24;
    (base * 1.05 + attack * 0.68 + wire * 0.86 + rim).tanh()
}

fn articulation_envelope(phase: f32, velocity: f32) -> f32 {
    let attack = 0.0012;
    let (body_decay, wire_decay) = (1.15 - velocity * 0.12, 0.78);
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let body = (-phase * body_decay * 5.2).exp();
    let wire = (-phase * wire_decay * 9.8).exp();
    (attack_env * (body * 0.62 + wire * 0.38)).clamp(0.0, 1.0)
}
