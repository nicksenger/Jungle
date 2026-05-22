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
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let pitch_bias = ((note.n_midi as f32 - 46.0) / 10.0).clamp(-0.35, 0.35);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, phase, t, velocity, pitch_bias);
        let env = articulation_envelope(note.articulation, phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: HiHatArticulation) -> Duration {
    let scale = match articulation {
        HiHatArticulation::ClosedTip => 0.42,
        HiHatArticulation::ClosedEdge => 0.54,
        HiHatArticulation::HalfOpen => 0.78,
        HiHatArticulation::FullOpen => 1.25,
        HiHatArticulation::FootSplash => 0.32,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

fn articulation_output_shape(articulation: HiHatArticulation) -> (f32, f32) {
    match articulation {
        HiHatArticulation::ClosedTip => (0.72, 1.0),
        HiHatArticulation::ClosedEdge => (0.78, 1.0),
        HiHatArticulation::HalfOpen => (0.82, 1.0),
        HiHatArticulation::FullOpen => (0.88, 1.0),
        HiHatArticulation::FootSplash => (0.64, 1.0),
    }
}

fn articulation_sample(
    articulation: HiHatArticulation,
    phase: f32,
    t: f32,
    velocity: f32,
    pitch_bias: f32,
) -> f32 {
    let tilt = 1.0 + pitch_bias * 0.08;
    let attack_focus = 1.0 - smoothstep((phase / 0.16).clamp(0.0, 1.0));
    let stick = hash_noise(t * 34_000.0) * (0.16 + 0.44 * velocity) * attack_focus;
    let bright = hash_noise(t * 21_500.0) * (0.5 + 0.22 * velocity);
    let hiss = hash_noise(t * 11_400.0) * (0.22 + 0.16 * smoothstep(phase * 1.15));
    let metallic = triangle(6_900.0 * tilt, t) * 0.24
        + triangle(8_700.0 * tilt, t) * 0.18
        + triangle(11_400.0 * tilt, t) * 0.12;

    match articulation {
        HiHatArticulation::ClosedTip => {
            let bark = 1.0 - smoothstep((phase / 0.58).clamp(0.0, 1.0));
            (bright * 0.54 + metallic * 0.6 + hiss * 0.26 + stick * 0.68) * bark
        }
        HiHatArticulation::ClosedEdge => {
            let lower = triangle(4_800.0 * tilt, t) * 0.18 + triangle(5_900.0 * tilt, t) * 0.12;
            let bark = 1.0 - smoothstep((phase / 0.7).clamp(0.0, 1.0));
            (bright * 0.5 + metallic * 0.62 + lower * 0.45 + stick * 0.72) * bark
        }
        HiHatArticulation::HalfOpen => {
            let sizzle = hash_noise(t * 14_500.0) * (0.34 + 0.3 * (phase * 4.8).sin().abs());
            bright * 0.46 + metallic * 0.3 + hiss * 0.34 + sizzle + stick * 0.5
        }
        HiHatArticulation::FullOpen => {
            let wash = hash_noise(t * 9_600.0) * (0.45 + 0.3 * smoothstep(phase * 1.25));
            (bright * 0.38 + metallic * 0.24 + hiss * 0.42 + wash + stick * 0.36).tanh()
        }
        HiHatArticulation::FootSplash => {
            let chick = hash_noise(t * 17_000.0) * (1.0 - smoothstep(phase * 4.2));
            let pedal = triangle(1_250.0, t) * 0.12;
            (chick * 0.72 + hiss * 0.2 + pedal).tanh()
        }
    }
}

fn articulation_envelope(articulation: HiHatArticulation, phase: f32, velocity: f32) -> f32 {
    let attack = 0.002;
    let decay = match articulation {
        HiHatArticulation::ClosedTip => 1.05 - velocity * 0.14,
        HiHatArticulation::ClosedEdge => 0.92 - velocity * 0.12,
        HiHatArticulation::HalfOpen => 0.54 - velocity * 0.06,
        HiHatArticulation::FullOpen => 0.29,
        HiHatArticulation::FootSplash => 0.82,
    };
    let fast_choke = match articulation {
        HiHatArticulation::ClosedTip => 1.0 - smoothstep((phase / 0.95).clamp(0.0, 1.0)),
        HiHatArticulation::ClosedEdge => 1.0 - smoothstep((phase / 1.05).clamp(0.0, 1.0)),
        _ => 1.0,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 5.0).exp();
    (attack_env * decay_env * fast_choke).clamp(0.0, 1.0)
}
