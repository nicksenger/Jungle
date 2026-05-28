use jungle_sdk::effect;
use jungle_sdk::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

use crate::ecosystem::TheJungle;
use crate::instrumentation::{Instrument, Note};

const TICKS_PER_BEAT: u32 = 384;
const MIN_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(120);
const RHYTHM_PAN: f32 = 0.5;
const RHYTHM_VELOCITY: f32 = 37.0 / 127.0;
const EFFECT_CYCLE_LOG_INTERVAL: usize = 512;
const EFFECT_SLOW_CYCLE_WARN_THRESHOLD: Duration = Duration::from_millis(150);
const EFFECT_SLEEP_OVERSHOOT_WARN_THRESHOLD: Duration = Duration::from_millis(40);
const EFFECT_WAKE_DRIFT_WARN_THRESHOLD: Duration = Duration::from_millis(200);

static EFFECT_CYCLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static EFFECT_SKIPPED_NOTES: AtomicUsize = AtomicUsize::new(0);

pub struct Monad<
    I: Instrument,
    const LANE_ID: u8,
    const NOTE: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
>(PhantomData<I>);

pub struct Passthrough<T>(PhantomData<T>);

pub struct Rest<const LANE_ID: u8, const REST_TICKS: u32>;
#[effect(id = 2)]
impl<T> Effect<TheJungle> for Passthrough<T>
where
    T: Serialize + DeserializeOwned + Send + 'static,
{
    type In = T;
    type Out = T;
    type Err = String;

    async fn effect(_jungle: &TheJungle, input: Self::In) -> Result<Self::Out, Self::Err> {
        Ok(input)
    }
}

#[effect(id = 1)]
impl<const LANE_ID: u8, const REST_TICKS: u32> Effect<TheJungle> for Rest<LANE_ID, REST_TICKS> {
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, _input: Self::In) -> Result<Self::Out, Self::Err> {
        let cycle_started_at = Instant::now();
        jungle.metronome().wait_for_start_barrier().await;
        let lane_id = u32::from(LANE_ID);
        let timing = jungle.metronome().rhythm_timing(
            lane_id,
            TICKS_PER_BEAT,
            0,
            REST_TICKS,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        let pre_play_sleep_elapsed = measure_note_window_sleep(lane_id, &timing).await;
        let post_cycle_sleep_elapsed = measure_next_cycle_sleep(lane_id, &timing).await;
        log_effect_cycle(
            lane_id,
            false,
            pre_play_sleep_elapsed,
            Duration::ZERO,
            post_cycle_sleep_elapsed,
            cycle_started_at.elapsed(),
        );
        Ok(())
    }
}

#[effect(id = 0)]
impl<I, const LANE_ID: u8, const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32>
    Effect<TheJungle> for Monad<I, LANE_ID, NOTE, NOTE_TICK, REST_TICK>
where
    I: Instrument,
    for<'a> &'a I: From<&'a TheJungle>,
    I::Articulation: Copy + Serialize + DeserializeOwned + Send + 'static,
{
    type In = I::Articulation;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let cycle_started_at = Instant::now();
        jungle.metronome().wait_for_start_barrier().await;
        let lane_id = u32::from(LANE_ID);
        let timing = jungle.metronome().rhythm_timing(
            lane_id,
            TICKS_PER_BEAT,
            NOTE_TICK as u32,
            REST_TICK as u32,
            MIN_LATE_NOTE_DROP_THRESHOLD,
            MAX_LATE_NOTE_DROP_THRESHOLD,
        );
        let pre_play_sleep_elapsed = measure_note_window_sleep(lane_id, &timing).await;
        let mut play_elapsed = Duration::ZERO;
        if timing.should_play() {
            let [note] = rhythm_notes(
                jungle,
                lane_id,
                [NOTE],
                timing.note_duration(),
                articulation,
            );
            let play_started_at = Instant::now();
            play_one::<I>(jungle, note).await?;
            play_elapsed = play_started_at.elapsed();
        }
        let post_cycle_sleep_elapsed = measure_next_cycle_sleep(lane_id, &timing).await;
        log_effect_cycle(
            lane_id,
            timing.should_play(),
            pre_play_sleep_elapsed,
            play_elapsed,
            post_cycle_sleep_elapsed,
            cycle_started_at.elapsed(),
        );
        Ok(())
    }
}

fn rhythm_notes<const N: usize, A: Copy>(
    jungle: &TheJungle,
    lane_id: u32,
    midi_notes: [u8; N],
    duration: std::time::Duration,
    articulation: A,
) -> [Note<A>; N] {
    midi_notes.map(|n_midi| rhythm_note(jungle, lane_id, n_midi, duration, articulation))
}

fn rhythm_note<A: Copy>(
    jungle: &TheJungle,
    lane_id: u32,
    n_midi: u8,
    duration: std::time::Duration,
    articulation: A,
) -> Note<A> {
    Note {
        n_midi,
        amplitude_multiplier: jungle.animal_volume_for_lane(lane_id),
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

async fn measure_note_window_sleep(
    lane_id: u32,
    timing: &crate::metronome::RhythmTiming,
) -> Duration {
    let expected_sleep = timing.expected_pre_play_sleep_duration();
    let started_at = Instant::now();
    timing.sleep_until_note_window().await;
    let woke_at = Instant::now();
    let elapsed = woke_at.saturating_duration_since(started_at);
    let overshoot = elapsed.saturating_sub(expected_sleep);
    let note_window_target_at: Instant = timing.note_window_target_at().into();
    let drift_from_target = woke_at.saturating_duration_since(note_window_target_at);
    trace!(
        lane_id,
        expected_pre_play_sleep_ms = expected_sleep.as_millis(),
        pre_play_sleep_elapsed_ms = elapsed.as_millis(),
        pre_play_sleep_overshoot_ms = overshoot.as_millis(),
        note_window_drift_ms = drift_from_target.as_millis(),
        metronome_lateness_ms = timing.lateness().as_millis(),
        metronome_drop_threshold_ms = timing.late_note_drop_threshold().as_millis(),
        "effect note window sleep complete"
    );
    if overshoot > EFFECT_SLEEP_OVERSHOOT_WARN_THRESHOLD
        || drift_from_target > EFFECT_WAKE_DRIFT_WARN_THRESHOLD
    {
        warn!(
            lane_id,
            expected_pre_play_sleep_ms = expected_sleep.as_millis(),
            pre_play_sleep_elapsed_ms = elapsed.as_millis(),
            pre_play_sleep_overshoot_ms = overshoot.as_millis(),
            note_window_drift_ms = drift_from_target.as_millis(),
            metronome_lateness_ms = timing.lateness().as_millis(),
            metronome_drop_threshold_ms = timing.late_note_drop_threshold().as_millis(),
            "effect note window wake drift exceeded threshold"
        );
    }
    elapsed
}

async fn measure_next_cycle_sleep(
    lane_id: u32,
    timing: &crate::metronome::RhythmTiming,
) -> Duration {
    let expected_sleep = timing.expected_post_cycle_sleep_duration();
    let started_at = Instant::now();
    timing.sleep_until_next_cycle().await;
    let woke_at = Instant::now();
    let elapsed = woke_at.saturating_duration_since(started_at);
    let overshoot = elapsed.saturating_sub(expected_sleep);
    let cycle_end_target_at: Instant = timing.cycle_end_target_at().into();
    let drift_from_cycle_end = woke_at.saturating_duration_since(cycle_end_target_at);
    trace!(
        lane_id,
        expected_post_cycle_sleep_ms = expected_sleep.as_millis(),
        post_cycle_sleep_elapsed_ms = elapsed.as_millis(),
        post_cycle_sleep_overshoot_ms = overshoot.as_millis(),
        cycle_end_drift_ms = drift_from_cycle_end.as_millis(),
        "effect post-cycle sleep complete"
    );
    if overshoot > EFFECT_SLEEP_OVERSHOOT_WARN_THRESHOLD
        || drift_from_cycle_end > EFFECT_WAKE_DRIFT_WARN_THRESHOLD
    {
        warn!(
            lane_id,
            expected_post_cycle_sleep_ms = expected_sleep.as_millis(),
            post_cycle_sleep_elapsed_ms = elapsed.as_millis(),
            post_cycle_sleep_overshoot_ms = overshoot.as_millis(),
            cycle_end_drift_ms = drift_from_cycle_end.as_millis(),
            "effect post-cycle wake drift exceeded threshold"
        );
    }
    elapsed
}

fn log_effect_cycle(
    lane_id: u32,
    should_play: bool,
    pre_play_sleep_elapsed: Duration,
    play_elapsed: Duration,
    post_cycle_sleep_elapsed: Duration,
    cycle_elapsed: Duration,
) {
    let cycle_count = EFFECT_CYCLE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if !should_play {
        let skipped_total = EFFECT_SKIPPED_NOTES.fetch_add(1, Ordering::Relaxed) + 1;
        if skipped_total % EFFECT_CYCLE_LOG_INTERVAL == 0 {
            warn!(
                lane_id,
                cycle_count,
                skipped_total,
                cycle_elapsed_ms = cycle_elapsed.as_millis(),
                "effect skipped playback due to timing policy"
            );
        }
    }

    if cycle_elapsed > EFFECT_SLOW_CYCLE_WARN_THRESHOLD {
        warn!(
            lane_id,
            should_play,
            cycle_count,
            cycle_elapsed_ms = cycle_elapsed.as_millis(),
            pre_play_sleep_elapsed_ms = pre_play_sleep_elapsed.as_millis(),
            play_elapsed_ms = play_elapsed.as_millis(),
            post_cycle_sleep_elapsed_ms = post_cycle_sleep_elapsed.as_millis(),
            "slow effect cycle"
        );
    } else if cycle_count % EFFECT_CYCLE_LOG_INTERVAL == 0 {
        debug!(
            lane_id,
            should_play,
            cycle_count,
            cycle_elapsed_ms = cycle_elapsed.as_millis(),
            pre_play_sleep_elapsed_ms = pre_play_sleep_elapsed.as_millis(),
            play_elapsed_ms = play_elapsed.as_millis(),
            post_cycle_sleep_elapsed_ms = post_cycle_sleep_elapsed.as_millis(),
            skipped_total = EFFECT_SKIPPED_NOTES.load(Ordering::Relaxed),
            "effect cycle heartbeat"
        );
    }
}
