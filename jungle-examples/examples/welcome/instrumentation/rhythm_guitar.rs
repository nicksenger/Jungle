use std::{sync::Arc, time::Duration};

use crate::audio::{AudioHandle, PlayRequest};

use super::{
    synthesis::{
        duration_to_frames, hash_noise, midi_to_hz, saw, smoothstep, triangle, SAMPLE_RATE,
        SPAWN_BLOCKING_FRAME_THRESHOLD,
    },
    Error, Instrument, Note,
};

pub struct RhythmGuitar {
    audio: AudioHandle,
}

impl RhythmGuitar {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum RhythmGuitarArticulation {
    /// A standard, ringing open or barre chord.
    Sustained,
    /// Constant, tight palm-muting to drive the verses.
    PalmMuted,
    /// Lifting the fretting hand immediately after striking to choke the chord.
    /// Crucial for the staccato, funky stabs in the verse groove.
    Choked,
    /// Striking strings completely muted by the left hand.
    /// Used heavily during the scratchy intro buildup before the full band kicks in.
    RhythmicScratch,
    /// Sliding an entire chord shape up or down the neck.
    ChordSlide,
}

impl Instrument for RhythmGuitar {
    type Articulation = RhythmGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = if should_spawn_blocking(&note) {
            let note_for_synth = note;
            tokio::task::spawn_blocking(move || synthesize_rhythm_guitar(&note_for_synth))
                .await
                .map_err(|_| Error::Playback)?
        } else {
            synthesize_rhythm_guitar(&note)
        };

        let mut request = PlayRequest::new(pcm, 1, SAMPLE_RATE);
        request.start_offset = note.offset;
        request.gain = gain;
        request.playback_rate = playback_rate;
        request.pan = -0.25;
        self.audio.try_play(request).map_err(|_| Error::Submission)
    }
}

fn should_spawn_blocking(note: &Note<RhythmGuitarArticulation>) -> bool {
    let duration = articulation_duration(note.duration, note.articulation);
    duration_to_frames(duration, SAMPLE_RATE) >= SPAWN_BLOCKING_FRAME_THRESHOLD
}

fn synthesize_rhythm_guitar(note: &Note<RhythmGuitarArticulation>) -> (Arc<[f32]>, f32, f32) {
    let duration = articulation_duration(note.duration, note.articulation);
    let frame_count = duration_to_frames(duration, SAMPLE_RATE).max(1);
    let root_hz = midi_to_hz(note.n_midi).max(55.0);
    let velocity = note.velocity.clamp(0.0, 1.0);
    let minor_third_hz = root_hz * 2.0_f32.powf(3.0 / 12.0);
    let fifth_hz = root_hz * 2.0_f32.powf(7.0 / 12.0);

    let mut pcm = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let phase = t / duration.as_secs_f32().max(1e-6);
        let sample = articulation_sample(
            note.articulation,
            root_hz,
            minor_third_hz,
            fifth_hz,
            phase,
            t,
        );
        let env = articulation_envelope(note.articulation, phase);
        pcm.push((sample * env * velocity).clamp(-1.0, 1.0));
    }

    let (gain, playback_rate) = articulation_output_shape(note.articulation);
    (Arc::from(pcm), gain, playback_rate)
}

fn articulation_duration(base: Duration, articulation: RhythmGuitarArticulation) -> Duration {
    let scale = match articulation {
        RhythmGuitarArticulation::Sustained => 1.15,
        RhythmGuitarArticulation::PalmMuted => 0.42,
        RhythmGuitarArticulation::Choked => 0.22,
        RhythmGuitarArticulation::RhythmicScratch => 0.18,
        RhythmGuitarArticulation::ChordSlide => 1.0,
    };
    Duration::from_secs_f32((base.as_secs_f32() * scale).max(0.03))
}

fn articulation_output_shape(articulation: RhythmGuitarArticulation) -> (f32, f32) {
    match articulation {
        RhythmGuitarArticulation::Sustained => (0.82, 1.0),
        RhythmGuitarArticulation::PalmMuted => (0.75, 1.0),
        RhythmGuitarArticulation::Choked => (0.7, 1.0),
        RhythmGuitarArticulation::RhythmicScratch => (0.66, 1.0),
        RhythmGuitarArticulation::ChordSlide => (0.84, 1.0),
    }
}

fn articulation_sample(
    articulation: RhythmGuitarArticulation,
    root_hz: f32,
    third_hz: f32,
    fifth_hz: f32,
    phase: f32,
    t: f32,
) -> f32 {
    match articulation {
        RhythmGuitarArticulation::Sustained => {
            chord_stack(root_hz, third_hz, fifth_hz, t, 0.68) + saw(root_hz * 2.0, t) * 0.1
        }
        RhythmGuitarArticulation::PalmMuted => {
            (chord_stack(root_hz, third_hz, fifth_hz, t, 0.5) + saw(root_hz * 3.0, t) * 0.14)
                * (1.0 - phase * 0.75)
        }
        RhythmGuitarArticulation::Choked => {
            chord_stack(root_hz, third_hz, fifth_hz, t, 0.55) * (1.0 - smoothstep(phase * 2.5))
        }
        RhythmGuitarArticulation::RhythmicScratch => {
            let scratch = hash_noise(t * 18_000.0);
            (scratch * 0.6 + triangle(root_hz * 0.7, t) * 0.1) * (1.0 - phase).max(0.0)
        }
        RhythmGuitarArticulation::ChordSlide => {
            let glide = smoothstep(phase);
            let ratio = 0.84 + glide * 0.16;
            chord_stack(root_hz * ratio, third_hz * ratio, fifth_hz * ratio, t, 0.63)
        }
    }
}

fn chord_stack(root_hz: f32, third_hz: f32, fifth_hz: f32, t: f32, drive: f32) -> f32 {
    let raw = saw(root_hz, t) * 0.42 + saw(third_hz, t) * 0.3 + saw(fifth_hz, t) * 0.28;
    (raw * (1.0 + drive)).tanh()
}

fn articulation_envelope(articulation: RhythmGuitarArticulation, phase: f32) -> f32 {
    let attack = match articulation {
        RhythmGuitarArticulation::RhythmicScratch => 0.01,
        RhythmGuitarArticulation::PalmMuted => 0.015,
        _ => 0.02,
    };
    let decay = match articulation {
        RhythmGuitarArticulation::Sustained => 0.46,
        RhythmGuitarArticulation::ChordSlide => 0.55,
        RhythmGuitarArticulation::PalmMuted => 1.0,
        RhythmGuitarArticulation::Choked => 1.35,
        RhythmGuitarArticulation::RhythmicScratch => 1.5,
    };
    let attack_env = smoothstep((phase / attack).clamp(0.0, 1.0));
    let decay_env = (-phase * decay * 4.0).exp();
    (attack_env * decay_env).clamp(0.0, 1.0)
}
