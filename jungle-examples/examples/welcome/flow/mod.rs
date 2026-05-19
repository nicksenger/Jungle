use std::time::Duration;

use tokio::{
    sync::broadcast,
    time::{Instant, MissedTickBehavior},
};

const BROADCAST_CAPACITY: usize = 64;

#[derive(Clone)]
pub struct Metronome {
    started_at: Instant,
    tick: Duration,
    tick_tx: broadcast::Sender<Instant>,
}

impl Metronome {
    pub fn spawn(tick: Duration) -> Self {
        let (tick_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let metronome = Self {
            started_at: Instant::now(),
            tick,
            tick_tx,
        };
        metronome.start_task();
        metronome
    }

    pub fn subscribe(&self) -> MetronomeSync {
        MetronomeSync {
            started_at: self.started_at,
            tick: self.tick,
            tick_rx: self.tick_tx.subscribe(),
        }
    }

    fn start_task(&self) {
        let tick = self.tick;
        let tick_tx = self.tick_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let _ = tick_tx.send(Instant::now());
            }
        });
    }
}

pub struct MetronomeSync {
    started_at: Instant,
    tick: Duration,
    tick_rx: broadcast::Receiver<Instant>,
}

impl MetronomeSync {
    pub async fn synchronize(&mut self, target_offset: Duration) -> Duration {
        if target_offset.is_zero() {
            return Duration::ZERO;
        }

        loop {
            let elapsed = self.started_at.elapsed();
            if elapsed >= target_offset {
                return Duration::ZERO;
            }

            let remaining = target_offset - elapsed;
            if remaining <= self.tick {
                return remaining;
            }

            match self.tick_rx.recv().await {
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return remaining,
            }
        }
    }
}
