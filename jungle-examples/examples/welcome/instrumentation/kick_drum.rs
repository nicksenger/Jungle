use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct KickDrum {
    audio: AudioHandle,
}

impl KickDrum {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum KickDrumArticulation {
    /// A standard, powerful kick where the beater strikes and bounces off.
    StandardHit,
    /// Burying the beater into the head, dampening the sustain for a tighter, punchier thud.
    BuriedBeater,
    /// A soft, unaccented hit used in quick double-stroke patterns.
    GhostHit,
}

impl Instrument for KickDrum {
    type Articulation = KickDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_kick_drum(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_kick_drum(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<KickDrumArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_kick_drum(note: &Note<KickDrumArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(35.0, 85.0);
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

fn articulation_duration(base: Duration, articulation: KickDrumArticulation) -> Duration {
    let scale = match articulation {
        KickDrumArticulation::StandardHit => 0.35,
        KickDrumArticulation::BuriedBeater => 0.24,
        KickDrumArticulation::GhostHit => 0.18,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: KickDrumArticulation) -> (f32, f32) {
    match articulation {
        KickDrumArticulation::StandardHit => (1.1, 1.0),
        KickDrumArticulation::BuriedBeater => (1.0, 1.0),
        KickDrumArticulation::GhostHit => (0.72, 1.0),
    }
}

fn articulation_sample(
    articulation: KickDrumArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
) -> f32 {
    let drop = 1.85 - smoothstep(phase) * 0.95;
    let f = base_hz * drop;
    let boom = sine(f, t) * 0.85;
    let click = hash_noise(t * 11_000.0) * (1.0 - smoothstep(phase * 10.0));

    match articulation {
        KickDrumArticulation::StandardHit => boom + click * 0.14,
        KickDrumArticulation::BuriedBeater => (boom * 0.78 + click * 0.08) * (1.0 - phase * 0.35),
        KickDrumArticulation::GhostHit => (boom * 0.52 + click * 0.05) * (1.0 - phase * 0.7),
    }
}

fn articulation_envelope(articulation: KickDrumArticulation, phase: f32) -> f32 {
    let attack = 0.004;
    let decay = match articulation {
        KickDrumArticulation::StandardHit => 0.95,
        KickDrumArticulation::BuriedBeater => 1.35,
        KickDrumArticulation::GhostHit => 1.6,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 5.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
