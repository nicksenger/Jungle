use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct BackupVocals {
    audio: AudioHandle,
}

impl BackupVocals {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BackupVocalsArticulation {
    /// Clean, unified group harmony backing up a lead line.
    GroupHarmony,
    /// Aggressive, chanted, or shouted backing lines (e.g., shouting "Jungle!" in response to Axl).
    ShoutResponse,
    /// Sustained, open-vowel vocal beds ("Ahhs" or "Ohhs") used for atmospheric backing texture.
    VocalBed,
}

impl Instrument for BackupVocals {
    type Articulation = BackupVocalsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_backup_vocals(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_backup_vocals(&note)
        };

        // Layer multiple takes with slight timing/rate offsets to emulate a backing vocal gang.
        for layer in articulation_layers(note.articulation) {
            let mut request = PlayRequest::new(Arc::clone(&pcm), 1, SAMPLE_RATE);
            request.start_offset = note.offset + Duration::from_secs_f32(layer.delay_seconds);
            request.gain = gain * layer.gain_scale;
            request.playback_rate = playback_rate * layer.playback_rate_scale;
            request.pan = layer.pan;
            self.audio
                .try_play(request)
                .map_err(|_| Error::Submission)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
struct PlaybackLayer {
    pan: f32,
    gain_scale: f32,
    playback_rate_scale: f32,
    delay_seconds: f32,
}

fn articulation_layers(articulation: BackupVocalsArticulation) -> &'static [PlaybackLayer] {
    match articulation {
        BackupVocalsArticulation::GroupHarmony => &[
            PlaybackLayer {
                pan: -0.42,
                gain_scale: 0.7,
                playback_rate_scale: 0.992,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 0.9,
                playback_rate_scale: 1.0,
                delay_seconds: 0.011,
            },
            PlaybackLayer {
                pan: 0.38,
                gain_scale: 0.68,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
        BackupVocalsArticulation::ShoutResponse => &[
            PlaybackLayer {
                pan: -0.58,
                gain_scale: 0.68,
                playback_rate_scale: 0.985,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.1,
                gain_scale: 0.82,
                playback_rate_scale: 1.0,
                delay_seconds: 0.007,
            },
            PlaybackLayer {
                pan: 0.24,
                gain_scale: 0.78,
                playback_rate_scale: 1.013,
                delay_seconds: 0.014,
            },
            PlaybackLayer {
                pan: 0.62,
                gain_scale: 0.63,
                playback_rate_scale: 1.02,
                delay_seconds: 0.021,
            },
        ],
        BackupVocalsArticulation::VocalBed => &[
            PlaybackLayer {
                pan: -0.65,
                gain_scale: 0.58,
                playback_rate_scale: 0.994,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.24,
                gain_scale: 0.66,
                playback_rate_scale: 0.999,
                delay_seconds: 0.026,
            },
            PlaybackLayer {
                pan: 0.22,
                gain_scale: 0.66,
                playback_rate_scale: 1.005,
                delay_seconds: 0.033,
            },
            PlaybackLayer {
                pan: 0.64,
                gain_scale: 0.57,
                playback_rate_scale: 1.012,
                delay_seconds: 0.041,
            },
        ],
    }
}

fn should_spawn_blocking(note: &Note<BackupVocalsArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_backup_vocals(note: &Note<BackupVocalsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(90.0, 880.0);
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

fn articulation_duration(base: Duration, articulation: BackupVocalsArticulation) -> Duration {
    let scale = match articulation {
        BackupVocalsArticulation::GroupHarmony => 1.0,
        BackupVocalsArticulation::ShoutResponse => 0.72,
        BackupVocalsArticulation::VocalBed => 1.35,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: BackupVocalsArticulation) -> (f32, f32) {
    match articulation {
        BackupVocalsArticulation::GroupHarmony => (0.78, 1.0),
        BackupVocalsArticulation::ShoutResponse => (0.85, 1.0),
        BackupVocalsArticulation::VocalBed => (0.72, 1.0),
    }
}

fn articulation_sample(
    articulation: BackupVocalsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
) -> f32 {
    let unison = sine(base_hz * 0.995, t) * 0.42
        + sine(base_hz * 1.0, t) * 0.3
        + sine(base_hz * 1.007, t) * 0.28;

    match articulation {
        BackupVocalsArticulation::GroupHarmony => {
            unison + sine(base_hz * 2.0, t) * 0.15 + hash_noise(t * 4_500.0) * 0.08
        }
        BackupVocalsArticulation::ShoutResponse => {
            let punch = 1.0 - smoothstep(phase * 3.4);
            (unison * 1.1 + hash_noise(t * 8_500.0) * 0.22 * punch).tanh()
        }
        BackupVocalsArticulation::VocalBed => {
            let wide = sine(base_hz * 0.5, t) * 0.2 + sine(base_hz * 1.5, t) * 0.24;
            unison * 0.72 + wide + hash_noise(t * 2_500.0) * 0.06
        }
    }
}

fn articulation_envelope(articulation: BackupVocalsArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        BackupVocalsArticulation::ShoutResponse => 0.015,
        BackupVocalsArticulation::VocalBed => 0.08,
        BackupVocalsArticulation::GroupHarmony => 0.04,
    };
    let decay = match articulation {
        BackupVocalsArticulation::GroupHarmony => 0.45,
        BackupVocalsArticulation::ShoutResponse => 0.8,
        BackupVocalsArticulation::VocalBed => 0.32,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
