use std::{f32::consts::TAU, sync::Arc, time::Duration};

use super::{Expression, Note, SAMPLE_RATE};

#[derive(Debug, Clone, Copy)]
pub enum ElectricGuitarArticulation {
    Sustained,
    RhythmSustained,
}

pub fn synthesize_electric_guitar(
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
        matches!(self, Self::RhythmSustained)
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
        ElectricGuitarArticulation::RhythmSustained => ElectricTone {
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
        ElectricGuitarArticulation::RhythmSustained => 0.8,
    };

    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn lead_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::Sustained => (0.86, 1.0),
        ElectricGuitarArticulation::RhythmSustained => (0.8, 1.0),
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
    let vibrato_depth = expression.vibrato.clamp(-1.0, 1.0) * 0.01;
    let vibrato = (TAU * 6.1 * t).sin() * vibrato_depth;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.32;

    match articulation {
        ElectricGuitarArticulation::Sustained => {
            let f = base_hz * (1.0 + bend + vibrato);
            lead_stack(f, t, 0.95, tone.drive)
        }
        ElectricGuitarArticulation::RhythmSustained => lead_stack(base_hz, t, 0.7, tone.drive),
    }
}

fn lead_envelope(articulation: ElectricGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        ElectricGuitarArticulation::Sustained => 0.018,
        ElectricGuitarArticulation::RhythmSustained => 0.018,
    };
    let decay = match articulation {
        ElectricGuitarArticulation::Sustained => 0.5,
        ElectricGuitarArticulation::RhythmSustained => 0.5,
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
        ElectricGuitarArticulation::Sustained => RhythmTone {
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
        ElectricGuitarArticulation::Sustained => 1.0,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.025))
}

fn rhythm_output_shape(articulation: ElectricGuitarArticulation) -> (f32, f32) {
    match articulation {
        ElectricGuitarArticulation::RhythmSustained => (0.9, 1.0),
        ElectricGuitarArticulation::Sustained => (0.84, 1.0),
    }
}

fn rhythm_sample(
    articulation: ElectricGuitarArticulation,
    root_hz: f32,
    _phase: f32,
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
        ElectricGuitarArticulation::Sustained => rhythm_stack(root_hz, t, 0.82, 0.28),
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
        ElectricGuitarArticulation::RhythmSustained => 0.014,
        ElectricGuitarArticulation::Sustained => 0.014,
    };

    let decay = match articulation {
        ElectricGuitarArticulation::RhythmSustained => 0.5,
        ElectricGuitarArticulation::Sustained => 0.7,
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
