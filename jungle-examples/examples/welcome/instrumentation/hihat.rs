use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct HiHat {
    audio: AudioHandle,
}

impl HiHat {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum HiHatArticulation {
    /// Fully pressed closed, yielding a tight, crisp "chick" sound.
    ClosedTip,
    /// Striking the edge of a closed hi-hat with the shoulder of the stick for more bite.
    ClosedEdge,
    /// Slightly releasing foot pressure so the cymbals sizzle against each other.
    /// Essential for building tension in the pre-chorus.
    HalfOpen,
    /// Completely open, creating a loud, aggressive, sloshy wash. Used in the choruses.
    FullOpen,
    /// Closing the hats purely with the foot pedal, creating a soft "chick" with no stick attack.
    FootSplash,
}

impl Instrument for HiHat {
    type Articulation = HiHatArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_hihat(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_hihat(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.2;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<HiHatArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_hihat(note: &Note<HiHatArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let velocity = note.velocity.clamp(0.0, 1.0);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, phase, t);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: HiHatArticulation) -> Duration {
    let scale = match articulation {
        HiHatArticulation::ClosedTip => 0.14,
        HiHatArticulation::ClosedEdge => 0.2,
        HiHatArticulation::HalfOpen => 0.45,
        HiHatArticulation::FullOpen => 0.9,
        HiHatArticulation::FootSplash => 0.24,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.02))
}

fn articulation_output_shape(articulation: HiHatArticulation) -> (f32, f32) {
    match articulation {
        HiHatArticulation::ClosedTip => (0.62, 1.0),
        HiHatArticulation::ClosedEdge => (0.68, 1.0),
        HiHatArticulation::HalfOpen => (0.76, 1.0),
        HiHatArticulation::FullOpen => (0.84, 1.0),
        HiHatArticulation::FootSplash => (0.58, 1.0),
    }
}

fn articulation_sample(articulation: HiHatArticulation, phase: f32, t: f32) -> f32 {
    let bright = hash_noise(t * 22_000.0);
    let metallic = triangle(4_500.0, t) * 0.15 + triangle(6_800.0, t) * 0.1;

    match articulation {
        HiHatArticulation::ClosedTip => (bright * 0.7 + metallic * 0.3) * (1.0 - phase * 0.75),
        HiHatArticulation::ClosedEdge => {
            (bright * 0.78 + metallic * 0.38).tanh() * (1.0 - phase * 0.6)
        }
        HiHatArticulation::HalfOpen => {
            let sizzle = hash_noise(t * 13_000.0) * (0.4 + 0.4 * (phase * 4.0).sin().abs());
            bright * 0.55 + metallic * 0.24 + sizzle * 0.38
        }
        HiHatArticulation::FullOpen => {
            let wash = hash_noise(t * 10_500.0) * (0.55 + 0.25 * smoothstep(phase * 1.3));
            (bright * 0.42 + metallic * 0.2 + wash).tanh()
        }
        HiHatArticulation::FootSplash => {
            let chick = hash_noise(t * 18_000.0) * (1.0 - smoothstep(phase * 6.0));
            let pedal = triangle(1_250.0, t) * 0.12;
            chick * 0.6 + pedal
        }
    }
}

fn articulation_envelope(articulation: HiHatArticulation, phase: f32) -> f32 {
    let attack = 0.004;
    let decay = match articulation {
        HiHatArticulation::ClosedTip => 1.8,
        HiHatArticulation::ClosedEdge => 1.5,
        HiHatArticulation::HalfOpen => 0.8,
        HiHatArticulation::FullOpen => 0.42,
        HiHatArticulation::FootSplash => 1.2,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 5.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
