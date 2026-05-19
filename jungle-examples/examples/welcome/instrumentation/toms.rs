use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
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
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_toms(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_toms(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = -0.05;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<TomsArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_toms(note: &Note<TomsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(70.0, 220.0);
    let velocity = note.velocity.clamp(0.0, 1.0);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, base_hz, phase, t);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: TomsArticulation) -> Duration {
    let scale = match articulation {
        TomsArticulation::StandardHit => 0.45,
        TomsArticulation::AccentedHit => 0.52,
        TomsArticulation::DoubleHit => 0.55,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: TomsArticulation) -> (f32, f32) {
    match articulation {
        TomsArticulation::StandardHit => (0.92, 1.0),
        TomsArticulation::AccentedHit => (1.02, 1.0),
        TomsArticulation::DoubleHit => (1.05, 1.0),
    }
}

fn articulation_sample(articulation: TomsArticulation, base_hz: f32, phase: f32, t: f32) -> f32 {
    let main = sine(base_hz, t) * 0.55 + sine(base_hz * 1.95, t) * 0.18;
    let shell = triangle(base_hz * 2.6, t) * 0.12;
    let stick = hash_noise(t * 8_400.0) * (1.0 - smoothstep(phase * 8.0)) * 0.22;

    match articulation {
        TomsArticulation::StandardHit => main + shell + stick,
        TomsArticulation::AccentedHit => (main * 1.15 + shell * 1.1 + stick * 1.35).tanh(),
        TomsArticulation::DoubleHit => {
            let lower = sine(base_hz * 0.68, t) * 0.46 + sine(base_hz * 1.32, t) * 0.2;
            (main + lower + shell + stick * 1.2).tanh()
        }
    }
}

fn articulation_envelope(articulation: TomsArticulation, phase: f32) -> f32 {
    let attack = 0.006;
    let decay = match articulation {
        TomsArticulation::StandardHit => 0.75,
        TomsArticulation::AccentedHit => 0.66,
        TomsArticulation::DoubleHit => 0.62,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.8).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
