use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, SAMPLE_RATE,
    },
    Error, Expression, Instrument, Note,
};

pub struct Vocals {
    audio: AudioHandle,
}

impl Vocals {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum VocalsArticulation {
    /// Clean, melodic singing with standard resonance (e.g., the lower register parts of the verses).
    Clean,
    /// Pushing the voice into a distorted, high-register rock belt.
    /// This is Axl's signature sound for the choruses.
    GritRasp,
    /// The chest-voice, semi-spoken, low-register delivery.
    /// Essential for the "Do you know where you are?" breakdown.
    SpokenBreakdown,
    /// Ultra-high, piercing falsetto screams (like the legendary "Welcome to the Jungle!" intro howl).
    SirenScream,
    /// Rapid, rhythmic, percussive vocal sound effects (e.g., the stuttering "nn-nn-nn-nn-nn-nn-nn-f-f-freee").
    StutterStab,
}

impl Instrument for Vocals {
    type Articulation = VocalsArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_vocals(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

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

fn articulation_layers(articulation: VocalsArticulation) -> &'static [PlaybackLayer] {
    match articulation {
        VocalsArticulation::Clean => &[
            PlaybackLayer {
                pan: 0.02,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.17,
                gain_scale: 0.42,
                playback_rate_scale: 0.996,
                delay_seconds: 0.013,
            },
            PlaybackLayer {
                pan: 0.21,
                gain_scale: 0.33,
                playback_rate_scale: 1.007,
                delay_seconds: 0.021,
            },
        ],
        VocalsArticulation::GritRasp => &[
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.12,
                gain_scale: 0.48,
                playback_rate_scale: 0.992,
                delay_seconds: 0.009,
            },
            PlaybackLayer {
                pan: 0.16,
                gain_scale: 0.38,
                playback_rate_scale: 1.01,
                delay_seconds: 0.015,
            },
        ],
        VocalsArticulation::SpokenBreakdown => &[
            PlaybackLayer {
                pan: -0.02,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.14,
                gain_scale: 0.26,
                playback_rate_scale: 1.008,
                delay_seconds: 0.018,
            },
        ],
        VocalsArticulation::SirenScream => &[
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.09,
                gain_scale: 0.31,
                playback_rate_scale: 0.994,
                delay_seconds: 0.012,
            },
            PlaybackLayer {
                pan: 0.11,
                gain_scale: 0.29,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
        VocalsArticulation::StutterStab => &[
            PlaybackLayer {
                pan: 0.03,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.14,
                gain_scale: 0.24,
                playback_rate_scale: 0.99,
                delay_seconds: 0.008,
            },
        ],
    }
}

fn synthesize_vocals(note: &Note<VocalsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(85.0, 1_300.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, base_hz, phase, t, expression);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: VocalsArticulation) -> Duration {
    let scale = match articulation {
        VocalsArticulation::Clean => 1.05,
        VocalsArticulation::GritRasp => 1.0,
        VocalsArticulation::SpokenBreakdown => 0.8,
        VocalsArticulation::SirenScream => 1.1,
        VocalsArticulation::StutterStab => 0.28,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: VocalsArticulation) -> (f32, f32) {
    match articulation {
        VocalsArticulation::Clean => (0.9, 1.0),
        VocalsArticulation::GritRasp => (0.95, 1.0),
        VocalsArticulation::SpokenBreakdown => (0.84, 1.0),
        VocalsArticulation::SirenScream => (1.0, 1.03),
        VocalsArticulation::StutterStab => (0.86, 1.0),
    }
}

fn articulation_sample(
    articulation: VocalsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let vibrato = (TAU * 5.7 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.02;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.15;

    match articulation {
        VocalsArticulation::Clean => {
            let f = base_hz * (1.0 + bend + vibrato);
            vocal_formant(f, t, 0.16)
        }
        VocalsArticulation::GritRasp => {
            let f = base_hz * (1.0 + bend * 0.5 + vibrato * 0.45);
            let body = vocal_formant(f, t, 0.22);
            let grit = hash_noise(t * 7_000.0) * (0.25 + 0.2 * (TAU * 18.0 * t).sin().abs());
            (body + grit).tanh()
        }
        VocalsArticulation::SpokenBreakdown => {
            let f = base_hz * 0.8 * (1.0 + bend * 0.2);
            sine(f, t) * 0.55 + saw(f * 2.0, t) * 0.12 + hash_noise(t * 2_000.0) * 0.06
        }
        VocalsArticulation::SirenScream => {
            let rise = smoothstep(phase) * 0.35;
            let f = base_hz * (1.45 + rise) * (1.0 + vibrato * 1.6);
            let body = sine(f, t) * 0.62 + sine(f * 2.0, t) * 0.23;
            let air = hash_noise(t * 9_500.0) * 0.08;
            (body + air).tanh()
        }
        VocalsArticulation::StutterStab => {
            let f = base_hz * (1.0 + bend * 0.2);
            let body = vocal_formant(f, t, 0.18);
            let gate = ((phase * 12.0).fract() < 0.42) as i32 as f32;
            body * gate
        }
    }
}

fn vocal_formant(frequency_hz: f32, t: f32, airy: f32) -> f32 {
    let base = sine(frequency_hz, t) * 0.55;
    let second = sine(frequency_hz * 2.1, t) * 0.22;
    let third = sine(frequency_hz * 3.2, t) * 0.15;
    let breath = hash_noise(t * 5_000.0) * airy;
    base + second + third + breath
}

fn articulation_envelope(articulation: VocalsArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        VocalsArticulation::StutterStab => 0.01,
        VocalsArticulation::SirenScream => 0.06,
        _ => 0.03,
    };
    let body = match articulation {
        VocalsArticulation::Clean => 0.9,
        VocalsArticulation::GritRasp => 0.94,
        VocalsArticulation::SpokenBreakdown => 0.82,
        VocalsArticulation::SirenScream => 0.97,
        VocalsArticulation::StutterStab => 0.74,
    };
    let release_start = match articulation {
        VocalsArticulation::SpokenBreakdown => 0.66,
        VocalsArticulation::StutterStab => 0.48,
        VocalsArticulation::SirenScream => 0.84,
        _ => 0.8,
    };
    let release: f32 = match articulation {
        VocalsArticulation::Clean => 0.18,
        VocalsArticulation::GritRasp => 0.15,
        VocalsArticulation::SpokenBreakdown => 0.22,
        VocalsArticulation::SirenScream => 0.2,
        VocalsArticulation::StutterStab => 0.1,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let release_phase = ((phase - release_start) / release.max(1e-3_f32)).clamp(0.0, 1.0);
    let release_env = 1.0 - smoothstep(release_phase);
    (attack_env * body * release_env).clamp(0.0, 1.0)
}
