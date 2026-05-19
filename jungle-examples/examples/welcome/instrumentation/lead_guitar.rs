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
        let (pcm, gain, playback_rate) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_lead_guitar(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = 0.12;

        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

const SAMPLE_RATE: u32 = 48_000;
fn synthesize_lead_guitar(note: &Note<LeadGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let frequency_hz = midi_to_hz(note.n_midi).max(80.0);
    let velocity_gain = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    let mut cab_lowpass = 0.0;
    let mut body_highpass = 0.0;
    let mut prev_cab_lowpass = 0.0;

    let tone = articulation_tone(note.articulation);
    let sustain = 0.35 + velocity_gain * 0.5;

    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);

        let raw = articulation_sample(note.articulation, frequency_hz, phase, t, expression, tone);
        let envelope = articulation_envelope(note.articulation, phase) * sustain;
        let picked = raw + pick_attack(frequency_hz, phase, t, tone.pick_amount);
        let driven = amp_distortion(picked * envelope, tone.drive);

        // Simple cabinet/body voicing so the oscillator stack lands closer to a mic'd amp.
        cab_lowpass += tone.cab_smoothing * (driven - cab_lowpass);
        body_highpass = tone.body_mix * (body_highpass + cab_lowpass - prev_cab_lowpass);
        prev_cab_lowpass = cab_lowpass;

        let sample = (cab_lowpass + body_highpass * 0.5).clamp(-1.0, 1.0);
        pcm.push(sample * velocity_gain);
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

#[derive(Clone, Copy)]
struct LeadTone {
    drive: f32,
    pick_amount: f32,
    cab_smoothing: f32,
    body_mix: f32,
}

fn articulation_tone(articulation: LeadGuitarArticulation) -> LeadTone {
    match articulation {
        LeadGuitarArticulation::Sustained => LeadTone {
            drive: 3.0,
            pick_amount: 0.32,
            cab_smoothing: 0.16,
            body_mix: 0.06,
        },
        LeadGuitarArticulation::PalmMuted => LeadTone {
            drive: 2.6,
            pick_amount: 0.36,
            cab_smoothing: 0.12,
            body_mix: 0.05,
        },
        LeadGuitarArticulation::HammerOn => LeadTone {
            drive: 2.4,
            pick_amount: 0.08,
            cab_smoothing: 0.14,
            body_mix: 0.05,
        },
        LeadGuitarArticulation::PullOff => LeadTone {
            drive: 2.2,
            pick_amount: 0.04,
            cab_smoothing: 0.14,
            body_mix: 0.05,
        },
        LeadGuitarArticulation::NaturalHarmonic => LeadTone {
            drive: 1.9,
            pick_amount: 0.12,
            cab_smoothing: 0.17,
            body_mix: 0.04,
        },
        LeadGuitarArticulation::PinchHarmonic => LeadTone {
            drive: 3.4,
            pick_amount: 0.22,
            cab_smoothing: 0.12,
            body_mix: 0.08,
        },
        LeadGuitarArticulation::Slide => LeadTone {
            drive: 2.8,
            pick_amount: 0.2,
            cab_smoothing: 0.15,
            body_mix: 0.06,
        },
        LeadGuitarArticulation::RhythmicRake => LeadTone {
            drive: 2.5,
            pick_amount: 0.46,
            cab_smoothing: 0.09,
            body_mix: 0.1,
        },
    }
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
        LeadGuitarArticulation::Sustained => (0.86, 1.0),
        LeadGuitarArticulation::PalmMuted => (0.73, 1.0),
        LeadGuitarArticulation::HammerOn => (0.8, 1.01),
        LeadGuitarArticulation::PullOff => (0.72, 0.995),
        LeadGuitarArticulation::NaturalHarmonic => (0.8, 2.0),
        LeadGuitarArticulation::PinchHarmonic => (0.9, 1.0),
        LeadGuitarArticulation::Slide => (0.84, 1.0),
        LeadGuitarArticulation::RhythmicRake => (0.66, 1.0),
    }
}

fn articulation_sample(
    articulation: LeadGuitarArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
    tone: LeadTone,
) -> f32 {
    let vibrato_depth = expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let vibrato = (TAU * 6.1 * t).sin() * vibrato_depth;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.32;

    match articulation {
        LeadGuitarArticulation::Sustained => {
            let f = base_hz * (1.0 + bend + vibrato);
            guitar_stack(f, t, 0.95, tone.drive)
        }
        LeadGuitarArticulation::PalmMuted => {
            let f = base_hz * (1.0 + bend * 0.2);
            guitar_stack(f, t, 0.65, tone.drive) * (1.0 - phase * 0.82)
        }
        LeadGuitarArticulation::HammerOn => {
            let attack_f = base_hz * (0.94 + 0.06 * smoothstep(phase * 6.0));
            guitar_stack(attack_f * (1.0 + vibrato * 0.7), t, 0.74, tone.drive)
        }
        LeadGuitarArticulation::PullOff => {
            let f = base_hz * (1.0 + bend * 0.18 + vibrato * 0.85);
            guitar_stack(f, t, 0.68, tone.drive) * (1.0 - phase * 0.42)
        }
        LeadGuitarArticulation::NaturalHarmonic => {
            let f = base_hz * 2.0 * (1.0 + vibrato * 0.4);
            let bell = sine(f, t) * 0.68 + sine(f * 2.0, t) * 0.24 + sine(f * 3.0, t) * 0.1;
            (bell * 1.35).tanh()
        }
        LeadGuitarArticulation::PinchHarmonic => {
            let f = base_hz * 3.8 * (1.0 + bend * 0.6 + vibrato * 0.5);
            let squeal = saw(f, t) * 0.75 + sine(f * 1.5, t) * 0.26 + sine(f * 2.5, t) * 0.17;
            amp_distortion(squeal, tone.drive + 0.8)
        }
        LeadGuitarArticulation::Slide => {
            let glide = smoothstep(phase);
            let f = base_hz * (0.74 + glide * 0.26) * (1.0 + vibrato * 0.9);
            guitar_stack(f, t, 0.86, tone.drive)
        }
        LeadGuitarArticulation::RhythmicRake => {
            let noise = hash_noise(t * 20_000.0) * 0.78;
            let muted_tone = guitar_stack(base_hz * 0.5, t, 0.4, tone.drive) * 0.42;
            (noise + muted_tone) * (1.0 - phase).max(0.0)
        }
    }
}

fn articulation_envelope(articulation: LeadGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        LeadGuitarArticulation::HammerOn => 0.07,
        LeadGuitarArticulation::PullOff => 0.03,
        LeadGuitarArticulation::RhythmicRake => 0.01,
        LeadGuitarArticulation::PinchHarmonic => 0.015,
        _ => 0.018,
    };

    let decay = match articulation {
        LeadGuitarArticulation::PalmMuted => 0.96,
        LeadGuitarArticulation::RhythmicRake => 1.35,
        LeadGuitarArticulation::NaturalHarmonic => 0.38,
        LeadGuitarArticulation::PinchHarmonic => 0.58,
        _ => 0.5,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn guitar_stack(frequency_hz: f32, t: f32, body: f32, drive: f32) -> f32 {
    let raw = saw(frequency_hz, t) * 0.46
        + saw(frequency_hz * 2.0, t) * 0.24
        + sine(frequency_hz * 3.0, t) * 0.18
        + sine(frequency_hz * 5.0, t) * 0.08;
    amp_distortion(raw * body, drive)
}

fn pick_attack(frequency_hz: f32, phase: f32, t: f32, amount: f32) -> f32 {
    let transient = (1.0 - smoothstep(phase * 18.0)).max(0.0);
    let noise = hash_noise((t + frequency_hz * 0.0008) * 18_500.0) * 0.8;
    let click = sine(frequency_hz * 6.0, t) * 0.2;
    (noise + click) * transient * amount
}

fn amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.12).clamp(-2.2, 2.2);
    (asym.tanh() * 1.08).clamp(-1.0, 1.0)
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
