use std::{sync::Arc, time::Duration};

use crate::audio::PlayRequest;
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{duration_to_frames, hash_noise, smoothstep, triangle, SAMPLE_RATE},
    Error, Note,
};

use super::HiHatArticulation;

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    note: Note<HiHatArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = {
        let note_for_synth = note;
        tokio::task::spawn_blocking(move || synthesize_hihat(&note_for_synth))
            .await
            .map_err(|_| Error::Playback)?
    };

    let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
    request.gain = gain * amplitude_gain(&note);
    request.playback_rate = playback_rate;
    request.pan = 0.2;
    audio.play(request).await.map_err(|_| Error::Submission)
}

fn synthesize_hihat(note: &Note<HiHatArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let pitch_bias = ((note.n_midi as f32 - 46.0) / 10.0).clamp(-0.35, 0.35);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(phase, t, velocity, pitch_bias);
        let env = articulation_envelope(phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape();
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration) -> Duration {
    let scale = 0.42;
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

fn articulation_output_shape() -> (f32, f32) {
    (0.72, 1.0)
}

fn articulation_sample(phase: f32, t: f32, velocity: f32, pitch_bias: f32) -> f32 {
    let tilt = 1.0 + pitch_bias * 0.08;
    let attack_focus = 1.0 - smoothstep((phase / 0.16).clamp(0.0, 1.0));
    let stick = hash_noise(t * 34_000.0) * (0.16 + 0.44 * velocity) * attack_focus;
    let bright = hash_noise(t * 21_500.0) * (0.5 + 0.22 * velocity);
    let hiss = hash_noise(t * 11_400.0) * (0.22 + 0.16 * smoothstep(phase * 1.15));
    let metallic = triangle(6_900.0 * tilt, t) * 0.24
        + triangle(8_700.0 * tilt, t) * 0.18
        + triangle(11_400.0 * tilt, t) * 0.12;

    let bark = 1.0 - smoothstep((phase / 0.58).clamp(0.0, 1.0));
    (bright * 0.54 + metallic * 0.6 + hiss * 0.26 + stick * 0.68) * bark
}

fn articulation_envelope(phase: f32, velocity: f32) -> f32 {
    let attack = 0.002;
    let decay = 1.05 - velocity * 0.14;
    let fast_choke = 1.0 - smoothstep((phase / 0.95).clamp(0.0, 1.0));
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 5.0).exp();
    (attack_env * decay_env * fast_choke).clamp(0.0, 1.0)
}
