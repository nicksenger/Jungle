pub mod cymbal {
    use std::{sync::Arc, time::Duration};

    use super::super::{duration_to_frames, hash_noise, smoothstep, triangle, Note, SAMPLE_RATE};

    pub fn synthesize_cymbal(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
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
            + triangle(9_100.0 * tilt, t) * 0.08
            + triangle(12_000.0 * tilt, t) * 0.06
            + triangle(15_000.0 * tilt, t) * 0.04;
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
}

pub mod hihat {
    use std::{sync::Arc, time::Duration};

    use super::super::{duration_to_frames, hash_noise, smoothstep, triangle, Note, SAMPLE_RATE};

    pub fn synthesize_hihat(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
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
            + triangle(11_400.0 * tilt, t) * 0.12
            + triangle(14_000.0 * tilt, t) * 0.08
            + triangle(17_000.0 * tilt, t) * 0.05;

        let bark = 1.0 - smoothstep((phase / 0.58).clamp(0.0, 1.0));
        (bright * 0.6 + metallic * 0.65 + hiss * 0.3 + stick * 0.75) * bark
    }

    fn articulation_envelope(phase: f32, velocity: f32) -> f32 {
        let attack = 0.002;
        let decay = 1.05 - velocity * 0.14;
        let fast_choke = 1.0 - smoothstep((phase / 0.95).clamp(0.0, 1.0));
        let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
        let decay_env = (-phase * decay * 5.0).exp();
        (attack_env * decay_env * fast_choke).clamp(0.0, 1.0)
    }
}

pub mod kick_drum {
    use std::{sync::Arc, time::Duration};

    use super::super::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, Note, SAMPLE_RATE,
    };

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
        let beater_tone = sine(1_900.0 + pitch_env * 640.0, t)
            * (1.0 - smoothstep((phase * 26.0).clamp(0.0, 1.0)));
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
}

pub mod snare_drum {
    use std::{sync::Arc, time::Duration};

    use super::super::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, Note, SAMPLE_RATE,
    };

    pub fn synthesize_snare_drum(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
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

        let crack_tone = sine(1_860.0 + pitch_env * 1_050.0, t)
            * (1.0 - smoothstep((phase * 24.0).clamp(0.0, 1.0)));
        let stick_noise =
            hash_noise(t * 22_800.0) * (1.0 - smoothstep((phase * 28.0).clamp(0.0, 1.0)));

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
}

pub mod toms {
    use std::{sync::Arc, time::Duration};

    use super::super::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, Note, SAMPLE_RATE,
    };

    pub fn synthesize_toms(note: &Note<()>) -> (Arc<[f32]>, f32, f32) {
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
        let second_mode =
            sine(base_hz * (1.54 + pitch_env * 0.06), t) * (-phase * 6.8).exp() * 0.29;
        let shell = triangle(base_hz * 2.32, t) * (-phase * 7.5).exp() * 0.2;
        let floor_coupling = sine(base_hz * 0.71, t) * (-phase * 5.2).exp() * 0.22;

        let beater_noise = (hash_noise(t * 12_400.0) - hash_noise(t * 3_200.0) * 0.48)
            * (1.0 - smoothstep((phase * 16.5).clamp(0.0, 1.0)));
        let beater_tone = sine(2_240.0 + pitch_env * 560.0, t)
            * (1.0 - smoothstep((phase * 24.0).clamp(0.0, 1.0)));
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
}
