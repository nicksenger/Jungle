use std::{sync::Arc, time::Duration};

use crate::audio::{PlayPriority, PlayRequest};
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Note,
};

use super::TomsArticulation;

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<TomsArticulation>,
) -> Result<(), Error> {
    let (pcm, mut gain, mut playback_rate) = synth.toms(note).await?;

    let velocity = note.velocity.clamp(0.0, 1.0);
    gain *= 0.86 + velocity * 0.42;
    playback_rate *= 0.985 + velocity * 0.045;

    let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = -0.14 + (velocity - 0.5) * 0.08;
    request.priority = PlayPriority::Normal;
    audio.play(request).await.map_err(|_| Error::Submission)
}

pub(in crate::instrumentation) fn synthesize_toms(
    note: &Note<TomsArticulation>,
) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(70.0, 220.0);
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.72);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(base_hz, phase, t, velocity);
        let env = articulation_envelope(phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 0.42;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.04))
}

fn articulation_output_shape() -> (f32, f32) {
    (0.98, 1.0)
}

fn articulation_sample(base_hz: f32, phase: f32, t: f32, velocity: f32) -> f32 {
    let pitch_env = 1.0 - smoothstep((phase * 6.4).clamp(0.0, 1.0));
    let sweep_hz = base_hz * (1.0 + pitch_env * 0.34 + velocity * 0.08);

    let head = sine(sweep_hz, t) * 0.7;
    let second_mode = sine(base_hz * (1.54 + pitch_env * 0.06), t) * (-phase * 6.8).exp() * 0.29;
    let shell = triangle(base_hz * 2.32, t) * (-phase * 7.5).exp() * 0.2;
    let floor_coupling = sine(base_hz * 0.71, t) * (-phase * 5.2).exp() * 0.22;

    let beater_noise = (hash_noise(t * 12_400.0) - hash_noise(t * 3_200.0) * 0.48)
        * (1.0 - smoothstep((phase * 16.5).clamp(0.0, 1.0)));
    let beater_tone =
        sine(2_240.0 + pitch_env * 560.0, t) * (1.0 - smoothstep((phase * 24.0).clamp(0.0, 1.0)));
    let transient = (beater_noise * 0.72 + beater_tone * 0.34) * (0.48 + velocity * 0.66);

    let base = head + second_mode + shell + floor_coupling;

    (base + transient * 0.38).tanh()
}

fn articulation_envelope(phase: f32, velocity: f32) -> f32 {
    let attack = 0.0022;
    let (head_decay, shell_decay) = (1.08 - velocity * 0.18, 0.78);
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let head = (-phase * head_decay * 4.4).exp();
    let shell = (-phase * shell_decay * 8.2).exp();
    (attack_env * (head * 0.7 + shell * 0.3)).clamp(0.0, 1.0)
}
