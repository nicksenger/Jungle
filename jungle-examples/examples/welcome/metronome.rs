use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::time::{Instant, MissedTickBehavior};

#[derive(Debug, Clone, Copy)]
pub struct BeatEvent {
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy)]
pub struct RhythmTiming {
    note_duration: Duration,
    should_play: bool,
    pre_play_sleep_duration: Duration,
    post_cycle_sleep_duration: Duration,
}

impl RhythmTiming {
    pub fn note_duration(&self) -> Duration {
        self.note_duration
    }

    pub fn should_play(&self) -> bool {
        self.should_play
    }

    pub async fn sleep_until_note_window(&self) {
        if !self.pre_play_sleep_duration.is_zero() {
            tokio::time::sleep(self.pre_play_sleep_duration).await;
        }
    }

    pub async fn sleep_until_next_cycle(&self) {
        if !self.post_cycle_sleep_duration.is_zero() {
            tokio::time::sleep(self.post_cycle_sleep_duration).await;
        }
    }
}

#[derive(Clone)]
pub struct Metronome {
    started_at: Instant,
    beat: Duration,
    latest_beat: Arc<RwLock<Option<BeatEvent>>>,
    lane_ticks: Arc<RwLock<HashMap<u32, u64>>>,
}

impl Metronome {
    pub fn spawn(bpm: f32) -> Self {
        let beat = beat_duration(bpm);
        let latest_beat = Arc::new(RwLock::new(None));
        let metronome = Self {
            started_at: Instant::now(),
            beat,
            latest_beat,
            lane_ticks: Arc::new(RwLock::new(HashMap::new())),
        };
        metronome.start_task();
        metronome
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    pub fn beat_duration(&self) -> Duration {
        self.beat
    }

    pub fn latest_beat(&self) -> Option<BeatEvent> {
        let latest_beat = self
            .latest_beat
            .read()
            .expect("latest beat rwlock should not be poisoned");
        *latest_beat
    }

    pub fn elapsed(&self) -> Duration {
        let now_elapsed = self.started_at.elapsed();
        let beat_elapsed = self
            .latest_beat()
            .map(|event| event.timestamp.saturating_duration_since(self.started_at))
            .unwrap_or(Duration::ZERO);
        now_elapsed.max(beat_elapsed)
    }

    pub fn tick_duration(&self, ticks_per_beat: u32) -> Duration {
        self.beat.div_f32(ticks_per_beat as f32)
    }

    pub fn duration_for_ticks(&self, ticks_per_beat: u32, ticks: u32) -> Duration {
        self.tick_duration(ticks_per_beat).mul_f32(ticks as f32)
    }

    pub fn phase_offset(&self, period: Duration) -> Duration {
        if period.is_zero() {
            return Duration::ZERO;
        }

        let now = Instant::now();
        let anchor = self
            .latest_beat()
            .map(|event| event.timestamp)
            .unwrap_or_else(|| self.started_at());
        let elapsed = now.saturating_duration_since(anchor);
        duration_mod(elapsed, period)
    }

    pub fn late_note_drop_threshold(&self, min: Duration, max: Duration) -> Duration {
        self.beat_duration().div_f32(4.0).clamp(min, max)
    }

    pub fn rhythm_timing(
        &self,
        lane_id: u32,
        ticks_per_beat: u32,
        note_ticks: u32,
        rest_ticks: u32,
        min_late_note_drop_threshold: Duration,
        max_late_note_drop_threshold: Duration,
    ) -> RhythmTiming {
        let note_duration = self.duration_for_ticks(ticks_per_beat, note_ticks);
        let rest_duration = self.duration_for_ticks(ticks_per_beat, rest_ticks);
        let lane_target_start = {
            let mut lane_ticks = self
                .lane_ticks
                .write()
                .expect("lane ticks rwlock should not be poisoned");
            let scheduled_tick = lane_ticks.entry(lane_id).or_insert(0);
            let scheduled_start_offset = self
                .tick_duration(ticks_per_beat)
                .mul_f64(*scheduled_tick as f64);
            *scheduled_tick = scheduled_tick.saturating_add(rest_ticks as u64);
            self.started_at + scheduled_start_offset
        };
        let now = Instant::now();
        let lateness = now.saturating_duration_since(lane_target_start);
        let late_note_drop_threshold = self
            .late_note_drop_threshold(min_late_note_drop_threshold, max_late_note_drop_threshold);
        let should_play = lateness <= late_note_drop_threshold;
        let pre_play_sleep_duration = lane_target_start.saturating_duration_since(now);
        let cycle_end = lane_target_start + rest_duration;
        let post_cycle_anchor = now + pre_play_sleep_duration + note_duration;
        let post_cycle_sleep_duration = cycle_end.saturating_duration_since(post_cycle_anchor);
        RhythmTiming {
            note_duration,
            should_play,
            pre_play_sleep_duration,
            post_cycle_sleep_duration,
        }
    }

    fn start_task(&self) {
        let beat = self.beat;
        let latest_beat = self.latest_beat.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(beat);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let event = BeatEvent {
                    timestamp: Instant::now(),
                };
                let mut latest_beat_lock = latest_beat
                    .write()
                    .expect("latest beat rwlock should not be poisoned");
                *latest_beat_lock = Some(event);
            }
        });
    }
}

fn beat_duration(bpm: f32) -> Duration {
    let sanitized_bpm = if bpm.is_finite() && bpm > 0.0 {
        bpm
    } else {
        123.0
    };
    Duration::from_secs_f32(60.0 / sanitized_bpm)
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
