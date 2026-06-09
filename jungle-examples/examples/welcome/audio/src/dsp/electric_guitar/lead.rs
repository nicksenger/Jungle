use std::{f32::consts::TAU, sync::Arc, time::Duration};

use super::{ElectricGuitarArticulation, Expression, Note, SAMPLE_RATE};

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct ElectricTone {
    drive: f32,
    pick_amount: f32,
    cab_smoothing: f32,
    body_mix: f32,
    high_freq: f32,
}

fn lead_tone(articulation: ElectricGuitarArticulation) -> ElectricTone {
    match articulation {
        ElectricGuitarArticulation::Sustained => ElectricTone {
            drive: 38.0,
            pick_amount: 1.5,
            cab_smoothing: 0.04,
            body_mix: 0.03,
            high_freq: 6.0,
        },
        ElectricGuitarArticulation::RhythmSustained => ElectricTone {
            drive: 3.2,
            pick_amount: 0.38,
            cab_smoothing: 0.15,
            body_mix: 0.1,
            high_freq: 0.2,
        },
    }
}

fn lead_duration(base: Duration, articulation: ElectricGuitarArticulation) -> Duration {
    let scale = match articulation {
        ElectricGuitarArticulation::Sustained => 1.2,
        ElectricGuitarArticulation::RhythmSustained => 0.85,
    };

    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn lead_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::Sustained => (0.92, 1.0),
        ElectricGuitarArticulation::RhythmSustained => (0.86, 1.0),
    }
}

fn lead_sample(
    articulation: ElectricGuitarArticulation,
    base_hz: f32,
    _phase: f32,
    t: f32,
    expression: Expression,
    tone: ElectricTone,
) -> f32 {
    let vibrato_depth = expression.vibrato.clamp(-1.0, 1.0) * 0.012;
    let vibrato = (TAU * 6.5 * t).sin() * vibrato_depth;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.35;

    match articulation {
        ElectricGuitarArticulation::Sustained => {
            let f = base_hz * (1.0 + bend + vibrato);
            lead_stack(f, t, 0.98, tone.drive, tone.high_freq)
        }
        ElectricGuitarArticulation::RhythmSustained => {
            lead_stack(base_hz, t, 0.75, tone.drive, tone.high_freq)
        }
    }
}

fn lead_envelope(articulation: ElectricGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        ElectricGuitarArticulation::Sustained => 0.015,
        ElectricGuitarArticulation::RhythmSustained => 0.015,
    };
    let decay = match articulation {
        ElectricGuitarArticulation::Sustained => 0.45,
        ElectricGuitarArticulation::RhythmSustained => 0.45,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

fn lead_stack(frequency_hz: f32, t: f32, body: f32, drive: f32, high_freq: f32) -> f32 {
    let raw = saw(frequency_hz, t) * 0.48
        + saw(frequency_hz * 2.0, t) * 0.28
        + sine(frequency_hz * 3.0, t) * 0.16
        + sine(frequency_hz * 4.0, t) * 0.15 * (1.0 + high_freq)
        + sine(frequency_hz * 5.0, t) * 0.12 * (1.0 + high_freq)
        + sine(frequency_hz * 6.0, t) * 0.09 * (1.0 + high_freq)
        + sine(frequency_hz * 7.0, t) * 0.06 * (1.0 + high_freq)
        + sine(frequency_hz * 8.0, t) * 0.05 * (1.0 + high_freq * 1.2)
        + sine(frequency_hz * 9.0, t) * 0.04 * (1.0 + high_freq * 1.2)
        + sine(frequency_hz * 10.0, t) * 0.03 * (1.0 + high_freq * 1.2);
    lead_amp_distortion(raw * body, drive)
}

fn lead_pick_attack(frequency_hz: f32, phase: f32, t: f32, amount: f32) -> f32 {
    let transient = (1.0 - smoothstep(phase * 22.0)).max(0.0);
    let noise = hash_noise((t + frequency_hz * 0.0009) * 19_000.0) * 0.85;
    let click = sine(frequency_hz * 7.0, t) * 0.25;
    (noise + click) * transient * amount
}

fn lead_amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.15).clamp(-2.5, 2.5);
    (asym.tanh() * 1.15).clamp(-1.0, 1.0)
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct GrooveShape {
    downstroke: f32,
    amp_jitter: f32,
}

fn groove_shape(duration: Duration, n_midi: u8) -> GrooveShape {
    let micros = duration.as_micros() as f32;
    let stroke_clock =
        smoothstep((((micros * 0.000_015) + n_midi as f32 * 0.01).sin() + 1.0) * 0.5);
    let downstroke = 0.85 + stroke_clock * 0.35;
    let amp_jitter = hash_noise(micros * 0.000_03 + n_midi as f32 * 0.13) * 0.09;
    GrooveShape {
        downstroke,
        amp_jitter,
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct RhythmTone {
    drive: f32,
    pick_amount: f32,
    pre_gain: f32,
    cab_smoothing: f32,
    body_mix: f32,
}

pub fn synthesize_lead_guitar(note: &Note<ElectricGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = lead_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let root_hz = midi_to_hz(note.n_midi).max(70.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });
    let _groove = groove_shape(note.duration, note.n_midi);
    let tone = lead_tone(note.articulation);

    let mut pcm = Vec::with_capacity(frame_count);
    let mut cab_lowpass = 0.0;
    let mut body_highpass = 0.0;
    let mut prev_cab_lowpass = 0.0;

    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);

        let raw = lead_sample(note.articulation, root_hz, phase, t, expression, tone);
        let picked = raw + lead_pick_attack(root_hz, phase, t, tone.pick_amount);
        let env = lead_envelope(note.articulation, phase);

        let driven = lead_amp_distortion(picked * env, tone.drive);

        cab_lowpass += tone.cab_smoothing * (driven - cab_lowpass);
        body_highpass = tone.body_mix * (body_highpass + cab_lowpass - prev_cab_lowpass);
        prev_cab_lowpass = cab_lowpass;

        let sample = (cab_lowpass + body_highpass * 0.5).clamp(-1.0, 1.0);
        pcm.push(sample * velocity);
    }

    let (gain, playback_rate) = lead_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

#[allow(dead_code)]
fn rhythm_tone(articulation: ElectricGuitarArticulation, groove: GrooveShape) -> RhythmTone {
    match articulation {
        ElectricGuitarArticulation::RhythmSustained => RhythmTone {
            drive: 2.8 + groove.amp_jitter,
            pick_amount: 0.32 * groove.downstroke,
            pre_gain: 1.0,
            cab_smoothing: 0.16,
            body_mix: 0.08,
        },
        ElectricGuitarArticulation::Sustained => RhythmTone {
            drive: 2.6,
            pick_amount: 0.35,
            pre_gain: 1.0,
            cab_smoothing: 0.14,
            body_mix: 0.1,
        },
    }
}

#[allow(dead_code)]
fn rhythm_duration(base: Duration, articulation: ElectricGuitarArticulation) -> Duration {
    let scale = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 1.15,
        ElectricGuitarArticulation::Sustained => 1.05,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

#[allow(dead_code)]
fn rhythm_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::RhythmSustained => (0.94, 1.0),
        ElectricGuitarArticulation::Sustained => (0.88, 1.0),
    }
}

#[allow(dead_code)]
fn rhythm_sample(
    articulation: ElectricGuitarArticulation,
    root_hz: f32,
    _phase: f32,
    t: f32,
    expression: Expression,
    groove: GrooveShape,
) -> f32 {
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.12;
    let vibrato = expression.vibrato.clamp(-1.0, 1.0) * 0.006;
    let wobble = triangle(6.0, t) * vibrato;

    match articulation {
        ElectricGuitarArticulation::RhythmSustained => {
            let f = root_hz * (1.0 + bend + wobble);
            rhythm_stack(f, t, 0.92, 0.48 * groove.downstroke)
        }
        ElectricGuitarArticulation::Sustained => rhythm_stack(root_hz, t, 0.88, 0.32),
    }
}

#[allow(dead_code)]
fn rhythm_stack(frequency_hz: f32, t: f32, body: f32, top_end: f32) -> f32 {
    let fifth = frequency_hz * 2.0_f32.powf(7.0 / 12.0);
    let octave = frequency_hz * 2.0;
    let raw = saw(frequency_hz, t) * 0.55
        + saw(fifth, t) * 0.36
        + saw(octave, t) * (0.12 + top_end * 0.08)
        + triangle(frequency_hz * 3.0, t) * (0.06 + top_end * 0.05)
        + sine(frequency_hz * 4.0, t) * (0.03 + top_end * 0.03);
    (raw * body).clamp(-1.5, 1.5)
}

#[allow(dead_code)]
fn rhythm_envelope(articulation: ElectricGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 0.018,
        ElectricGuitarArticulation::Sustained => 0.018,
    };

    let decay = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 0.55,
        ElectricGuitarArticulation::Sustained => 0.75,
    };

    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}

#[allow(dead_code)]
fn rhythm_pick_attack(
    frequency_hz: f32,
    phase: f32,
    t: f32,
    amount: f32,
    groove: GrooveShape,
) -> f32 {
    let transient = (1.0 - smoothstep(phase * 24.0)).max(0.0);
    let edge = hash_noise((t + frequency_hz * 0.0007) * 19_000.0) * (0.68 * groove.downstroke);
    let scrape = saw(frequency_hz * 6.0, t).abs() * 0.28;
    (edge + scrape) * transient * amount
}

#[allow(dead_code)]
fn rhythm_amp_distortion(sample: f32, drive: f32) -> f32 {
    let pre = sample * drive;
    let asym = (pre + pre * pre.abs() * 0.12).clamp(-2.5, 2.5);
    (asym.tanh() * 1.15).clamp(-1.0, 1.0)
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

#[allow(dead_code)]
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
