use jungle_sdk::effect;
use std::{marker::PhantomData, time::Duration};
use tokio::time::Instant;

use crate::ecosystem::WelcomeEcosystem;
use crate::instrumentation::{ElectricGuitarArticulation, Instrument, Note};

const TICKS_PER_BEAT: u32 = 384;
const MIN_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(120);

pub struct Monad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Dyad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Triad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Tetrad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Pentad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_FIVE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Hexad<
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_FIVE: u8,
    const NOTE_SIX: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub trait RhythmArticulation: Copy + Into<ElectricGuitarArticulation> {
    fn rhythm_sustained() -> Self;
}

impl RhythmArticulation for ElectricGuitarArticulation {
    fn rhythm_sustained() -> Self {
        Self::RhythmSustained
    }
}

#[effect(id = 500)]
impl<I, A, const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>
    jungle_sdk::prelude::Effect<WelcomeEcosystem> for Monad<I, A, NOTE, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let playable_note = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: duration_for_ticks(tick_duration, NOTE_TICK as u32),
            velocity: 37.0 / 127.0,
            expression: None,
            articulation: A::rhythm_sustained().into(),
        };

        if phase_offset <= late_note_drop_threshold {
            jungle
                .rhythm_guitar()
                .play(playable_note)
                .await
                .map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

#[effect(id = 501)]
impl<I, A, const NOTE_ONE: u8, const NOTE_TWO: u8, const NOTE_TICK: u8, const REST_TICK: u8>
    jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Dyad<I, A, NOTE_ONE, NOTE_TWO, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let note_duration = duration_for_ticks(tick_duration, NOTE_TICK as u32);
        let articulation = A::rhythm_sustained().into();
        let playable_note_one = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_ONE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_two = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_TWO,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };

        if phase_offset <= late_note_drop_threshold {
            let (first, second) = tokio::join!(
                jungle.rhythm_guitar().play(playable_note_one),
                jungle.rhythm_guitar().play(playable_note_two)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

#[effect(id = 502)]
impl<
    I,
    A,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
> jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Triad<I, A, NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let note_duration = duration_for_ticks(tick_duration, NOTE_TICK as u32);
        let articulation = A::rhythm_sustained().into();
        let playable_note_one = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_ONE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_two = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_TWO,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_three = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_THREE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };

        if phase_offset <= late_note_drop_threshold {
            let (first, second, third) = tokio::join!(
                jungle.rhythm_guitar().play(playable_note_one),
                jungle.rhythm_guitar().play(playable_note_two),
                jungle.rhythm_guitar().play(playable_note_three)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
            third.map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

#[effect(id = 504)]
impl<
    I,
    A,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
> jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Tetrad<I, A, NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_FOUR, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let note_duration = duration_for_ticks(tick_duration, NOTE_TICK as u32);
        let articulation = A::rhythm_sustained().into();
        let playable_note_one = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_ONE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_two = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_TWO,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_three = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_THREE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_four = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_FOUR,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };

        if phase_offset <= late_note_drop_threshold {
            let (first, second, third, fourth) = tokio::join!(
                jungle.rhythm_guitar().play(playable_note_one),
                jungle.rhythm_guitar().play(playable_note_two),
                jungle.rhythm_guitar().play(playable_note_three),
                jungle.rhythm_guitar().play(playable_note_four)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
            third.map_err(|err| err.to_string())?;
            fourth.map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

#[effect(id = 505)]
impl<
    I,
    A,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_FIVE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
> jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Pentad<I, A, NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_FOUR, NOTE_FIVE, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let note_duration = duration_for_ticks(tick_duration, NOTE_TICK as u32);
        let articulation = A::rhythm_sustained().into();
        let playable_note_one = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_ONE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_two = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_TWO,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_three = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_THREE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_four = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_FOUR,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_five = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_FIVE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };

        if phase_offset <= late_note_drop_threshold {
            let (first, second, third, fourth, fifth) = tokio::join!(
                jungle.rhythm_guitar().play(playable_note_one),
                jungle.rhythm_guitar().play(playable_note_two),
                jungle.rhythm_guitar().play(playable_note_three),
                jungle.rhythm_guitar().play(playable_note_four),
                jungle.rhythm_guitar().play(playable_note_five)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
            third.map_err(|err| err.to_string())?;
            fourth.map_err(|err| err.to_string())?;
            fifth.map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

#[effect(id = 506)]
impl<
    I,
    A,
    const NOTE_ONE: u8,
    const NOTE_TWO: u8,
    const NOTE_THREE: u8,
    const NOTE_FOUR: u8,
    const NOTE_FIVE: u8,
    const NOTE_SIX: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
> jungle_sdk::prelude::Effect<WelcomeEcosystem>
    for Hexad<
        I,
        A,
        NOTE_ONE,
        NOTE_TWO,
        NOTE_THREE,
        NOTE_FOUR,
        NOTE_FIVE,
        NOTE_SIX,
        NOTE_TICK,
        REST_TICK,
    >
where
    I: Instrument<Articulation = A>,
    A: RhythmArticulation,
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        let metronome = jungle.metronome();
        let beat_duration = metronome.beat_duration();
        let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
        let rest_duration = duration_for_ticks(tick_duration, REST_TICK as u32);
        let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
        let phase_offset = current_phase_offset(metronome, rest_duration);

        let note_duration = duration_for_ticks(tick_duration, NOTE_TICK as u32);
        let articulation = A::rhythm_sustained().into();
        let playable_note_one = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_ONE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_two = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_TWO,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_three = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_THREE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_four = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_FOUR,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_five = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_FIVE,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };
        let playable_note_six = Note::<ElectricGuitarArticulation> {
            n_midi: NOTE_SIX,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: note_duration,
            velocity: 37.0 / 127.0,
            expression: None,
            articulation,
        };

        if phase_offset <= late_note_drop_threshold {
            let (first, second, third, fourth, fifth, sixth) = tokio::join!(
                jungle.rhythm_guitar().play(playable_note_one),
                jungle.rhythm_guitar().play(playable_note_two),
                jungle.rhythm_guitar().play(playable_note_three),
                jungle.rhythm_guitar().play(playable_note_four),
                jungle.rhythm_guitar().play(playable_note_five),
                jungle.rhythm_guitar().play(playable_note_six)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
            third.map_err(|err| err.to_string())?;
            fourth.map_err(|err| err.to_string())?;
            fifth.map_err(|err| err.to_string())?;
            sixth.map_err(|err| err.to_string())?;
        }

        let sleep_for = rest_duration.saturating_sub(phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }

        Ok(())
    }
}

fn duration_for_ticks(tick_duration: Duration, ticks: u32) -> Duration {
    tick_duration.mul_f32(ticks as f32)
}

fn current_phase_offset(metronome: &crate::metronome::Metronome, period: Duration) -> Duration {
    if period.is_zero() {
        return Duration::ZERO;
    }

    let now = Instant::now();
    let anchor = metronome
        .latest_beat()
        .map(|event| event.timestamp)
        .unwrap_or_else(|| metronome.started_at());
    let elapsed = now.saturating_duration_since(anchor);
    duration_mod(elapsed, period)
}

fn duration_mod(value: Duration, modulus: Duration) -> Duration {
    if modulus.is_zero() {
        return Duration::ZERO;
    }
    let modulus_nanos = modulus.as_nanos();
    if modulus_nanos == 0 {
        return Duration::ZERO;
    }
    let remainder_nanos = value.as_nanos() % modulus_nanos;
    let bounded_nanos = remainder_nanos.min(u64::MAX as u128) as u64;
    Duration::from_nanos(bounded_nanos)
}

fn late_note_drop_threshold(beat: Duration) -> Duration {
    beat.div_f32(4.0)
        .clamp(MIN_LATE_NOTE_DROP_THRESHOLD, MAX_LATE_NOTE_DROP_THRESHOLD)
}
