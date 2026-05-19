use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct RhythmGuitar {
    audio: AudioHandle,
}

impl RhythmGuitar {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum RhythmGuitarArticulation {
    /// A standard, ringing open or barre chord.
    Sustained,
    /// Constant, tight palm-muting to drive the verses.
    PalmMuted,
    /// Lifting the fretting hand immediately after striking to choke the chord.
    /// Crucial for the staccato, funky stabs in the verse groove.
    Choked,
    /// Striking strings completely muted by the left hand.
    /// Used heavily during the scratchy intro buildup before the full band kicks in.
    RhythmicScratch,
    /// Sliding an entire chord shape up or down the neck.
    ChordSlide,
}

impl Instrument for RhythmGuitar {
    type Articulation = RhythmGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_rhythm_guitar(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_rhythm_guitar(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = -0.25;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<RhythmGuitarArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_rhythm_guitar(note: &Note<RhythmGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let root_hz = midi_to_hz(note.n_midi).max(70.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(super::Expression {
        bend: 0.0,
        vibrato: 0.0,
    });
    let groove = groove_shape(note.offset, note.n_midi);
    let tone = articulation_tone(note.articulation, groove);

    let mut pcm = Vec::with_capacity(frame_count);
    let mut cab_lowpass = 0.0;
    let mut body_highpass = 0.0;
    let mut prev_cab_lowpass = 0.0;

    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);

        let raw = articulation_sample(note.articulation, root_hz, phase, t, expression, groove);
        let picked = raw + pick_attack(root_hz, phase, t, tone.pick_amount, groove);
        let env = articulation_envelope(note.articulation, phase);

        let driven = amp_distortion(picked * env * tone.pre_gain, tone.drive);

        cab_lowpass += tone.cab_smoothing * (driven - cab_lowpass);
        body_highpass = tone.body_mix * (body_highpass + cab_lowpass - prev_cab_lowpass);
        prev_cab_lowpass = cab_lowpass;

        let sample = (cab_lowpass + body_highpass * 0.5).clamp(-1.0, 1.0);
        pcm.push(sample * velocity);
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

#[derive(Clone, Copy)]
struct GrooveShape {
    downstroke: f32,
    amp_jitter: f32,
}

fn groove_shape(offset: Duration, n_midi: u8) -> GrooveShape {
    let micros = offset.as_micros() as f32;
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

fn articulation_tone(articulation: RhythmGuitarArticulation, groove: GrooveShape) -> RhythmTone {
    match articulation {
        RhythmGuitarArticulation::Sustained => RhythmTone {
            drive: 2.55 + groove.amp_jitter,
            pick_amount: 0.26 * groove.downstroke,
            pre_gain: 1.0,
            cab_smoothing: 0.14,
            body_mix: 0.06,
        },
        RhythmGuitarArticulation::PalmMuted => RhythmTone {
            drive: 2.25 + groove.amp_jitter,
            pick_amount: 0.34 * groove.downstroke,
            pre_gain: 0.96,
            cab_smoothing: 0.11,
            body_mix: 0.09,
        },
        RhythmGuitarArticulation::Choked => RhythmTone {
            drive: 2.0 + groove.amp_jitter * 0.8,
            pick_amount: 0.3,
            pre_gain: 0.92,
            cab_smoothing: 0.1,
            body_mix: 0.08,
        },
        RhythmGuitarArticulation::RhythmicScratch => RhythmTone {
            drive: 1.65 + groove.amp_jitter * 0.7,
            pick_amount: 0.48,
            pre_gain: 0.9,
            cab_smoothing: 0.08,
            body_mix: 0.12,
        },
        RhythmGuitarArticulation::ChordSlide => RhythmTone {
            drive: 2.45 + groove.amp_jitter,
            pick_amount: 0.24,
            pre_gain: 1.02,
            cab_smoothing: 0.13,
            body_mix: 0.06,
        },
    }
}

fn articulation_duration(base: Duration, articulation: RhythmGuitarArticulation) -> Duration {
    let scale = match articulation {
        RhythmGuitarArticulation::Sustained => 1.1,
        RhythmGuitarArticulation::PalmMuted => 0.34,
        RhythmGuitarArticulation::Choked => 0.2,
        RhythmGuitarArticulation::RhythmicScratch => 0.16,
        RhythmGuitarArticulation::ChordSlide => 0.95,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

fn articulation_output_shape(articulation: RhythmGuitarArticulation) -> (f32, f32) {
    match articulation {
        RhythmGuitarArticulation::Sustained => (0.9, 1.0),
        RhythmGuitarArticulation::PalmMuted => (0.78, 1.0),
        RhythmGuitarArticulation::Choked => (0.74, 1.0),
        RhythmGuitarArticulation::RhythmicScratch => (0.69, 1.0),
        RhythmGuitarArticulation::ChordSlide => (0.88, 1.0),
    }
}

fn articulation_sample(
    articulation: RhythmGuitarArticulation,
    root_hz: f32,
    phase: f32,
    t: f32,
    expression: super::Expression,
    groove: GrooveShape,
) -> f32 {
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.1;
    let vibrato = expression.vibrato.clamp(-1.0, 1.0) * 0.005;
    let wobble = triangle(5.4, t) * vibrato;

    match articulation {
        RhythmGuitarArticulation::Sustained => {
            let f = root_hz * (1.0 + bend + wobble);
            rhythm_stack(f, t, 0.88, 0.45 * groove.downstroke)
        }
        RhythmGuitarArticulation::PalmMuted => {
            let f = root_hz * (1.0 + bend * 0.25);
            rhythm_stack(f, t, 0.68, 0.22 * groove.downstroke) * (1.0 - phase * 0.92)
        }
        RhythmGuitarArticulation::Choked => {
            let f = root_hz * (1.0 + bend * 0.14);
            let gate = (1.0 - smoothstep(phase * 3.6)).max(0.0);
            rhythm_stack(f, t, 0.72, 0.34) * gate
        }
        RhythmGuitarArticulation::RhythmicScratch => {
            let noise = hash_noise((t + root_hz * 0.0002) * 20_000.0) * 0.72;
            let muted = rhythm_stack(root_hz * 0.48, t, 0.4, 0.0) * 0.34;
            (noise + muted) * (1.0 - phase * 1.2).max(0.0)
        }
        RhythmGuitarArticulation::ChordSlide => {
            let glide = smoothstep(phase);
            let ratio = 0.82 + glide * 0.18;
            let f = root_hz * ratio * (1.0 + wobble * 0.6);
            rhythm_stack(f, t, 0.83, 0.38)
        }
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

fn articulation_envelope(articulation: RhythmGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        RhythmGuitarArticulation::RhythmicScratch => 0.006,
        RhythmGuitarArticulation::PalmMuted => 0.008,
        RhythmGuitarArticulation::Choked => 0.009,
        _ => 0.014,
    };

    let decay = match articulation {
        RhythmGuitarArticulation::Sustained => 0.5,
        RhythmGuitarArticulation::ChordSlide => 0.56,
        RhythmGuitarArticulation::PalmMuted => 1.26,
        RhythmGuitarArticulation::Choked => 1.62,
        RhythmGuitarArticulation::RhythmicScratch => 1.78,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn pick_attack(frequency_hz: f32, phase: f32, t: f32, amount: f32, groove: GrooveShape) -> f32 {
    let transient = (1.0 - smoothstep(phase * 20.0)).max(0.0);
    let edge = hash_noise((t + frequency_hz * 0.0006) * 18_700.0) * (0.62 * groove.downstroke);
    let scrape = saw(frequency_hz * 5.0, t).abs() * 0.24;
    (edge + scrape) * transient * amount
}

fn amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.1).clamp(-2.2, 2.2);
    (asym.tanh() * 1.08).clamp(-1.0, 1.0)
}
