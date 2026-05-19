use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Expression, Instrument, Note,
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
        let (pcm, gain, playback_rate) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_bass(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn synthesize_bass(note: &Note<BassArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(35.0, 220.0);
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
        BassArticulation::Picked => (0.93, 1.0),
        BassArticulation::AccentedClank => (0.98, 1.0),
        BassArticulation::StaccatoMute => (0.9, 1.0),
        BassArticulation::SlideDown => (0.94, 1.0),
        BassArticulation::GhostNote => (0.82, 1.0),
    }
}

fn articulation_sample(
    articulation: BassArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.14;
    let vibrato = (TAU * 5.2 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let freq = base_hz * (1.0 + bend + vibrato);

    match articulation {
        BassArticulation::Picked => {
            let transient = hash_noise(t * 15_500.0) * (1.0 - smoothstep(phase * 13.0));
            let growl = saw(freq, t) * 0.56 + saw(freq * 2.0, t) * 0.22;
            let low_body = sine(freq * 0.5, t) * 0.2 + triangle(freq, t) * 0.18;
            ((growl + low_body + transient * 0.24) * 1.25).tanh()
        }
        BassArticulation::AccentedClank => {
            let transient = hash_noise(t * 18_000.0) * (1.0 - smoothstep(phase * 10.0));
            let clank = saw(freq * 2.4, t) * 0.34 + sine(freq * 3.0, t) * 0.15;
            let body = saw(freq, t) * 0.48 + triangle(freq * 0.5, t) * 0.24;
            ((body + clank + transient * 0.44) * 1.55).tanh()
        }
        BassArticulation::StaccatoMute => {
            let punch = saw(freq, t) * 0.54 + sine(freq * 0.5, t) * 0.24;
            (punch * (1.0 - phase * 0.9)).tanh()
        }
        BassArticulation::SlideDown => {
            let slide = 1.16 - smoothstep(phase) * 0.28;
            let f = freq * slide;
            let body = saw(f, t) * 0.5 + triangle(f * 0.5, t) * 0.28;
            let attack = hash_noise(t * 8_500.0) * (1.0 - smoothstep(phase * 9.0)) * 0.14;
            ((body + attack) * 1.2).tanh()
        }
        BassArticulation::GhostNote => {
            let thud = sine(freq * 0.45, t) * 0.25;
            let muted_noise = hash_noise(t * 6_500.0) * 0.38;
            (thud + muted_noise) * (1.0 - phase).max(0.0)
        }
    }
}

fn articulation_envelope(articulation: BassArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        BassArticulation::GhostNote => 0.005,
        BassArticulation::AccentedClank => 0.006,
        BassArticulation::Picked => 0.008,
        _ => 0.012,
    };
    let decay = match articulation {
        BassArticulation::Picked => 0.48,
        BassArticulation::AccentedClank => 0.72,
        BassArticulation::StaccatoMute => 1.2,
        BassArticulation::SlideDown => 0.56,
        BassArticulation::GhostNote => 1.7,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
