use std::{sync::Arc, time::Duration};

use crate::audio::PlayRequest;
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{duration_to_frames, hash_noise, smoothstep, triangle, SAMPLE_RATE},
    Error, Note,
};

use super::CymbalArticulation;

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    note: Note<CymbalArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = {
        let note_for_synth = note;
        tokio::task::spawn_blocking(move || synthesize_cymbal(&note_for_synth))
            .await
            .map_err(|_| Error::Playback)?
    };

    let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.25;
    audio.play(request).await.map_err(|_| Error::Submission)
}

fn synthesize_cymbal(note: &Note<CymbalArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let pitch_bias = ((note.n_midi as f32 - 49.0) / 16.0).clamp(-0.35, 0.35);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(phase, t, velocity, pitch_bias);
        let env = articulation_envelope(phase, velocity) * release_taper(phase);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 1.85;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape() -> (f32, f32) {
    (0.9, 1.0)
}

fn articulation_sample(phase: f32, t: f32, velocity: f32, pitch_bias: f32) -> f32 {
    let tilt = 1.0 + pitch_bias * 0.08;
    let attack_focus = 1.0 - smoothstep((phase / 0.2).clamp(0.0, 1.0));
    let bite = 0.65 + 0.35 * velocity;
    let broadband = hash_noise(t * 19_500.0) * (0.24 + 0.14 * bite);
    let wash = hash_noise(t * 10_800.0) * (0.46 + 0.24 * smoothstep(phase * 1.1));
    let stick = hash_noise(t * 31_000.0) * (0.24 + 0.3 * velocity) * attack_focus;
    let metallic = triangle(3_800.0 * tilt, t) * 0.2
        + triangle(5_250.0 * tilt, t) * 0.16
        + triangle(6_850.0 * tilt, t) * 0.11
        + triangle(9_100.0 * tilt, t) * 0.08;
    let low_metal = triangle(1_250.0 * tilt, t) * 0.08 + triangle(1_900.0 * tilt, t) * 0.06;
    (wash + metallic * bite + broadband + low_metal + stick).tanh()
}

fn articulation_envelope(phase: f32, velocity: f32) -> f32 {
    let attack = 0.0018;
    let decay = 0.86 - velocity * 0.12;
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.6).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn release_taper(phase: f32) -> f32 {
    let release_start = 0.88;
    let tail = ((phase - release_start) / (1.0 - release_start)).clamp(0.0, 1.0);
    1.0 - smoothstep(tail)
}
