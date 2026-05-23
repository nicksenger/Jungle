use std::{f32::consts::TAU, sync::Arc, time::Duration};

use crate::audio::PlayRequest;
use crate::instrumentation::{
    amplitude_gain,
    synthesis::{duration_to_frames, hash_noise, midi_to_hz, saw, sine, smoothstep, SAMPLE_RATE},
    Error, Expression, Note,
};

use super::VocalsArticulation;

pub(super) async fn play(
    audio: &crate::audio::AudioHandle,
    note: Note<VocalsArticulation>,
) -> Result<(), Error> {
    let (pcm, gain, playback_rate) = {
        let note_for_synth = note;
        tokio::task::spawn_blocking(move || synthesize_vocals(&note_for_synth))
            .await
            .map_err(|_| Error::Playback)?
    };

    for layer in articulation_layers(note.articulation) {
        if layer.delay_seconds > 0.0 {
            tokio::time::sleep(Duration::from_secs_f32(layer.delay_seconds)).await;
        }
        let mut request = PlayRequest::new(Arc::clone(&pcm), 1, SAMPLE_RATE);
        request.gain = gain * layer.gain_scale * amplitude_gain(&note);
        request.playback_rate = playback_rate * layer.playback_rate_scale;
        request.pan = layer.pan;
        audio.play(request).await.map_err(|_| Error::Submission)?;
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct PlaybackLayer {
    pan: f32,
    gain_scale: f32,
    playback_rate_scale: f32,
    delay_seconds: f32,
}

fn articulation_layers(articulation: VocalsArticulation) -> &'static [PlaybackLayer] {
    match articulation {
        VocalsArticulation::Clean => &[
            PlaybackLayer {
                pan: 0.02,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.17,
                gain_scale: 0.26,
                playback_rate_scale: 0.998,
                delay_seconds: 0.008,
            },
            PlaybackLayer {
                pan: 0.21,
                gain_scale: 0.21,
                playback_rate_scale: 1.004,
                delay_seconds: 0.015,
            },
        ],
        VocalsArticulation::GritRasp => &[
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.12,
                gain_scale: 0.48,
                playback_rate_scale: 0.992,
                delay_seconds: 0.009,
            },
            PlaybackLayer {
                pan: 0.16,
                gain_scale: 0.38,
                playback_rate_scale: 1.01,
                delay_seconds: 0.015,
            },
        ],
        VocalsArticulation::SpokenBreakdown => &[
            PlaybackLayer {
                pan: -0.02,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.14,
                gain_scale: 0.26,
                playback_rate_scale: 1.008,
                delay_seconds: 0.018,
            },
        ],
        VocalsArticulation::SirenScream => &[
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.09,
                gain_scale: 0.31,
                playback_rate_scale: 0.994,
                delay_seconds: 0.012,
            },
            PlaybackLayer {
                pan: 0.11,
                gain_scale: 0.29,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
        VocalsArticulation::StutterStab => &[
            PlaybackLayer {
                pan: 0.03,
                gain_scale: 1.0,
                playback_rate_scale: 1.0,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.14,
                gain_scale: 0.24,
                playback_rate_scale: 0.99,
                delay_seconds: 0.008,
            },
        ],
        VocalsArticulation::GroupHarmony => &[
            PlaybackLayer {
                pan: -0.42,
                gain_scale: 0.7,
                playback_rate_scale: 0.992,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: 0.0,
                gain_scale: 0.9,
                playback_rate_scale: 1.0,
                delay_seconds: 0.011,
            },
            PlaybackLayer {
                pan: 0.38,
                gain_scale: 0.68,
                playback_rate_scale: 1.008,
                delay_seconds: 0.019,
            },
        ],
        VocalsArticulation::ShoutResponse => &[
            PlaybackLayer {
                pan: -0.58,
                gain_scale: 0.68,
                playback_rate_scale: 0.985,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.1,
                gain_scale: 0.82,
                playback_rate_scale: 1.0,
                delay_seconds: 0.007,
            },
            PlaybackLayer {
                pan: 0.24,
                gain_scale: 0.78,
                playback_rate_scale: 1.013,
                delay_seconds: 0.014,
            },
            PlaybackLayer {
                pan: 0.62,
                gain_scale: 0.63,
                playback_rate_scale: 1.02,
                delay_seconds: 0.021,
            },
        ],
        VocalsArticulation::VocalBed => &[
            PlaybackLayer {
                pan: -0.65,
                gain_scale: 0.58,
                playback_rate_scale: 0.994,
                delay_seconds: 0.0,
            },
            PlaybackLayer {
                pan: -0.24,
                gain_scale: 0.66,
                playback_rate_scale: 0.999,
                delay_seconds: 0.026,
            },
            PlaybackLayer {
                pan: 0.22,
                gain_scale: 0.66,
                playback_rate_scale: 1.005,
                delay_seconds: 0.033,
            },
            PlaybackLayer {
                pan: 0.64,
                gain_scale: 0.57,
                playback_rate_scale: 1.012,
                delay_seconds: 0.041,
            },
        ],
    }
}

fn synthesize_vocals(note: &Note<VocalsArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let base_hz = midi_to_hz(note.n_midi).clamp(85.0, 1_300.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let expression = note.expression.unwrap_or(Expression {
        bend: 0.0,
        vibrato: 0.0,
    });

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(note.articulation, base_hz, phase, t, expression);
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: VocalsArticulation) -> Duration {
    let scale = match articulation {
        VocalsArticulation::Clean => 1.05,
        VocalsArticulation::GritRasp => 1.0,
        VocalsArticulation::SpokenBreakdown => 0.8,
        VocalsArticulation::SirenScream => 1.1,
        VocalsArticulation::StutterStab => 0.28,
        VocalsArticulation::GroupHarmony => 1.0,
        VocalsArticulation::ShoutResponse => 0.72,
        VocalsArticulation::VocalBed => 1.35,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: VocalsArticulation) -> (f32, f32) {
    match articulation {
        VocalsArticulation::Clean => (0.83, 1.0),
        VocalsArticulation::GritRasp => (0.95, 1.0),
        VocalsArticulation::SpokenBreakdown => (0.84, 1.0),
        VocalsArticulation::SirenScream => (1.0, 1.03),
        VocalsArticulation::StutterStab => (0.86, 1.0),
        VocalsArticulation::GroupHarmony => (0.78, 1.0),
        VocalsArticulation::ShoutResponse => (0.85, 1.0),
        VocalsArticulation::VocalBed => (0.72, 1.0),
    }
}

fn articulation_sample(
    articulation: VocalsArticulation,
    base_hz: f32,
    phase: f32,
    t: f32,
    expression: Expression,
) -> f32 {
    let vibrato = (TAU * 5.7 * t).sin() * expression.vibrato.clamp(-1.0, 1.0) * 0.02;
    let bend = expression.bend.clamp(-1.0, 1.0) * 0.15;

    match articulation {
        VocalsArticulation::Clean => {
            let f = base_hz * (1.0 + bend + vibrato);
            reed_formant(f, t, phase)
        }
        VocalsArticulation::GritRasp => {
            let f = base_hz * (1.0 + bend * 0.5 + vibrato * 0.45);
            let body = vocal_formant(f, t, 0.22);
            let grit = hash_noise(t * 7_000.0) * (0.25 + 0.2 * (TAU * 18.0 * t).sin().abs());
            (body + grit).tanh()
        }
        VocalsArticulation::SpokenBreakdown => {
            let f = base_hz * 0.8 * (1.0 + bend * 0.2);
            sine(f, t) * 0.55 + saw(f * 2.0, t) * 0.12 + hash_noise(t * 2_000.0) * 0.06
        }
        VocalsArticulation::SirenScream => {
            let rise = smoothstep(phase) * 0.35;
            let f = base_hz * (1.45 + rise) * (1.0 + vibrato * 1.6);
            let body = sine(f, t) * 0.62 + sine(f * 2.0, t) * 0.23;
            let air = hash_noise(t * 9_500.0) * 0.08;
            (body + air).tanh()
        }
        VocalsArticulation::StutterStab => {
            let f = base_hz * (1.0 + bend * 0.2);
            let body = vocal_formant(f, t, 0.18);
            let gate = ((phase * 12.0).fract() < 0.42) as i32 as f32;
            body * gate
        }
        VocalsArticulation::GroupHarmony => {
            let f = base_hz.clamp(90.0, 880.0);
            let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
            unison + sine(f * 2.0, t) * 0.15 + hash_noise(t * 4_500.0) * 0.08
        }
        VocalsArticulation::ShoutResponse => {
            let f = base_hz.clamp(90.0, 880.0);
            let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
            let punch = 1.0 - smoothstep(phase * 3.4);
            (unison * 1.1 + hash_noise(t * 8_500.0) * 0.22 * punch).tanh()
        }
        VocalsArticulation::VocalBed => {
            let f = base_hz.clamp(90.0, 880.0);
            let unison = sine(f * 0.995, t) * 0.42 + sine(f, t) * 0.3 + sine(f * 1.007, t) * 0.28;
            let wide = sine(f * 0.5, t) * 0.2 + sine(f * 1.5, t) * 0.24;
            unison * 0.72 + wide + hash_noise(t * 2_500.0) * 0.06
        }
    }
}

fn reed_formant(frequency_hz: f32, t: f32, phase: f32) -> f32 {
    let mouthpiece = smoothstep((phase / 0.12).clamp(0.0, 1.0));
    let body = sine(frequency_hz, t) * 0.62 + saw(frequency_hz, t) * 0.17;
    let warmth = sine(frequency_hz * 2.0, t) * 0.18 + sine(frequency_hz * 3.0, t) * 0.08;
    let breath = hash_noise(t * 6_100.0) * (0.03 + 0.03 * (1.0 - mouthpiece));
    (body + warmth + breath).tanh()
}

fn vocal_formant(frequency_hz: f32, t: f32, airy: f32) -> f32 {
    let base = sine(frequency_hz, t) * 0.55;
    let second = sine(frequency_hz * 2.1, t) * 0.22;
    let third = sine(frequency_hz * 3.2, t) * 0.15;
    let breath = hash_noise(t * 5_000.0) * airy;
    base + second + third + breath
}

fn articulation_envelope(articulation: VocalsArticulation, phase: f32) -> f32 {
    if matches!(
        articulation,
        VocalsArticulation::GroupHarmony
            | VocalsArticulation::ShoutResponse
            | VocalsArticulation::VocalBed
    ) {
        return backup_vocal_envelope(articulation, phase);
    }

    let attack = match articulation {
        VocalsArticulation::StutterStab => 0.01,
        VocalsArticulation::SirenScream => 0.06,
        _ => 0.03,
    };
    let body = match articulation {
        VocalsArticulation::Clean => 0.9,
        VocalsArticulation::GritRasp => 0.94,
        VocalsArticulation::SpokenBreakdown => 0.82,
        VocalsArticulation::SirenScream => 0.97,
        VocalsArticulation::StutterStab => 0.74,
        VocalsArticulation::GroupHarmony
        | VocalsArticulation::ShoutResponse
        | VocalsArticulation::VocalBed => unreachable!(),
    };
    let release_start = match articulation {
        VocalsArticulation::SpokenBreakdown => 0.66,
        VocalsArticulation::StutterStab => 0.48,
        VocalsArticulation::SirenScream => 0.84,
        _ => 0.8,
    };
    let release: f32 = match articulation {
        VocalsArticulation::Clean => 0.18,
        VocalsArticulation::GritRasp => 0.15,
        VocalsArticulation::SpokenBreakdown => 0.22,
        VocalsArticulation::SirenScream => 0.2,
        VocalsArticulation::StutterStab => 0.1,
        VocalsArticulation::GroupHarmony
        | VocalsArticulation::ShoutResponse
        | VocalsArticulation::VocalBed => unreachable!(),
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let release_phase = ((phase - release_start) / release.max(1e-3_f32)).clamp(0.0, 1.0);
    let release_env = 1.0 - smoothstep(release_phase);
    (attack_env * body * release_env).clamp(0.0, 1.0)
}

fn backup_vocal_envelope(articulation: VocalsArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        VocalsArticulation::ShoutResponse => 0.015,
        VocalsArticulation::VocalBed => 0.08,
        VocalsArticulation::GroupHarmony => 0.04,
        _ => unreachable!(),
    };
    let decay = match articulation {
        VocalsArticulation::GroupHarmony => 0.45,
        VocalsArticulation::ShoutResponse => 0.8,
        VocalsArticulation::VocalBed => 0.32,
        _ => unreachable!(),
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
