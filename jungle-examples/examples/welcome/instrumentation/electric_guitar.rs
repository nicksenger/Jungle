use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{amplitude_gain, Error, Expression, Instrument, Note};

pub struct ElectricGuitar {
    audio: AudioHandle,
}

impl ElectricGuitar {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }

    pub fn audio(&self) -> &AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ElectricGuitarArticulation {
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

    /// A sustained rhythm-guitar chord voice.
    RhythmSustained,

    /// A tightly palm-muted rhythm-guitar chord voice.
    RhythmPalmMuted,

    /// Lifting the fretting hand immediately after striking to choke the chord.
    /// Crucial for the staccato, funky stabs in the verse groove.
    Choked,

    /// Striking strings completely muted by the left hand.
    /// Used heavily during the scratchy intro buildup before the full band kicks in.
    RhythmicScratch,

    /// Sliding an entire chord shape up or down the neck.
    ChordSlide,
}

impl Instrument for ElectricGuitar {
    type Articulation = ElectricGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate, pan) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_electric_guitar(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = pan;

        self.audio.play(request).await.map_err(|_| Error::Submission)
    }
}

const SAMPLE_RATE: u32 = 48_000;

fn synthesize_electric_guitar(
    note: &Note<ElectricGuitarArticulation>,
) -> (Arc<[f32]>, f32, f32, f32) {
    if note.articulation.is_rhythm_voice() {
        let (pcm, gain, playback_rate) = synthesize_rhythm_guitar(note);
        (pcm, gain, playback_rate, -0.25)
    } else {
        let (pcm, gain, playback_rate) = synthesize_lead_guitar(note);
        (pcm, gain, playback_rate, 0.12)
    }
}

impl ElectricGuitarArticulation {
    fn is_rhythm_voice(self) -> bool {
        matches!(
            self,
            Self::RhythmSustained
                | Self::RhythmPalmMuted
                | Self::Choked
                | Self::RhythmicScratch
                | Self::ChordSlide
        )
    }
}

#[derive(Clone, Copy)]
struct ElectricTone {
    drive: f32,
    pick_amount: f32,
    cab_smoothing: f32,
    body_mix: f32,
}

fn synthesize_lead_guitar(note: &Note<ElectricGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = lead_duration(note.duration, note.articulation);
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

    let tone = lead_tone(note.articulation);
    let sustain = 0.35 + velocity_gain * 0.5;

    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);

        let raw = lead_sample(note.articulation, frequency_hz, phase, t, expression, tone);
        let envelope = lead_envelope(note.articulation, phase) * sustain;
        let picked = raw + lead_pick_attack(frequency_hz, phase, t, tone.pick_amount);
        let driven = lead_amp_distortion(picked * envelope, tone.drive);

        // Simple cabinet/body voicing so the oscillator stack lands closer to a mic'd amp.
        cab_lowpass += tone.cab_smoothing * (driven - cab_lowpass);
        body_highpass = tone.body_mix * (body_highpass + cab_lowpass - prev_cab_lowpass);
        prev_cab_lowpass = cab_lowpass;

        let sample = (cab_lowpass + body_highpass * 0.5).clamp(-1.0, 1.0);
        pcm.push(sample * velocity_gain);
    }

    let (gain, playback_rate) = lead_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn lead_tone(articulation: ElectricGuitarArticulation) -> ElectricTone {
    match articulation {
        ElectricGuitarArticulation::Sustained => ElectricTone {
            drive: 3.0,
            pick_amount: 0.32,
            cab_smoothing: 0.16,
            body_mix: 0.06,
        },
        ElectricGuitarArticulation::PalmMuted => ElectricTone {
            drive: 2.6,
            pick_amount: 0.36,
            cab_smoothing: 0.12,
            body_mix: 0.05,
        },
        ElectricGuitarArticulation::HammerOn => ElectricTone {
            drive: 2.4,
            pick_amount: 0.08,
            cab_smoothing: 0.14,
            body_mix: 0.05,
        },
        ElectricGuitarArticulation::PullOff => ElectricTone {
            drive: 2.2,
            pick_amount: 0.04,
            cab_smoothing: 0.14,
            body_mix: 0.05,
        },
        ElectricGuitarArticulation::NaturalHarmonic => ElectricTone {
            drive: 1.9,
            pick_amount: 0.12,
            cab_smoothing: 0.17,
            body_mix: 0.04,
        },
        ElectricGuitarArticulation::PinchHarmonic => ElectricTone {
            drive: 3.4,
            pick_amount: 0.22,
            cab_smoothing: 0.12,
            body_mix: 0.08,
        },
        ElectricGuitarArticulation::Slide => ElectricTone {
            drive: 2.8,
            pick_amount: 0.2,
            cab_smoothing: 0.15,
            body_mix: 0.06,
        },
        ElectricGuitarArticulation::RhythmicRake => ElectricTone {
            drive: 2.5,
            pick_amount: 0.46,
            cab_smoothing: 0.09,
            body_mix: 0.1,
        },
        ElectricGuitarArticulation::RhythmSustained
        | ElectricGuitarArticulation::RhythmPalmMuted
        | ElectricGuitarArticulation::Choked
        | ElectricGuitarArticulation::RhythmicScratch
        | ElectricGuitarArticulation::ChordSlide => ElectricTone {
            drive: 2.6,
            pick_amount: 0.3,
            cab_smoothing: 0.12,
            body_mix: 0.06,
        },
    }
}

fn lead_duration(base: Duration, articulation: ElectricGuitarArticulation) -> Duration {
    let scale = match articulation {
        ElectricGuitarArticulation::Sustained => 1.15,
        ElectricGuitarArticulation::PalmMuted => 0.38,
        ElectricGuitarArticulation::HammerOn => 0.9,
        ElectricGuitarArticulation::PullOff => 0.82,
        ElectricGuitarArticulation::NaturalHarmonic => 1.2,
        ElectricGuitarArticulation::PinchHarmonic => 0.95,
        ElectricGuitarArticulation::Slide => 1.0,
        ElectricGuitarArticulation::RhythmicRake => 0.22,
        ElectricGuitarArticulation::RhythmSustained
        | ElectricGuitarArticulation::RhythmPalmMuted
        | ElectricGuitarArticulation::Choked
        | ElectricGuitarArticulation::RhythmicScratch
        | ElectricGuitarArticulation::ChordSlide => 0.8,
    };

    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn lead_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::Sustained => (0.86, 1.0),
        ElectricGuitarArticulation::PalmMuted => (0.73, 1.0),
        ElectricGuitarArticulation::HammerOn => (0.8, 1.01),
        ElectricGuitarArticulation::PullOff => (0.72, 0.995),
        ElectricGuitarArticulation::NaturalHarmonic => (0.8, 2.0),
        ElectricGuitarArticulation::PinchHarmonic => (0.9, 1.0),
        ElectricGuitarArticulation::Slide => (0.84, 1.0),
        ElectricGuitarArticulation::RhythmicRake => (0.66, 1.0),
        ElectricGuitarArticulation::RhythmSustained
        | ElectricGuitarArticulation::RhythmPalmMuted
        | ElectricGuitarArticulation::Choked
        | ElectricGuitarArticulation::RhythmicScratch
        | ElectricGuitarArticulation::ChordSlide => (0.8, 1.0),
    }
}

fn lead_sample(
    articulation: ElectricGuitarArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
    tone: ElectricTone,
) -> f32 {
    let vibrato_depth = expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let vibrato = (TAU * 6.1 * t).sin() * vibrato_depth;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.32;

    match articulation {
        ElectricGuitarArticulation::Sustained => {
            let f = base_hz * (1.0 + bend + vibrato);
            lead_stack(f, t, 0.95, tone.drive)
        }
        ElectricGuitarArticulation::PalmMuted => {
            let f = base_hz * (1.0 + bend * 0.2);
            lead_stack(f, t, 0.65, tone.drive) * (1.0 - phase * 0.82)
        }
        ElectricGuitarArticulation::HammerOn => {
            let attack_f = base_hz * (0.94 + 0.06 * smoothstep(phase * 6.0));
            lead_stack(attack_f * (1.0 + vibrato * 0.7), t, 0.74, tone.drive)
        }
        ElectricGuitarArticulation::PullOff => {
            let f = base_hz * (1.0 + bend * 0.18 + vibrato * 0.85);
            lead_stack(f, t, 0.68, tone.drive) * (1.0 - phase * 0.42)
        }
        ElectricGuitarArticulation::NaturalHarmonic => {
            let f = base_hz * 2.0 * (1.0 + vibrato * 0.4);
            let bell = sine(f, t) * 0.68 + sine(f * 2.0, t) * 0.24 + sine(f * 3.0, t) * 0.1;
            (bell * 1.35).tanh()
        }
        ElectricGuitarArticulation::PinchHarmonic => {
            let f = base_hz * 3.8 * (1.0 + bend * 0.6 + vibrato * 0.5);
            let squeal = saw(f, t) * 0.75 + sine(f * 1.5, t) * 0.26 + sine(f * 2.5, t) * 0.17;
            lead_amp_distortion(squeal, tone.drive + 0.8)
        }
        ElectricGuitarArticulation::Slide => {
            let glide = smoothstep(phase);
            let f = base_hz * (0.74 + glide * 0.26) * (1.0 + vibrato * 0.9);
            lead_stack(f, t, 0.86, tone.drive)
        }
        ElectricGuitarArticulation::RhythmicRake => {
            let noise = hash_noise(t * 20_000.0) * 0.78;
            let muted_tone = lead_stack(base_hz * 0.5, t, 0.4, tone.drive) * 0.42;
            (noise + muted_tone) * (1.0 - phase).max(0.0)
        }
        ElectricGuitarArticulation::RhythmSustained
        | ElectricGuitarArticulation::RhythmPalmMuted
        | ElectricGuitarArticulation::Choked
        | ElectricGuitarArticulation::RhythmicScratch
        | ElectricGuitarArticulation::ChordSlide => lead_stack(base_hz, t, 0.7, tone.drive),
    }
}

fn lead_envelope(articulation: ElectricGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        ElectricGuitarArticulation::HammerOn => 0.07,
        ElectricGuitarArticulation::PullOff => 0.03,
        ElectricGuitarArticulation::RhythmicRake => 0.01,
        ElectricGuitarArticulation::PinchHarmonic => 0.015,
        _ => 0.018,
    };

    let decay = match articulation {
        ElectricGuitarArticulation::PalmMuted => 0.96,
        ElectricGuitarArticulation::RhythmicRake => 1.35,
        ElectricGuitarArticulation::NaturalHarmonic => 0.38,
        ElectricGuitarArticulation::PinchHarmonic => 0.58,
        _ => 0.5,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn lead_stack(frequency_hz: f32, t: f32, body: f32, drive: f32) -> f32 {
    let raw = saw(frequency_hz, t) * 0.46
        + saw(frequency_hz * 2.0, t) * 0.24
        + sine(frequency_hz * 3.0, t) * 0.18
        + sine(frequency_hz * 5.0, t) * 0.08;
    lead_amp_distortion(raw * body, drive)
}

fn lead_pick_attack(frequency_hz: f32, phase: f32, t: f32, amount: f32) -> f32 {
    let transient = (1.0 - smoothstep(phase * 18.0)).max(0.0);
    let noise = hash_noise((t + frequency_hz * 0.0008) * 18_500.0) * 0.8;
    let click = sine(frequency_hz * 6.0, t) * 0.2;
    (noise + click) * transient * amount
}

fn lead_amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.12).clamp(-2.2, 2.2);
    (asym.tanh() * 1.08).clamp(-1.0, 1.0)
}

#[derive(Clone, Copy)]
struct GrooveShape {
    downstroke: f32,
    amp_jitter: f32,
}

fn groove_shape(duration: Duration, n_midi: u8) -> GrooveShape {
    let micros = duration.as_micros() as f32;
    let stroke_clock =
        smoothstep((((micros * 0.000_015) + n_midi as f32 * 0.01).sin() + 1.0) * 0.5);
    let downstroke = 0.82 + stroke_clock * 0.36;
    let amp_jitter = hash_noise(micros * 0.000_03 + n_midi as f32 * 0.13) * 0.08;
    GrooveShape {
        downstroke,
        amp_jitter,
    }
}

#[derive(Clone, Copy)]
struct RhythmTone {
    drive: f32,
    pick_amount: f32,
    pre_gain: f32,
    cab_smoothing: f32,
    body_mix: f32,
}

fn synthesize_rhythm_guitar(note: &Note<ElectricGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = rhythm_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let root_hz = midi_to_hz(note.n_midi).max(70.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });
    let groove = groove_shape(note.duration, note.n_midi);
    let tone = rhythm_tone(note.articulation, groove);

    let mut pcm = Vec::with_capacity(frame_count);
    let mut cab_lowpass = 0.0;
    let mut body_highpass = 0.0;
    let mut prev_cab_lowpass = 0.0;

    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);

        let raw = rhythm_sample(note.articulation, root_hz, phase, t, expression, groove);
        let picked = raw + rhythm_pick_attack(root_hz, phase, t, tone.pick_amount, groove);
        let env = rhythm_envelope(note.articulation, phase);

        let driven = rhythm_amp_distortion(picked * env * tone.pre_gain, tone.drive);

        cab_lowpass += tone.cab_smoothing * (driven - cab_lowpass);
        body_highpass = tone.body_mix * (body_highpass + cab_lowpass - prev_cab_lowpass);
        prev_cab_lowpass = cab_lowpass;

        let sample = (cab_lowpass + body_highpass * 0.5).clamp(-1.0, 1.0);
        pcm.push(sample * velocity);
    }

    let (gain, playback_rate) = rhythm_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn rhythm_tone(articulation: ElectricGuitarArticulation, groove: GrooveShape) -> RhythmTone {
    match articulation {
        ElectricGuitarArticulation::RhythmSustained => RhythmTone {
            drive: 2.55 + groove.amp_jitter,
            pick_amount: 0.26 * groove.downstroke,
            pre_gain: 1.0,
            cab_smoothing: 0.14,
            body_mix: 0.06,
        },
        ElectricGuitarArticulation::RhythmPalmMuted => RhythmTone {
            drive: 2.25 + groove.amp_jitter,
            pick_amount: 0.34 * groove.downstroke,
            pre_gain: 0.96,
            cab_smoothing: 0.11,
            body_mix: 0.09,
        },
        ElectricGuitarArticulation::Choked => RhythmTone {
            drive: 2.0 + groove.amp_jitter * 0.8,
            pick_amount: 0.3,
            pre_gain: 0.92,
            cab_smoothing: 0.1,
            body_mix: 0.08,
        },
        ElectricGuitarArticulation::RhythmicScratch => RhythmTone {
            drive: 1.65 + groove.amp_jitter * 0.7,
            pick_amount: 0.48,
            pre_gain: 0.9,
            cab_smoothing: 0.08,
            body_mix: 0.12,
        },
        ElectricGuitarArticulation::ChordSlide => RhythmTone {
            drive: 2.45 + groove.amp_jitter,
            pick_amount: 0.24,
            pre_gain: 1.02,
            cab_smoothing: 0.13,
            body_mix: 0.06,
        },
        ElectricGuitarArticulation::Sustained
        | ElectricGuitarArticulation::PalmMuted
        | ElectricGuitarArticulation::HammerOn
        | ElectricGuitarArticulation::PullOff
        | ElectricGuitarArticulation::NaturalHarmonic
        | ElectricGuitarArticulation::PinchHarmonic
        | ElectricGuitarArticulation::Slide
        | ElectricGuitarArticulation::RhythmicRake => RhythmTone {
            drive: 2.4,
            pick_amount: 0.3,
            pre_gain: 1.0,
            cab_smoothing: 0.12,
            body_mix: 0.08,
        },
    }
}

fn rhythm_duration(base: Duration, articulation: ElectricGuitarArticulation) -> Duration {
    let scale = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 1.1,
        ElectricGuitarArticulation::RhythmPalmMuted => 0.34,
        ElectricGuitarArticulation::Choked => 0.2,
        ElectricGuitarArticulation::RhythmicScratch => 0.16,
        ElectricGuitarArticulation::ChordSlide => 0.95,
        _ => 1.0,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

fn rhythm_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::RhythmSustained => (0.9, 1.0),
        ElectricGuitarArticulation::RhythmPalmMuted => (0.78, 1.0),
        ElectricGuitarArticulation::Choked => (0.74, 1.0),
        ElectricGuitarArticulation::RhythmicScratch => (0.69, 1.0),
        ElectricGuitarArticulation::ChordSlide => (0.88, 1.0),
        _ => (0.84, 1.0),
    }
}

fn rhythm_sample(
    articulation: ElectricGuitarArticulation,
    root_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
    groove: GrooveShape,
) -> f32 {
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.1;
    let vibrato = expression.vibrato.clamp(-1.0, 1.0) * 0.005;
    let wobble = triangle(5.4, t) * vibrato;

    match articulation {
        ElectricGuitarArticulation::RhythmSustained => {
            let f = root_hz * (1.0 + bend + wobble);
            rhythm_stack(f, t, 0.88, 0.45 * groove.downstroke)
        }
        ElectricGuitarArticulation::RhythmPalmMuted => {
            let f = root_hz * (1.0 + bend * 0.25);
            rhythm_stack(f, t, 0.68, 0.22 * groove.downstroke) * (1.0 - phase * 0.92)
        }
        ElectricGuitarArticulation::Choked => {
            let f = root_hz * (1.0 + bend * 0.14);
            let gate = (1.0 - smoothstep(phase * 3.6)).max(0.0);
            rhythm_stack(f, t, 0.72, 0.34) * gate
        }
        ElectricGuitarArticulation::RhythmicScratch => {
            let noise = hash_noise((t + root_hz * 0.0002) * 20_000.0) * 0.72;
            let muted = rhythm_stack(root_hz * 0.48, t, 0.4, 0.0) * 0.34;
            (noise + muted) * (1.0 - phase * 1.2).max(0.0)
        }
        ElectricGuitarArticulation::ChordSlide => {
            let glide = smoothstep(phase);
            let ratio = 0.82 + glide * 0.18;
            let f = root_hz * ratio * (1.0 + wobble * 0.6);
            rhythm_stack(f, t, 0.83, 0.38)
        }
        _ => rhythm_stack(root_hz, t, 0.82, 0.28),
    }
}

fn rhythm_stack(frequency_hz: f32, t: f32, body: f32, top_end: f32) -> f32 {
    let fifth = frequency_hz * 2.0_f32.powf(7.0 / 12.0);
    let octave = frequency_hz * 2.0;
    let raw = saw(frequency_hz, t) * 0.52
        + saw(fifth, t) * 0.34
        + saw(octave, t) * (0.1 + top_end * 0.06)
        + triangle(frequency_hz * 3.0, t) * (0.05 + top_end * 0.04);
    (raw * body).clamp(-1.4, 1.4)
}

fn rhythm_envelope(articulation: ElectricGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        ElectricGuitarArticulation::RhythmicScratch => 0.006,
        ElectricGuitarArticulation::RhythmPalmMuted => 0.008,
        ElectricGuitarArticulation::Choked => 0.009,
        _ => 0.014,
    };

    let decay = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 0.5,
        ElectricGuitarArticulation::ChordSlide => 0.56,
        ElectricGuitarArticulation::RhythmPalmMuted => 1.26,
        ElectricGuitarArticulation::Choked => 1.62,
        ElectricGuitarArticulation::RhythmicScratch => 1.78,
        _ => 0.7,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn rhythm_pick_attack(
    frequency_hz: f32,
    phase: f32,
    t: f32,
    amount: f32,
    groove: GrooveShape,
) -> f32 {
    let transient = (1.0 - smoothstep(phase * 20.0)).max(0.0);
    let edge = hash_noise((t + frequency_hz * 0.0006) * 18_700.0) * (0.62 * groove.downstroke);
    let scrape = saw(frequency_hz * 5.0, t).abs() * 0.24;
    (edge + scrape) * transient * amount
}

fn rhythm_amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.1).clamp(-2.2, 2.2);
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

fn triangle(frequency_hz: f32, t: f32) -> f32 {
    let phase = (t * frequency_hz).fract();
    (4.0 * (phase - 0.5).abs() - 1.0).clamp(-1.0, 1.0)
}

fn smoothstep(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash_noise(x: f32) -> f32 {
    let n = (x * 12.9898).sin() * 43_758.547;
    ((n.fract() * 2.0) - 1.0).clamp(-1.0, 1.0)
}
