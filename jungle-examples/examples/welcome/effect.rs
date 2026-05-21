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
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_5: u8,
    const NOTE_6: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Pentad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_5: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Tetrad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_4: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Triad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Dyad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

pub struct Monad<
    I: Instrument<Articulation = A>,
    A: Copy,
    const NOTE: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>(PhantomData<(I, A)>);

#[effect(id = 506)]
impl<
        I,
        A,
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_5: u8,
        const NOTE_6: u8,
        const NOTE_TICK: u8,
        const REST_TICK: u8,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Hexad<I, A, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5, NOTE_6, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_5: u8,
        const NOTE_TICK: u8,
        const REST_TICK: u8,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Pentad<I, A, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_5, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_4: u8,
        const NOTE_TICK: u8,
        const REST_TICK: u8,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Tetrad<I, A, NOTE_1, NOTE_2, NOTE_3, NOTE_4, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_TICK: u8,
        const REST_TICK: u8,
    > jungle_sdk::prelude::Effect<TheJungle>
    for Triad<I, A, NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
impl<I, A, const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8>
    jungle_sdk::prelude::Effect<TheJungle> for Dyad<I, A, NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
impl<I, A, const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>
    jungle_sdk::prelude::Effect<TheJungle> for Monad<I, A, NOTE, NOTE_TICK, REST_TICK>
where
    I: Instrument<Articulation = A>,
    for<'a> &'a I: From<&'a TheJungle>,
    A: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = A;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = jungle.metronome().rhythm_timing(
            TICKS_PER_BEAT,
            NOTE_TICK,
            REST_TICK,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
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
