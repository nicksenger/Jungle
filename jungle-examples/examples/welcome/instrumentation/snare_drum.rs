use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct SnareDrum {
    audio: AudioHandle,
}

impl SnareDrum {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SnareDrumArticulation {
    /// A standard, clean strike to the center of the drum head.
    CenterHit,
    /// Striking the center of the head and the metal rim simultaneously.
    /// This is the primary articulation for the massive verse/chorus backbeats.
    Rimshot,
    /// Laying the stick across the head and striking the rim for a woody click.
    /// Useful for low-energy dynamic drops.
    Sidestick,
    /// A very soft, low-velocity hit. Adler uses these to fill the space between backbeats.
    GhostNote,
    /// Two rapid, almost overlapping strikes (one hand trailing the other) to add weight.
    Flam,
}

impl Instrument for SnareDrum {
    type Articulation = SnareDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_snare_drum(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_snare_drum(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.1;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<SnareDrumArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_snare_drum(note: &Note<SnareDrumArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let body_hz = midi_to_hz(note.n_midi).clamp(110.0, 290.0);
    let velocity = note.velocity.clamp(0.0, 1.0);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, body_hz, phase, t);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: SnareDrumArticulation) -> Duration {
    let scale = match articulation {
        SnareDrumArticulation::CenterHit => 0.3,
        SnareDrumArticulation::Rimshot => 0.32,
        SnareDrumArticulation::Sidestick => 0.24,
        SnareDrumArticulation::GhostNote => 0.22,
        SnareDrumArticulation::Flam => 0.34,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: SnareDrumArticulation) -> (f32, f32) {
    match articulation {
        SnareDrumArticulation::CenterHit => (0.9, 1.0),
        SnareDrumArticulation::Rimshot => (1.0, 1.0),
        SnareDrumArticulation::Sidestick => (0.72, 1.0),
        SnareDrumArticulation::GhostNote => (0.55, 1.0),
        SnareDrumArticulation::Flam => (0.95, 1.0),
    }
}

fn articulation_sample(
    articulation: SnareDrumArticulation,
    body_hz: f32,
    phase: f32,
    t: f32,
) -> f32 {
    let body = sine(body_hz, t) * 0.4 + sine(body_hz * 1.85, t) * 0.18;
    let snap = hash_noise(t * 9_800.0) * (1.0 - smoothstep(phase * 8.0));
    let sizzle = hash_noise(t * 6_400.0) * 0.28;

    match articulation {
        SnareDrumArticulation::CenterHit => body + snap * 0.48 + sizzle * 0.28,
        SnareDrumArticulation::Rimshot => {
            let rim = triangle(2_300.0, t) * 0.28;
            body + snap * 0.62 + rim + sizzle * 0.25
        }
        SnareDrumArticulation::Sidestick => {
            triangle(1_500.0, t) * 0.32 + triangle(2_100.0, t) * 0.18 + snap * 0.22
        }
        SnareDrumArticulation::GhostNote => (body * 0.5 + snap * 0.32 + sizzle * 0.2) * 0.68,
        SnareDrumArticulation::Flam => {
            let secondary = hash_noise((t - 0.008).max(0.0) * 9_200.0)
                * (1.0 - smoothstep(((phase - 0.05) * 8.0).max(0.0)));
            body + snap * 0.54 + secondary * 0.35 + sizzle * 0.24
        }
    }
}

fn articulation_envelope(articulation: SnareDrumArticulation, phase: f32) -> f32 {
    let attack = 0.004;
    let decay = match articulation {
        SnareDrumArticulation::CenterHit => 1.05,
        SnareDrumArticulation::Rimshot => 1.15,
        SnareDrumArticulation::Sidestick => 1.5,
        SnareDrumArticulation::GhostNote => 1.45,
        SnareDrumArticulation::Flam => 1.0,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 5.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
