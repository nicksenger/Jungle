use iced::Color;
use std::time::{Duration, Instant};

use crate::{ClusterKind, ClusterLive, Phase};

#[derive(Debug, Clone, Copy)]
pub struct Transition {
    from: Color,
    to: Color,
    started_at: Instant,
}

impl Transition {
    pub fn new(color: Color, now: Instant) -> Self {
        Self {
            from: color,
            to: color,
            started_at: now,
        }
    }

    pub fn update_target(&mut self, target: Color, now: Instant, duration: Duration) -> bool {
        if self.to == target {
            return false;
        }
        let current = self.sample(now, duration);
        self.from = current;
        self.to = target;
        self.started_at = now;
        true
    }

    pub fn sample(&self, now: Instant, duration: Duration) -> Color {
        if self.from == self.to {
            return self.to;
        }
        let t = ease_out_cubic(self.extent(now, duration));
        lerp_color(self.from, self.to, t)
    }

    pub fn extent(&self, now: Instant, duration: Duration) -> f32 {
        if duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn is_animating(&self, now: Instant, duration: Duration) -> bool {
        self.from != self.to && self.extent(now, duration) < 1.0
    }

    pub fn settle(&mut self, now: Instant, duration: Duration) -> bool {
        if !self.is_animating(now, duration) && self.from != self.to {
            self.from = self.to;
            return true;
        }
        false
    }
}

pub fn target_color(kind: ClusterKind, phase: Phase<ClusterLive>) -> Color {
    let alpha = match phase {
        Phase::Static => kind_pending_alpha(kind),
        Phase::Live(live) => {
            if live.has_failed {
                kind_failed_alpha(kind)
            } else if live.has_running {
                kind_running_alpha(kind)
            } else if live.has_completed {
                kind_completed_alpha(kind)
            } else {
                kind_pending_alpha(kind)
            }
        }
    };
    Color::from_rgba8(20, 46, 30, alpha)
}

fn kind_pending_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.14,
        ClusterKind::Transparent => 0.08,
    }
}

fn kind_running_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.24,
        ClusterKind::Transparent => 0.16,
    }
}

fn kind_completed_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.19,
        ClusterKind::Transparent => 0.12,
    }
}

fn kind_failed_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.26,
        ClusterKind::Transparent => 0.18,
    }
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
