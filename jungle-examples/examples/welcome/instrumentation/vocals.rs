use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
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
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_vocals(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_vocals(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.05;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<VocalsArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
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
    let decay = match articulation {
        VocalsArticulation::Clean => 0.42,
        VocalsArticulation::GritRasp => 0.5,
        VocalsArticulation::SpokenBreakdown => 0.7,
        VocalsArticulation::SirenScream => 0.38,
        VocalsArticulation::StutterStab => 1.45,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
