use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::{
    sync::broadcast,
    time::{Instant, MissedTickBehavior},
};

const BROADCAST_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct BeatEvent {
    pub timestamp: Instant,
    pub bar: u32,
    pub beat: u32,
}

#[derive(Clone)]
pub struct Metronome {
    started_at: Instant,
    beat: Duration,
    beats_per_bar: u32,
    beat_tx: broadcast::Sender<BeatEvent>,
    latest_beat: Arc<RwLock<Option<BeatEvent>>>,
}

impl Metronome {
    pub fn spawn(bpm: f32, beats_per_bar: u32) -> Self {
        let (beat_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let beat = beat_duration(bpm);
        let latest_beat = Arc::new(RwLock::new(None));
        let metronome = Self {
            started_at: Instant::now(),
            beat,
            beats_per_bar: beats_per_bar.max(1),
            beat_tx,
            latest_beat,
        };
        metronome.start_task();
        metronome
    }

    pub fn subscribe(&self) -> MetronomeSync {
        MetronomeSync {
            started_at: self.started_at,
            beat: self.beat,
            beat_rx: self.beat_tx.subscribe(),
            last_beat_timestamp: None,
            last_bar: 0,
            last_beat_in_bar: 0,
        }
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
        let beats_per_bar = self.beats_per_bar;
        let beat_tx = self.beat_tx.clone();
        let latest_beat = self.latest_beat.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(beat);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut beat_index: u64 = 0;
            loop {
                interval.tick().await;
                let beat_in_bar = ((beat_index % beats_per_bar as u64) + 1) as u32;
                let bar = (beat_index / beats_per_bar as u64 + 1) as u32;
                beat_index = beat_index.saturating_add(1);
                let event = BeatEvent {
                    timestamp: Instant::now(),
                    bar,
                    beat: beat_in_bar,
                };
                {
                    let mut latest_beat_lock = latest_beat
                        .write()
                        .expect("latest beat rwlock should not be poisoned");
                    *latest_beat_lock = Some(event);
                }
                let _ = beat_tx.send(event);
            }
        });
    }
}

pub struct MetronomeSync {
    started_at: Instant,
    beat: Duration,
    beat_rx: broadcast::Receiver<BeatEvent>,
    last_beat_timestamp: Option<Instant>,
    last_bar: u32,
    last_beat_in_bar: u32,
}

impl MetronomeSync {
    pub fn beat_duration(&self) -> Duration {
        self.beat
    }

    pub fn target_instant(&self, target_offset: Duration) -> Instant {
        self.started_at + target_offset
    }

    pub fn elapsed(&mut self) -> Duration {
        self.refresh_latest_beat();
        let now_elapsed = self.started_at.elapsed();
        let beat_elapsed = self
            .last_beat_timestamp
            .map(|timestamp| timestamp.saturating_duration_since(self.started_at))
            .unwrap_or(Duration::ZERO);
        now_elapsed.max(beat_elapsed)
    }

    fn refresh_latest_beat(&mut self) {
        loop {
            match self.beat_rx.try_recv() {
                Ok(event) => {
                    self.last_beat_timestamp = Some(event.timestamp);
                    self.last_bar = event.bar;
                    self.last_beat_in_bar = event.beat;
                }
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
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
