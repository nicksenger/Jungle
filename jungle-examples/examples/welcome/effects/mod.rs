use jungle_sdk::effect;
use std::{marker::PhantomData, time::Duration};
use tokio::time::Instant;

use crate::ecosystem::WelcomeEcosystem;
use crate::instrumentation::{ElectricGuitarArticulation, Instrument, Note};

const TICKS_PER_BEAT: u32 = 384;
const MIN_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: Duration = Duration::from_millis(120);
const RHYTHM_AMPLITUDE_MULTIPLIER: f32 = 0.5;
const RHYTHM_PAN: f32 = 0.5;
const RHYTHM_VELOCITY: f32 = 37.0 / 127.0;

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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note] = rhythm_notes([NOTE], &timing, A::rhythm_sustained().into());
            play_one(jungle, note).await?;
        }
        timing.sleep_until_next_cycle().await;
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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note_one, note_two] =
                rhythm_notes([NOTE_ONE, NOTE_TWO], &timing, A::rhythm_sustained().into());
            play_two(jungle, note_one, note_two).await?;
        }
        timing.sleep_until_next_cycle().await;
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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note_one, note_two, note_three] = rhythm_notes(
                [NOTE_ONE, NOTE_TWO, NOTE_THREE],
                &timing,
                A::rhythm_sustained().into(),
            );
            play_three(jungle, note_one, note_two, note_three).await?;
        }
        timing.sleep_until_next_cycle().await;
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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note_one, note_two, note_three, note_four] = rhythm_notes(
                [NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_FOUR],
                &timing,
                A::rhythm_sustained().into(),
            );
            play_four(jungle, note_one, note_two, note_three, note_four).await?;
        }
        timing.sleep_until_next_cycle().await;
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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note_one, note_two, note_three, note_four, note_five] = rhythm_notes(
                [NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_FOUR, NOTE_FIVE],
                &timing,
                A::rhythm_sustained().into(),
            );
            play_five(jungle, note_one, note_two, note_three, note_four, note_five).await?;
        }
        timing.sleep_until_next_cycle().await;
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
        let timing = rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play() {
            let [note_one, note_two, note_three, note_four, note_five, note_six] = rhythm_notes(
                [NOTE_ONE, NOTE_TWO, NOTE_THREE, NOTE_FOUR, NOTE_FIVE, NOTE_SIX],
                &timing,
                A::rhythm_sustained().into(),
            );
            play_six(
                jungle, note_one, note_two, note_three, note_four, note_five, note_six,
            )
            .await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

struct RhythmTiming {
    note_duration: Duration,
    rest_duration: Duration,
    phase_offset: Duration,
    late_note_drop_threshold: Duration,
}

impl RhythmTiming {
    fn should_play(&self) -> bool {
        self.phase_offset <= self.late_note_drop_threshold
    }

    async fn sleep_until_next_cycle(&self) {
        let sleep_for = self.rest_duration.saturating_sub(self.phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }
    }
}

fn rhythm_timing(jungle: &WelcomeEcosystem, note_tick: u8, rest_tick: u8) -> RhythmTiming {
    let metronome = jungle.metronome();
    let beat_duration = metronome.beat_duration();
    let tick_duration = beat_duration.div_f32(TICKS_PER_BEAT as f32);
    let note_duration = duration_for_ticks(tick_duration, note_tick as u32);
    let rest_duration = duration_for_ticks(tick_duration, rest_tick as u32);
    let late_note_drop_threshold = late_note_drop_threshold(beat_duration);
    let phase_offset = current_phase_offset(metronome, rest_duration);
    RhythmTiming {
        note_duration,
        rest_duration,
        phase_offset,
        late_note_drop_threshold,
    }
}

fn rhythm_notes<const N: usize>(
    midi_notes: [u8; N],
    timing: &RhythmTiming,
    articulation: ElectricGuitarArticulation,
) -> [Note<ElectricGuitarArticulation>; N] {
    midi_notes.map(|n_midi| rhythm_note(n_midi, timing.note_duration, articulation))
}

fn rhythm_note(
    n_midi: u8,
    duration: Duration,
    articulation: ElectricGuitarArticulation,
) -> Note<ElectricGuitarArticulation> {
    Note {
        n_midi,
        amplitude_multiplier: RHYTHM_AMPLITUDE_MULTIPLIER,
        pan: RHYTHM_PAN,
        duration,
        velocity: RHYTHM_VELOCITY,
        expression: None,
        articulation,
    }
}

fn map_playback_err(
    result: Result<(), crate::instrumentation::Error>,
) -> Result<(), String> {
    result.map_err(|err| err.to_string())
}

async fn play_one(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    map_playback_err(jungle.rhythm_guitar().play(note_one).await)
}

async fn play_two(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
    note_two: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    let (first, second) = tokio::join!(
        jungle.rhythm_guitar().play(note_one),
        jungle.rhythm_guitar().play(note_two)
    );
    map_playback_err(first)?;
    map_playback_err(second)
}

async fn play_three(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
    note_two: Note<ElectricGuitarArticulation>,
    note_three: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    let (first, second, third) = tokio::join!(
        jungle.rhythm_guitar().play(note_one),
        jungle.rhythm_guitar().play(note_two),
        jungle.rhythm_guitar().play(note_three)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)
}

async fn play_four(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
    note_two: Note<ElectricGuitarArticulation>,
    note_three: Note<ElectricGuitarArticulation>,
    note_four: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    let (first, second, third, fourth) = tokio::join!(
        jungle.rhythm_guitar().play(note_one),
        jungle.rhythm_guitar().play(note_two),
        jungle.rhythm_guitar().play(note_three),
        jungle.rhythm_guitar().play(note_four)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)
}

async fn play_five(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
    note_two: Note<ElectricGuitarArticulation>,
    note_three: Note<ElectricGuitarArticulation>,
    note_four: Note<ElectricGuitarArticulation>,
    note_five: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    let (first, second, third, fourth, fifth) = tokio::join!(
        jungle.rhythm_guitar().play(note_one),
        jungle.rhythm_guitar().play(note_two),
        jungle.rhythm_guitar().play(note_three),
        jungle.rhythm_guitar().play(note_four),
        jungle.rhythm_guitar().play(note_five)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)?;
    map_playback_err(fifth)
}

async fn play_six(
    jungle: &WelcomeEcosystem,
    note_one: Note<ElectricGuitarArticulation>,
    note_two: Note<ElectricGuitarArticulation>,
    note_three: Note<ElectricGuitarArticulation>,
    note_four: Note<ElectricGuitarArticulation>,
    note_five: Note<ElectricGuitarArticulation>,
    note_six: Note<ElectricGuitarArticulation>,
) -> Result<(), String> {
    let (first, second, third, fourth, fifth, sixth) = tokio::join!(
        jungle.rhythm_guitar().play(note_one),
        jungle.rhythm_guitar().play(note_two),
        jungle.rhythm_guitar().play(note_three),
        jungle.rhythm_guitar().play(note_four),
        jungle.rhythm_guitar().play(note_five),
        jungle.rhythm_guitar().play(note_six)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)?;
    map_playback_err(fifth)?;
    map_playback_err(sixth)
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
