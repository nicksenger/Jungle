use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    amplitude_gain,
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, sine, smoothstep, triangle, SAMPLE_RATE,
    },
    Error, Instrument, Note,
};

pub struct SnareDrum {
    audio: AudioHandle,
}

impl SnareDrum {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SnareDrumArticulation {
    /// A standard, clean strike to the center of the drum head.
    CenterHit,
    /// Striking the center of the head and the metal rim simultaneously.
    /// This is the primary articulation for the massive verse/chorus backbeats.
    Rimshot,
    /// Laying the stick across the head and striking the rim for a woody click.
    /// Useful for low-energy dynamic drops.
    Sidestick,
    /// A very soft, low-velocity hit. Adler uses these to fill the space between backbeats.
    GhostNote,
    /// Two rapid, almost overlapping strikes (one hand trailing the other) to add weight.
    Flam,
}

impl Instrument for SnareDrum {
    type Articulation = SnareDrumArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, mut gain, mut playback_rate) = {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_snare_drum(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        };

        let velocity = note.velocity.clamp(0.0, 1.0);
        gain *= 0.88 + velocity * 0.52;
        playback_rate *= 0.98 + velocity * 0.06;

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.08 + (velocity - 0.5) * 0.06;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}

fn resolve_articulation(note: &Note<SnareDrumArticulation>) -> SnareDrumArticulation {
    if !matches!(note.articulation, SnareDrumArticulation::CenterHit) {
        return note.articulation;
    }

    let velocity = note.velocity.clamp(0.0, 1.0);
    let duration_ms = note.duration.as_secs_f32() * 1_000.0;

    if duration_ms <= 68.0 && velocity >= 0.27 {
        return SnareDrumArticulation::Flam;
    }
    if velocity >= 0.27 {
        return SnareDrumArticulation::Rimshot;
    }
    if velocity <= 0.245 {
        return SnareDrumArticulation::GhostNote;
    }
    SnareDrumArticulation::CenterHit
}

fn synthesize_snare_drum(note: &Note<SnareDrumArticulation>) -> (Arc<[f32]>, f32, f32) {
    let articulation = resolve_articulation(note);
    let duration = articulation_duration(note.duration, articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let body_hz = midi_to_hz(note.n_midi).clamp(145.0, 262.0);
    let velocity = note.velocity.clamp(0.0, 1.0).powf(0.55);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(articulation, body_hz, phase, t, velocity);
        let env = articulation_envelope(articulation, phase, velocity);
        pcm.push((sample * env).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: SnareDrumArticulation) -> Duration {
    let scale = match articulation {
        SnareDrumArticulation::CenterHit => 0.36,
        SnareDrumArticulation::Rimshot => 0.4,
        SnareDrumArticulation::Sidestick => 0.24,
        SnareDrumArticulation::GhostNote => 0.28,
        SnareDrumArticulation::Flam => 0.42,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.035))
}

fn articulation_output_shape(articulation: SnareDrumArticulation) -> (f32, f32) {
    match articulation {
        SnareDrumArticulation::CenterHit => (1.08, 1.0),
        SnareDrumArticulation::Rimshot => (1.22, 1.0),
        SnareDrumArticulation::Sidestick => (0.76, 1.0),
        SnareDrumArticulation::GhostNote => (0.66, 1.0),
        SnareDrumArticulation::Flam => (1.15, 0.995),
    }
}

fn articulation_sample(
    articulation: SnareDrumArticulation,
    body_hz: f32,
    phase: f32,
    t: f32,
    velocity: f32,
) -> f32 {
    let pitch_env = 1.0 - smoothstep((phase * 7.0).clamp(0.0, 1.0));
    let body_fund = sine(body_hz * (1.0 + pitch_env * 0.18), t) * 0.58;
    let body_ring = sine(body_hz * 2.05, t) * (-phase * 7.8).exp() * 0.3;
    let shell = triangle(body_hz * 3.15, t) * (-phase * 8.8).exp() * 0.14;

    let crack_tone =
        sine(1_860.0 + pitch_env * 1_050.0, t) * (1.0 - smoothstep((phase * 24.0).clamp(0.0, 1.0)));
    let stick_noise = hash_noise(t * 22_800.0) * (1.0 - smoothstep((phase * 28.0).clamp(0.0, 1.0)));

    let wire_white = hash_noise(t * 15_600.0);
    let wire_dark = hash_noise(t * 6_300.0);
    let wire = (wire_white - wire_dark * 0.58)
        * (0.22 + 0.78 * (1.0 - smoothstep((phase * 3.3).clamp(0.0, 1.0))));

    let base = body_fund + body_ring + shell;
    let attack = (stick_noise * 0.92 + crack_tone * 0.72) * (0.6 + velocity * 0.55);

    match articulation {
        SnareDrumArticulation::CenterHit => (base + attack * 0.46 + wire * 0.78).tanh(),
        SnareDrumArticulation::Rimshot => {
            let rim = triangle(2_450.0 + velocity * 280.0, t) * 0.24;
            (base * 1.05 + attack * 0.68 + wire * 0.86 + rim).tanh()
        }
        SnareDrumArticulation::Sidestick => {
            let wood = triangle(1_380.0, t) * 0.4 + triangle(2_180.0, t) * 0.22;
            let click = stick_noise * 0.48 + crack_tone * 0.16;
            (wood + click + wire * 0.18).tanh()
        }
        SnareDrumArticulation::GhostNote => (base * 0.58 + attack * 0.22 + wire * 0.5).tanh(),
        SnareDrumArticulation::Flam => {
            let lag_t = (t - 0.009).max(0.0);
            let lag_phase = ((phase - 0.045) * 7.4).max(0.0);
            let lag_attack = (hash_noise(lag_t * 21_200.0) * 0.62 + sine(1_650.0, lag_t) * 0.38)
                * (1.0 - smoothstep(lag_phase.clamp(0.0, 1.0)));
            (base + attack * 0.54 + lag_attack + wire * 0.84).tanh()
        }
    }
}

fn articulation_envelope(articulation: SnareDrumArticulation, phase: f32, velocity: f32) -> f32 {
    let attack = 0.0012;
    let (body_decay, wire_decay) = match articulation {
        SnareDrumArticulation::CenterHit => (1.35 - velocity * 0.16, 0.9),
        SnareDrumArticulation::Rimshot => (1.15 - velocity * 0.12, 0.78),
        SnareDrumArticulation::Sidestick => (1.9, 1.45),
        SnareDrumArticulation::GhostNote => (1.78, 1.2),
        SnareDrumArticulation::Flam => (1.08, 0.72),
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let body = (-phase * body_decay * 5.2).exp();
    let wire = (-phase * wire_decay * 9.8).exp();
    (attack_env * (body * 0.62 + wire * 0.38)).clamp(0.0, 1.0)
}
