use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{Error, Expression, Instrument, Note};

pub struct LeadGuitar {
    audio: AudioHandle,
}

impl LeadGuitar {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }

    pub fn audio(&self) -> &AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum LeadGuitarArticulation {
    /// Standard picked note with normal sustain and release.
    Sustained,

    /// Restricting the string vibration with the side of the picking hand.
    /// Essential for the driving rhythm fills under the vocals.
    PalmMuted,

    /// A note sounded entirely by the fretting hand striking the fretboard.
    /// Crucial for the fluid, rapid note runs in the main solo.
    HammerOn,

    /// A note sounded by pulling a fretting finger off a string to release a lower note.
    /// Used in tandem with HammerOns for smooth, unpicked legato phrasing.
    PullOff,

    /// Gently touching the string at specific nodes (like the 5th, 7th, or 12th frets)
    /// to get a bell-like chime. Slash uses these for texture.
    NaturalHarmonic,

    /// "Pinch" harmonics. Pressing the thumb of the picking hand against the string
    /// instantly after picking it, forcing a screaming, high-pitched squeal.
    /// Slash peppers these heavily throughout the verses and fills.
    PinchHarmonic,

    /// Sliding into a note from an indefinite lower or higher pitch.
    /// The signature entry mechanism for almost every phrase in the song.
    Slide,

    /// Striking a string that is completely muted by the fretting hand.
    /// This creates a purely rhythmic, percussive "scratch" or "chug" sound
    /// right before a big chord hits.
    RhythmicRake,
}

impl Instrument for LeadGuitar {
    type Articulation = LeadGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_lead_guitar(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_lead_guitar(&note)
        };
        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.0;

        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

const SAMPLE_RATE: u32 = 48_000;
const SPAWN_BLOCKING_FRAME_THRESHOLD: usize = 8_192;

fn should_spawn_blocking(note: &Note<LeadGuitarArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_lead_guitar(note: &Note<LeadGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let frequency_hz = midi_to_hz(note.n_midi);
    let velocity_gain = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / (duration.as_secs_f32().max(1e-6));
        let sample = articulation_sample(note.articulation, frequency_hz, phase, t, expression);
        let envelope = articulation_envelope(note.articulation, phase);
        pcm.push((sample * envelope * velocity_gain).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: LeadGuitarArticulation) -> Duration {
    let scale = match articulation {
        LeadGuitarArticulation::Sustained => 1.15,
        LeadGuitarArticulation::PalmMuted => 0.38,
        LeadGuitarArticulation::HammerOn => 0.9,
        LeadGuitarArticulation::PullOff => 0.82,
        LeadGuitarArticulation::NaturalHarmonic => 1.2,
        LeadGuitarArticulation::PinchHarmonic => 0.95,
        LeadGuitarArticulation::Slide => 1.0,
        LeadGuitarArticulation::RhythmicRake => 0.22,
    };

    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: LeadGuitarArticulation) -> (f32, f32) {
    match articulation {
        LeadGuitarArticulation::Sustained => (0.9, 1.0),
        LeadGuitarArticulation::PalmMuted => (0.75, 1.0),
        LeadGuitarArticulation::HammerOn => (0.8, 1.01),
        LeadGuitarArticulation::PullOff => (0.72, 0.995),
        LeadGuitarArticulation::NaturalHarmonic => (0.82, 2.0),
        LeadGuitarArticulation::PinchHarmonic => (0.95, 1.0),
        LeadGuitarArticulation::Slide => (0.88, 1.0),
        LeadGuitarArticulation::RhythmicRake => (0.68, 1.0),
    }
}

fn articulation_sample(
    articulation: LeadGuitarArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let vibrato_depth = expression.vibrato.clamp(-1.0, 1.0) * 0.008;
    let vibrato = (TAU * 6.0 * t).sin() * vibrato_depth;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.25;

    match articulation {
        LeadGuitarArticulation::Sustained => {
            let f = base_hz * (1.0 + bend + vibrato);
            saw(f, t) * 0.7 + sine(f * 2.0, t) * 0.25
        }
        LeadGuitarArticulation::PalmMuted => {
            let f = base_hz * (1.0 + bend * 0.4);
            (saw(f, t) * 0.45 + saw(f * 2.0, t) * 0.25) * (1.0 - (phase * 0.8))
        }
        LeadGuitarArticulation::HammerOn => {
            let attack_f = base_hz * (0.96 + 0.04 * smoothstep(phase * 5.0));
            sine(attack_f * (1.0 + vibrato), t) * 0.7 + saw(attack_f, t) * 0.2
        }
        LeadGuitarArticulation::PullOff => {
            let f = base_hz * (1.0 + bend * 0.2 + vibrato);
            (sine(f, t) * 0.75 + sine(f * 1.5, t) * 0.2) * (1.0 - phase * 0.5)
        }
        LeadGuitarArticulation::NaturalHarmonic => {
            let f = base_hz * 2.0 * (1.0 + vibrato * 0.4);
            sine(f, t) * 0.8 + sine(f * 2.0, t) * 0.18 + sine(f * 3.0, t) * 0.08
        }
        LeadGuitarArticulation::PinchHarmonic => {
            let f = base_hz * 4.0 * (1.0 + bend * 0.6 + vibrato * 0.5);
            let raw = saw(f, t) * 0.9 + sine(f * 0.5, t) * 0.2;
            (raw * 2.2).tanh()
        }
        LeadGuitarArticulation::Slide => {
            let glide = smoothstep(phase);
            let f = base_hz * (0.72 + glide * 0.28) * (1.0 + vibrato * 0.8);
            saw(f, t) * 0.6 + sine(f * 2.0, t) * 0.2
        }
        LeadGuitarArticulation::RhythmicRake => {
            let noise = hash_noise(t * 22_000.0);
            let muted_tone = saw(base_hz * 0.5, t) * 0.2;
            (noise * 0.75 + muted_tone) * (1.0 - phase).max(0.0)
        }
    }
}

fn articulation_envelope(articulation: LeadGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        LeadGuitarArticulation::HammerOn => 0.08,
        LeadGuitarArticulation::PullOff => 0.03,
        LeadGuitarArticulation::RhythmicRake => 0.01,
        _ => 0.02,
    };

    let decay = match articulation {
        LeadGuitarArticulation::PalmMuted => 0.95,
        LeadGuitarArticulation::RhythmicRake => 1.35,
        LeadGuitarArticulation::NaturalHarmonic => 0.45,
        LeadGuitarArticulation::PinchHarmonic => 0.65,
        _ => 0.55,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn midi_to_hz(midi: u8) -> f32 {
    let semitones = midi as f32 - 69.0;
    440.0 * 2.0_f32.powf(semitones / 12.0)
}

fn duration_to_frames(duration: Duration, sample_rate: u32) -> usize {
    let seconds = duration.as_secs() as usize * sample_rate as usize;
    let nanos = (duration.subsec_nanos() as usize * sample_rate as usize) / 1_000_000_000usize;
    seconds.saturating_add(nanos)
}

fn saw(frequency_hz: f32, t: f32) -> f32 {
    let phase = (t * frequency_hz).fract();
    (phase * 2.0) - 1.0
}

fn sine(frequency_hz: f32, t: f32) -> f32 {
    (TAU * frequency_hz * t).sin()
}

fn smoothstep(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash_noise(x: f32) -> f32 {
    let n = (x * 12.9898).sin() * 43_758.547;
    ((n.fract() * 2.0) - 1.0).clamp(-1.0, 1.0)
}
