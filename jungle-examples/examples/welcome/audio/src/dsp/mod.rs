use std::{f32::consts::TAU, time::Duration};

pub mod bass;
pub mod drums;
pub mod electric_guitar;
pub mod toms;
pub mod vocals;

pub const SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, Clone, Copy)]
pub struct Expression {
    pub bend: f32,
    pub vibrato: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Note<Articulation> {
    pub n_midi: u8,
    pub duration: Duration,
    pub velocity: f32,
    pub expression: Option<Expression>,
    pub articulation: Articulation,
}

pub(crate) fn duration_to_frames(duration: Duration, sample_rate: u32) -> usize {
    let seconds = duration.as_secs() as usize * sample_rate as usize;
    let nanos = (duration.subsec_nanos() as usize * sample_rate as usize) / 1_000_000_000usize;
    seconds.saturating_add(nanos)
}

pub(crate) fn midi_to_hz(midi: u8) -> f32 {
    let semitones = midi as f32 - 69.0;
    440.0 * 2.0_f32.powf(semitones / 12.0)
}

pub(crate) fn sine(frequency_hz: f32, t: f32) -> f32 {
    (TAU * frequency_hz * t).sin()
}

pub(crate) fn saw(frequency_hz: f32, t: f32) -> f32 {
    let phase = (t * frequency_hz).fract();
    (phase * 2.0) - 1.0
}

pub(crate) fn triangle(frequency_hz: f32, t: f32) -> f32 {
    let phase = (t * frequency_hz).fract();
    ((4.0 * (phase - 0.5).abs()) - 1.0).clamp(-1.0, 1.0)
}

pub(crate) fn hash_noise(x: f32) -> f32 {
    let n = (x * 12.9898).sin() * 43_758.547;
    ((n.fract() * 2.0) - 1.0).clamp(-1.0, 1.0)
}

pub(crate) fn smoothstep(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
