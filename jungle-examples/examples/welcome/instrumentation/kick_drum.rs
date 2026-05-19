use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    amplitude_gain, pitch_hz_list,
    synthesis::{duration_to_frames, hash_noise, sine, smoothstep, SAMPLE_RATE},
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
        let (pcm, gain, playback_rate) = {
            let note_for_synth = note.clone();
            tokio::task::spawn_blocking(move || synthesize_kick_drum(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn synthesize_kick_drum(note: &Note<KickDrumArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let pitches_hz = pitch_hz_list(note, 36)
        .into_iter()
        .map(|hz| hz.clamp(48.0, 74.0))
        .collect::<Vec<_>>();
    let base_hz = pitches_hz.iter().sum::<f32>() / pitches_hz.len() as f32;
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.72);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, base_hz, phase, t, velocity);
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
        KickDrumArticulation::StandardHit => (1.28, 1.0),
        KickDrumArticulation::BuriedBeater => (1.14, 1.0),
        KickDrumArticulation::GhostHit => (0.68, 1.0),
    }
}

fn articulation_sample(
    articulation: KickDrumArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    velocity: f32,
) -> f32 {
    let pitch_env = 1.0 - smoothstep((phase * 5.8).clamp(0.0, 1.0));
    let sweep_hz = base_hz * (1.0 + pitch_env * 1.85);
    let sub = sine(sweep_hz, t);
    let punch = sine(base_hz * (1.95 + pitch_env * 0.45), t) * (-phase * 14.0).exp();
    let ring = sine(base_hz * 2.8, t) * (-phase * 10.5).exp();
    let beater_noise = (hash_noise(t * 14_600.0) - hash_noise(t * 2_300.0) * 0.55)
        * (1.0 - smoothstep((phase * 18.0).clamp(0.0, 1.0)));
    let beater_tone =
        sine(1_900.0 + pitch_env * 640.0, t) * (1.0 - smoothstep((phase * 26.0).clamp(0.0, 1.0)));
    let click = (beater_noise * 0.9 + beater_tone * 0.55) * (0.65 + velocity * 0.5);

    match articulation {
        KickDrumArticulation::StandardHit => sub * 0.88 + punch * 0.42 + ring * 0.18 + click * 0.34,
        KickDrumArticulation::BuriedBeater => sub * 0.74 + punch * 0.5 + ring * 0.1 + click * 0.4,
        KickDrumArticulation::GhostHit => {
            (sub * 0.56 + punch * 0.22 + ring * 0.06 + click * 0.14) * (1.0 - phase * 0.5)
        }
    }
}

fn articulation_envelope(articulation: KickDrumArticulation, phase: f32) -> f32 {
    let attack = 0.0012;
    let (body_decay, tail_decay) = match articulation {
        KickDrumArticulation::StandardHit => (1.15, 2.3),
        KickDrumArticulation::BuriedBeater => (1.45, 3.2),
        KickDrumArticulation::GhostHit => (1.9, 3.6),
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let body = (-phase * body_decay * 4.2).exp();
    let tail = (-phase * tail_decay * 9.4).exp();
    (attack_env * (body * 0.74 + tail * 0.26)).clamp(0.0, 1.0)
}
