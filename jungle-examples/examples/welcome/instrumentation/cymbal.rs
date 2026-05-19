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

fn articulation_duration(base: Duration, articulation: CymbalArticulation) -> Duration {
    let scale = match articulation {
        CymbalArticulation::StandardCrash => 1.4,
        CymbalArticulation::ChokedCrash => 0.24,
        CymbalArticulation::RideTip => 0.65,
        CymbalArticulation::RideBell => 0.8,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: CymbalArticulation) -> (f32, f32) {
    match articulation {
        CymbalArticulation::StandardCrash => (0.86, 1.0),
        CymbalArticulation::ChokedCrash => (0.8, 1.0),
        CymbalArticulation::RideTip => (0.72, 1.0),
        CymbalArticulation::RideBell => (0.78, 1.0),
    }
}

fn articulation_sample(articulation: CymbalArticulation, phase: f32, t: f32) -> f32 {
    let broadband = hash_noise(t * 20_000.0) * 0.62;
    let shimmer = triangle(3_900.0, t) * 0.12 + triangle(5_700.0, t) * 0.1;
    let ping = triangle(1_600.0, t) * 0.22;

    match articulation {
        CymbalArticulation::StandardCrash => {
            let wash = hash_noise(t * 9_500.0) * (0.42 + 0.28 * smoothstep(phase));
            (broadband + shimmer + wash).tanh()
        }
        CymbalArticulation::ChokedCrash => {
            (broadband * 0.9 + shimmer * 0.85) * (1.0 - smoothstep(phase * 3.8))
        }
        CymbalArticulation::RideTip => ping + shimmer * 0.65 + broadband * 0.28,
        CymbalArticulation::RideBell => {
            let bell = triangle(2_200.0, t) * 0.32 + triangle(3_300.0, t) * 0.2;
            (bell + shimmer * 0.42 + broadband * 0.2).tanh()
        }
    }
}

fn articulation_envelope(articulation: CymbalArticulation, phase: f32) -> f32 {
    let attack = 0.004;
    let decay = match articulation {
        CymbalArticulation::StandardCrash => 0.24,
        CymbalArticulation::ChokedCrash => 1.5,
        CymbalArticulation::RideTip => 0.72,
        CymbalArticulation::RideBell => 0.62,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.8).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
