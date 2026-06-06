use iced::Color;

use crate::{ClusterKind, ClusterLive, Phase};

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
    Color::from_rgba8(20, 46, 30, alpha.clamp(0.0, 1.0))
}

fn kind_pending_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.10,
        ClusterKind::Join => 0.10,
        ClusterKind::Transparent => 0.10,
    }
}

fn kind_running_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.10,
        ClusterKind::Join => 0.10,
        ClusterKind::Transparent => 0.10,
    }
}

fn kind_completed_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.10,
        ClusterKind::Join => 0.10,
        ClusterKind::Transparent => 0.10,
    }
}

fn kind_failed_alpha(kind: ClusterKind) -> f32 {
    match kind {
        ClusterKind::While => 0.10,
        ClusterKind::Join => 0.10,
        ClusterKind::Transparent => 0.10,
    }
}
