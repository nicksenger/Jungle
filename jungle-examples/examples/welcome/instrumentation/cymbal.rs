use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct Cymbal {
    audio: AudioHandle,
}

impl Cymbal {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum CymbalArticulation {
    /// A standard, explosive crash on the edge of the cymbal.
    StandardCrash,
    /// Grabbing the cymbal with the hand immediately after striking to choke the sound.
    ChokedCrash,
    /// Striking the flat surface of the ride cymbal with the tip of the stick for a clear ping.
    RideTip,
    /// Striking the dome/bell of the ride cymbal.
    /// Adds distinct, bright, metallic punctuation to specific grooves.
    RideBell,
}

impl Instrument for Cymbal {
    type Articulation = CymbalArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_cymbal(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_cymbal(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.25;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<CymbalArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_cymbal(note: &Note<CymbalArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let pitch_bias = ((note.n_midi as f32 - 49.0) / 16.0).clamp(-0.35, 0.35);

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

fn articulation_duration(base: Duration, articulation: CymbalArticulation) -> Duration {
    let scale = match articulation {
        CymbalArticulation::StandardCrash => 1.85,
        CymbalArticulation::ChokedCrash => 0.24,
        CymbalArticulation::RideTip => 0.65,
        CymbalArticulation::RideBell => 0.8,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: CymbalArticulation) -> (f32, f32) {
    match articulation {
        CymbalArticulation::StandardCrash => (0.9, 1.0),
        CymbalArticulation::ChokedCrash => (0.8, 1.0),
        CymbalArticulation::RideTip => (0.72, 1.0),
        CymbalArticulation::RideBell => (0.78, 1.0),
    }
}

fn articulation_sample(
    articulation: CymbalArticulation,
    phase: f32,
    t: f32,
    velocity: f32,
    pitch_bias: f32,
) -> f32 {
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
    let ping = triangle(1_550.0 * tilt, t) * 0.22;

    match articulation {
        CymbalArticulation::StandardCrash => {
            let low_metal = triangle(1_250.0 * tilt, t) * 0.08 + triangle(1_900.0 * tilt, t) * 0.06;
            (wash + metallic * bite + broadband + low_metal + stick).tanh()
        }
        CymbalArticulation::ChokedCrash => {
            let choke = 1.0 - smoothstep((phase / 0.28).clamp(0.0, 1.0));
            (broadband * 0.75 + metallic * 0.95 + stick * 0.8) * choke
        }
        CymbalArticulation::RideTip => ping + metallic * 0.52 + broadband * 0.24 + stick * 0.35,
        CymbalArticulation::RideBell => {
            let bell = triangle(2_150.0 * tilt, t) * 0.32 + triangle(3_250.0 * tilt, t) * 0.22;
            (bell + metallic * 0.55 + broadband * 0.18 + stick * 0.3).tanh()
        }
    }
}

fn articulation_envelope(articulation: CymbalArticulation, phase: f32, velocity: f32) -> f32 {
    let attack = match articulation {
        CymbalArticulation::StandardCrash => 0.0018,
        _ => 0.0035,
    };
    let decay = match articulation {
        CymbalArticulation::StandardCrash => 0.18 - velocity * 0.05,
        CymbalArticulation::ChokedCrash => 1.5,
        CymbalArticulation::RideTip => 0.72,
        CymbalArticulation::RideBell => 0.62,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.6).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
