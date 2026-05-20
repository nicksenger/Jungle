use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    amplitude_gain,
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Instrument, Note,
};

pub struct Toms {
    audio: AudioHandle,
}

impl Toms {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TomsArticulation {
    /// A clean, resonant strike to the center of the tom.
    StandardHit,
    /// An extra-powerful strike maximizing shell resonance.
    AccentedHit,
    /// Striking two different toms simultaneously (e.g., Rack Tom 2 and Floor Tom).
    /// Used for the massive downbeat punctuation marks in the breakdown.
    DoubleHit,
}

impl Instrument for Toms {
    type Articulation = TomsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, mut gain, mut playback_rate) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_toms(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let velocity = note.velocity.clamp(0.0, 1.0);
        gain *= 0.86 + velocity * 0.42;
        playback_rate *= 0.985 + velocity * 0.045;

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = -0.14 + (velocity - 0.5) * 0.08;
        self.audio.play(request).await.map_err(|_| Error::Submission)
    }
}

fn resolve_articulation(note: &Note<TomsArticulation>) -> TomsArticulation {
    if !matches!(note.articulation, TomsArticulation::StandardHit) {
        return note.articulation;
    }

    let velocity = note.velocity.clamp(0.0, 1.0);
    let duration_ms = note.duration.as_secs_f32() * 1_000.0;

    if velocity >= 0.84 && duration_ms <= 120.0 {
        return TomsArticulation::DoubleHit;
    }
    if velocity >= 0.64 {
        return TomsArticulation::AccentedHit;
    }
    TomsArticulation::StandardHit
}

fn synthesize_toms(note: &Note<TomsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let articulation = resolve_articulation(note);
    let duration = articulation_duration(note.duration, articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(70.0, 220.0);
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.72);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(articulation, base_hz, phase, t, velocity);
        let env = articulation_envelope(articulation, phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: TomsArticulation) -> Duration {
    let scale = match articulation {
        TomsArticulation::StandardHit => 0.42,
        TomsArticulation::AccentedHit => 0.48,
        TomsArticulation::DoubleHit => 0.56,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.04))
}

fn articulation_output_shape(articulation: TomsArticulation) -> (f32, f32) {
    match articulation {
        TomsArticulation::StandardHit => (0.98, 1.0),
        TomsArticulation::AccentedHit => (1.09, 0.998),
        TomsArticulation::DoubleHit => (1.14, 0.994),
    }
}

fn articulation_sample(
    articulation: TomsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    velocity: f32,
) -> f32 {
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

    match articulation {
        TomsArticulation::StandardHit => (base + transient * 0.38).tanh(),
        TomsArticulation::AccentedHit => (base * 1.06 + transient * 0.6).tanh(),
        TomsArticulation::DoubleHit => {
            let lag_t = (t - 0.0105).max(0.0);
            let lag_phase = ((phase - 0.052) * 7.0).max(0.0);
            let lag_pitch_env = 1.0 - smoothstep(lag_phase.clamp(0.0, 1.0));
            let lag_hz = base_hz * 0.76 * (1.0 + lag_pitch_env * 0.28);
            let lag_head = sine(lag_hz, lag_t) * 0.62;
            let lag_mode = sine(lag_hz * 1.48, lag_t) * (-lag_phase * 6.4).exp() * 0.24;
            let lag_shell = triangle(lag_hz * 2.24, lag_t) * (-lag_phase * 6.9).exp() * 0.17;
            let lag_attack = (hash_noise(lag_t * 11_100.0) * 0.52 + sine(1_930.0, lag_t) * 0.32)
                * (1.0 - smoothstep((lag_phase * 3.8).clamp(0.0, 1.0)));
            (base * 0.94 + transient * 0.52 + lag_head + lag_mode + lag_shell + lag_attack).tanh()
        }
    }
}

fn articulation_envelope(articulation: TomsArticulation, phase: f32, velocity: f32) -> f32 {
    let attack = 0.0022;
    let (head_decay, shell_decay) = match articulation {
        TomsArticulation::StandardHit => (1.08 - velocity * 0.18, 0.78),
        TomsArticulation::AccentedHit => (0.96 - velocity * 0.14, 0.68),
        TomsArticulation::DoubleHit => (0.88, 0.58),
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let head = (-phase * head_decay * 4.4).exp();
    let shell = (-phase * shell_decay * 8.2).exp();
    (attack_env * (head * 0.7 + shell * 0.3)).clamp(0.0, 1.0)
}
