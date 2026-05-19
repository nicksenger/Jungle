use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct Bass {
    audio: AudioHandle,
}

impl Bass {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BassArticulation {
    /// A hard, aggressive pick strike with normal sustain.
    Picked,
    /// Forcing the string down so hard it clanks against the frets on attack.
    /// Used to accent the downbeats of the chorus.
    AccentedClank,
    /// Muting the string immediately with the fretting hand.
    /// Essential for keeping the fast-moving basslines crisp and preventing mud.
    StaccatoMute,
    /// Sliding from one note down into the next, a classic Duff transition tool.
    SlideDown,
    /// Striking a completely dead string for a purely percussive thud.
    GhostNote,
}

impl Instrument for Bass {
    type Articulation = BassArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_bass(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_bass(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<BassArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_bass(note: &Note<BassArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(35.0, 220.0);
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

fn articulation_duration(base: Duration, articulation: BassArticulation) -> Duration {
    let scale = match articulation {
        BassArticulation::Picked => 1.12,
        BassArticulation::AccentedClank => 1.0,
        BassArticulation::StaccatoMute => 0.36,
        BassArticulation::SlideDown => 1.0,
        BassArticulation::GhostNote => 0.2,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: BassArticulation) -> (f32, f32) {
    match articulation {
        BassArticulation::Picked => (1.0, 1.0),
        BassArticulation::AccentedClank => (1.08, 1.0),
        BassArticulation::StaccatoMute => (0.95, 1.0),
        BassArticulation::SlideDown => (1.0, 1.0),
        BassArticulation::GhostNote => (0.82, 1.0),
    }
}

fn articulation_sample(articulation: BassArticulation, base_hz: f32, phase: f32, t: f32) -> f32 {
    match articulation {
        BassArticulation::Picked => sine(base_hz, t) * 0.68 + triangle(base_hz * 0.5, t) * 0.22,
        BassArticulation::AccentedClank => {
            let transient = hash_noise(t * 9_000.0) * (1.0 - smoothstep(phase * 7.5));
            let body = sine(base_hz, t) * 0.62 + saw(base_hz * 2.0, t) * 0.22;
            body + transient * 0.35
        }
        BassArticulation::StaccatoMute => {
            (sine(base_hz, t) * 0.7 + saw(base_hz * 1.5, t) * 0.14) * (1.0 - phase * 0.85)
        }
        BassArticulation::SlideDown => {
            let slide = 1.12 - smoothstep(phase) * 0.2;
            let f = base_hz * slide;
            sine(f, t) * 0.65 + triangle(f * 0.5, t) * 0.28
        }
        BassArticulation::GhostNote => {
            let thud = sine(base_hz * 0.45, t) * 0.25;
            let muted_noise = hash_noise(t * 6_500.0) * 0.38;
            (thud + muted_noise) * (1.0 - phase).max(0.0)
        }
    }
}

fn articulation_envelope(articulation: BassArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        BassArticulation::GhostNote => 0.01,
        BassArticulation::AccentedClank => 0.012,
        _ => 0.02,
    };
    let decay = match articulation {
        BassArticulation::Picked => 0.34,
        BassArticulation::AccentedClank => 0.4,
        BassArticulation::StaccatoMute => 1.0,
        BassArticulation::SlideDown => 0.42,
        BassArticulation::GhostNote => 1.4,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
