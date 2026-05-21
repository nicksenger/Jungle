use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::time::{Instant, MissedTickBehavior};

#[derive(Debug, Clone, Copy)]
pub struct BeatEvent {
    pub timestamp: Instant,
}

#[derive(Clone)]
pub struct Metronome {
    started_at: Instant,
    beat: Duration,
    latest_beat: Arc<RwLock<Option<BeatEvent>>>,
}

impl Metronome {
    pub fn spawn(bpm: f32) -> Self {
        let beat = beat_duration(bpm);
        let latest_beat = Arc::new(RwLock::new(None));
        let metronome = Self {
            started_at: Instant::now(),
            beat,
            latest_beat,
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
