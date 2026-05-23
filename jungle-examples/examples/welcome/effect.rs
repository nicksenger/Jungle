use jungle_sdk::effect;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

use crate::ecosystem::TheJungle;
use crate::instrumentation::{Instrument, Note};

const TICKS_PER_BEAT: u32 = 384;
const MIN_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(120);
const RHYTHM_AMPLITUDE_MULTIPLIER: f32 = 0.5;
const RHYTHM_PAN: f32 = 0.5;
const RHYTHM_VELOCITY: f32 = 37.0 / 127.0;

pub struct Hexad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_5: u8,
    const NOTE_6: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);

pub struct Pentad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_5: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);

pub struct Tetrad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);

pub struct Triad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);

pub struct Dyad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);

pub struct Monad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const LANE_ID: u32,
    const NOTE: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<(I, A)>);
pub struct ImmediateMonad<I: Instrument<Articulation = A>, A: Copy, const NOTE: u8, const NOTE_TICK: u32>(
    PhantomData<(I, A)>,
);
pub struct ImmediateDyad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u32,
>(PhantomData<(I, A)>);

pub struct DecrementCounterEffect;
pub struct Rest<const LANE_ID: u32, const REST_TICKS: u32>;
pub struct AtomicDualHit<
    I1: Instrument<Articulation = A1>,
    I2: Instrument<Articulation = A2>,
    A1: Copy,
    A2: Copy,
    const LANE_ID: u32,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK_1: u32,
    const NOTE_TICK_2: u32,
    const REST_TICK: u32,
>(PhantomData<(I1, I2, A1, A2)>);

#[effect(id = 515)]
impl<I, A, const NOTE: u8, const NOTE_TICK: u32> jungle_sdk::prelude::Effect<TheJungle>
    for ImmediateMonad<I, A, NOTE, NOTE_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let note_duration = jungle.metronome().duration_for_ticks(TICKS_PER_BEAT, NOTE_TICK);
        let [note] = rhythm_notes([NOTE], note_duration, articulation);
        play_one::<I>(jungle, note).await
    }
}

#[effect(id = 516)]
impl<I, A, const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32>
    jungle_sdk::prelude::Effect<TheJungle> for ImmediateDyad<I, A, NOTE_1, NOTE_2, NOTE_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let note_duration = jungle.metronome().duration_for_ticks(TICKS_PER_BEAT, NOTE_TICK);
        let [note_1, note_2] = rhythm_notes([NOTE_1, NOTE_2], note_duration, articulation);
        play_two::<I>(jungle, note_1, note_2).await
    }
}

#[effect(id = 512)]
impl<J> jungle_sdk::prelude::Effect<J> for DecrementCounterEffect {
    type In = ();
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        std::future::ready(Ok(()))
    }
}

#[effect(id = 513)]
impl<const LANE_ID: u32, const REST_TICKS: u32> jungle_sdk::prelude::Effect<TheJungle>
    for Rest<LANE_ID, REST_TICKS>
{
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, _input: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            0,
            REST_TICKS,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 514)]
impl<
        I1,
        I2,
        A1,
        A2,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK_1: u32,
        const NOTE_TICK_2: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for AtomicDualHit<I1, I2, A1, A2, LANE_ID, NOTE_1, NOTE_2, NOTE_TICK_1, NOTE_TICK_2, REST_TICK>
where
    I1: Instrument<Articulation = A1>,
    I2: Instrument<Articulation = A2>,
    for<'a> &'a I1: From<&'a TheJungle>,
    for<'a> &'a I2: From<&'a TheJungle>,
    A1: Copy + Serialize + DeserializeOwned + Send + 'static,
    A2: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = (A1, A2);
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, input: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK_1.max(NOTE_TICK_2),
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let note_1 = rhythm_note(
                NOTE_1,
                jungle
                    .metronome()
                    .duration_for_ticks(TICKS_PER_BEAT, NOTE_TICK_1),
                input.0,
            );
            let note_2 = rhythm_note(
                NOTE_2,
                jungle
                    .metronome()
                    .duration_for_ticks(TICKS_PER_BEAT, NOTE_TICK_2),
                input.1,
            );
            play_two_instruments::<I1, I2>(jungle, note_1, note_2).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 506)]
impl<
        I,
        A,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_5: u8,
        const NOTE_6: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Hexad<I, A, LANE_ID, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5, NOTE_6, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note_1, note_2, note_3, note_4, note_5, note_6] = rhythm_notes(
                [NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5, NOTE_6],
                timing.note_duration(),
                articulation,
            );
            play_six::<I>(jungle, note_1, note_2, note_3, note_4, note_5, note_6).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 505)]
impl<
        I,
        A,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_5: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Pentad<I, A, LANE_ID, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note_1, note_2, note_3, note_4, note_5] = rhythm_notes(
                [NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5],
                timing.note_duration(),
                articulation,
            );
            play_five::<I>(jungle, note_1, note_2, note_3, note_4, note_5).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 504)]
impl<
        I,
        A,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Tetrad<I, A, LANE_ID, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note_1, note_2, note_3, note_4] = rhythm_notes(
                [NOTE_1, NOTE_2, NOTE_3, NOTE_4],
                timing.note_duration(),
                articulation,
            );
            play_four::<I>(jungle, note_1, note_2, note_3, note_4).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 502)]
impl<
        I,
        A,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Triad<I, A, LANE_ID, NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note_1, note_2, note_3] = rhythm_notes(
                [NOTE_1, NOTE_2, NOTE_3],
                timing.note_duration(),
                articulation,
            );
            play_three::<I>(jungle, note_1, note_2, note_3).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 501)]
impl<
        I,
        A,
        const LANE_ID: u32,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_TICK: u32,
        const REST_TICK: u32,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Dyad<I, A, LANE_ID, NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note_1, note_2] =
                rhythm_notes([NOTE_1, NOTE_2], timing.note_duration(), articulation);
            play_two::<I>(jungle, note_1, note_2).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

#[effect(id = 500)]
impl<I, A, const LANE_ID: u32, const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>
    jungle_sdk::prelude::Effect<TheJungle> for Monad<I, A, LANE_ID, NOTE, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        jungle.metronome().wait_for_start_barrier().await;
        let timing = jungle.metronome().rhythm_timing(
            LANE_ID,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        timing.sleep_until_note_window().await;
        if timing.should_play() {
            let [note] = rhythm_notes([NOTE], timing.note_duration(), articulation);
            play_one::<I>(jungle, note).await?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

fn rhythm_notes<const N: usize, A: Copy>(
    midi_notes: [u8; N],
    duration: std::time::Duration,
    articulation: A,
) -> [Note<A>; N] {
    midi_notes.map(|n_midi| rhythm_note(n_midi, duration, articulation))
}

fn rhythm_note<A: Copy>(n_midi: u8, duration: std::time::Duration, articulation: A) -> Note<A> {
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

fn map_playback_err(result: Result<(), crate::instrumentation::Error>) -> Result<(), String> {
    result.map_err(|err| err.to_string())
}

async fn play_one<I>(jungle: &TheJungle, note_1: Note<I::Articulation>) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    map_playback_err(instrument.play(note_1).await)
}

async fn play_two<I>(
    jungle: &TheJungle,
    note_1: Note<I::Articulation>,
    note_2: Note<I::Articulation>,
) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    let (first, second) = tokio::join!(instrument.play(note_1), instrument.play(note_2));
    map_playback_err(first)?;
    map_playback_err(second)
}

async fn play_two_instruments<I1, I2>(
    jungle: &TheJungle,
    note_1: Note<I1::Articulation>,
    note_2: Note<I2::Articulation>,
) -> Result<(), String>
where
    I1: Instrument,
    I2: Instrument,
    for<'a> &'a I1: From<&'a TheJungle>,
    for<'a> &'a I2: From<&'a TheJungle>,
{
    let instrument_1: &I1 = jungle.into();
    let instrument_2: &I2 = jungle.into();
    let (first, second) = tokio::join!(instrument_1.play(note_1), instrument_2.play(note_2));
    map_playback_err(first)?;
    map_playback_err(second)
}

async fn play_three<I>(
    jungle: &TheJungle,
    note_1: Note<I::Articulation>,
    note_2: Note<I::Articulation>,
    note_3: Note<I::Articulation>,
) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    let (first, second, third) = tokio::join!(
        instrument.play(note_1),
        instrument.play(note_2),
        instrument.play(note_3)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)
}

async fn play_four<I>(
    jungle: &TheJungle,
    note_1: Note<I::Articulation>,
    note_2: Note<I::Articulation>,
    note_3: Note<I::Articulation>,
    note_4: Note<I::Articulation>,
) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    let (first, second, third, fourth) = tokio::join!(
        instrument.play(note_1),
        instrument.play(note_2),
        instrument.play(note_3),
        instrument.play(note_4)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)
}

async fn play_five<I>(
    jungle: &TheJungle,
    note_1: Note<I::Articulation>,
    note_2: Note<I::Articulation>,
    note_3: Note<I::Articulation>,
    note_4: Note<I::Articulation>,
    note_5: Note<I::Articulation>,
) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    let (first, second, third, fourth, fifth) = tokio::join!(
        instrument.play(note_1),
        instrument.play(note_2),
        instrument.play(note_3),
        instrument.play(note_4),
        instrument.play(note_5)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)?;
    map_playback_err(fifth)
}

async fn play_six<I>(
    jungle: &TheJungle,
    note_1: Note<I::Articulation>,
    note_2: Note<I::Articulation>,
    note_3: Note<I::Articulation>,
    note_4: Note<I::Articulation>,
    note_5: Note<I::Articulation>,
    note_6: Note<I::Articulation>,
) -> Result<(), String>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
{
    let instrument: &I = jungle.into();
    let (first, second, third, fourth, fifth, sixth) = tokio::join!(
        instrument.play(note_1),
        instrument.play(note_2),
        instrument.play(note_3),
        instrument.play(note_4),
        instrument.play(note_5),
        instrument.play(note_6)
    );
    map_playback_err(first)?;
    map_playback_err(second)?;
    map_playback_err(third)?;
    map_playback_err(fourth)?;
    map_playback_err(fifth)?;
    map_playback_err(sixth)
}
