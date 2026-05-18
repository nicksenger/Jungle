use std::time::Duration;

use crate::instrumentation::{LeadGuitarArticulation, Note};

const TICKS_PER_QUARTER_NOTE: u64 = 384;
const TEMPO_MICROS_PER_QUARTER_NOTE: u64 = 483_870;

#[derive(Clone, Copy)]
struct ScoreEvent {
    start_tick: u32,
    duration_tick: u32,
    n_midi: u8,
    velocity: u8,
}

const SCORE: &[ScoreEvent] = &[
    ScoreEvent { start_tick: 39168, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 39552, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 39936, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 40320, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 40704, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 41088, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 41472, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 41856, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 60672, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 61056, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 61440, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 61824, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 62208, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 62592, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 62976, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 63360, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 94464, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 94848, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 95232, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 95616, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 96000, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 96384, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 96768, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 97152, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 175872, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 176256, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 176640, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 177024, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 177408, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 177792, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 178176, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 178560, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 182016, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 182400, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 182784, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 183168, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 183552, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 183936, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 184320, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 184704, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 188160, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 188544, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 188928, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 189312, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 189696, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 190080, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 190464, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 190848, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 194304, duration_tick: 384, n_midi: 71, velocity: 37 },
    ScoreEvent { start_tick: 194688, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 195072, duration_tick: 384, n_midi: 68, velocity: 37 },
    ScoreEvent { start_tick: 195456, duration_tick: 384, n_midi: 66, velocity: 37 },
    ScoreEvent { start_tick: 195840, duration_tick: 384, n_midi: 73, velocity: 37 },
    ScoreEvent { start_tick: 196224, duration_tick: 384, n_midi: 72, velocity: 37 },
    ScoreEvent { start_tick: 196608, duration_tick: 384, n_midi: 70, velocity: 37 },
    ScoreEvent { start_tick: 196992, duration_tick: 384, n_midi: 68, velocity: 37 },
];

pub fn flute_score() -> Vec<Note<LeadGuitarArticulation>> {
    SCORE
        .iter()
        .map(|event| Note {
            n_midi: event.n_midi,
            duration: ticks_to_duration(event.duration_tick),
            velocity: event.velocity as f32 / 127.0,
            expression: None,
            offset: ticks_to_duration(event.start_tick),
            articulation: LeadGuitarArticulation::Sustained,
        })
        .collect()
}

fn ticks_to_duration(ticks: u32) -> Duration {
    let micros = (ticks as u64)
        .saturating_mul(TEMPO_MICROS_PER_QUARTER_NOTE)
        .saturating_add(TICKS_PER_QUARTER_NOTE / 2)
        / TICKS_PER_QUARTER_NOTE;
    Duration::from_micros(micros)
}
