use std::{f32::consts::TAU, sync::Arc, time::Duration};

use welcome_audio::{PlayPriority, PlayRequest};
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Expression, Note,
};

use super::BassArticulation;

pub(super) async fn play(
    audio: &welcome_audio::AudioHandle,
    synth: &crate::instrumentation::SynthHandle,
    note: Note<BassArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = synth.bass(note).await?;

    let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.0;
    request.priority = PlayPriority::Normal;
    audio.play(request).await.map_err(|_| Error::Submission)
}

pub(in crate::instrumentation) fn synthesize_bass(
    note: &Note<BassArticulation>,
) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(35.0, 220.0);
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
    let vibrato = (TAU * 5.2 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let freq = base_hz * (1.0 + bend + vibrato);

    let transient = hash_noise(t * 15_500.0) * (1.0 - smoothstep(phase * 13.0));
    let growl = saw(freq, t) * 0.56 + saw(freq * 2.0, t) * 0.22;
    let low_body = sine(freq * 0.5, t) * 0.2 + triangle(freq, t) * 0.18;
    ((growl + low_body + transient * 0.24) * 1.25).tanh()
}

fn articulation_envelope(phase: f32) -> f32 {
    let attack = 0.008;
    let decay = 0.48;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
