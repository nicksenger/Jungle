use jungle_sdk::effect;

use crate::{
    flow::PlayNoteCommand,
    instrumentation::{ElectricGuitarArticulation, Instrument, Note},
    WelcomeEcosystem,
};

const BEATS_PER_BAR: f32 = 4.0;

pub struct SetRhythmGuitarBarProfile;

#[effect(id = 200)]
impl Effect<WelcomeEcosystem> for SetRhythmGuitarBarProfile {
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(_jungle: &WelcomeEcosystem, _input: Self::In) -> Result<Self::Out, Self::Err> {
        Ok(())
    }
}

pub struct PlayRhythmGuitarIntroNote;

#[effect(id = 201)]
impl Effect<WelcomeEcosystem> for PlayRhythmGuitarIntroNote {
    type In = PlayNoteCommand;
    type Out = ();
    type Err = String;

    async fn effect(
        jungle: &WelcomeEcosystem,
        note: Self::In,
    ) -> Result<Self::Out, Self::Err> {
        let bpm = jungle.bpm();
        let seconds_per_beat = 60.0_f32 / bpm;
        let beat_offset = if note.beat_offset_den == 0 {
            0.0
        } else {
            note.beat_offset_num as f32 / note.beat_offset_den as f32
        };
        let absolute_beats = ((note.bar.saturating_sub(1)) as f32 * BEATS_PER_BAR)
            + (note.beat.saturating_sub(1)) as f32
            + beat_offset;
        let target_instant =
            jungle.playback_clock().wait_started().await
                + std::time::Duration::from_secs_f32(absolute_beats * seconds_per_beat);
        if target_instant > tokio::time::Instant::now() {
            tokio::time::sleep_until(target_instant).await;
        }

        let duration_beats = note.duration_num as f32 / note.duration_den as f32;
        let playable_note = Note {
            n_midi: note.midi,
            amplitude_multiplier: 0.5,
            pan: 0.5,
            duration: std::time::Duration::from_secs_f32(duration_beats * seconds_per_beat),
            velocity: 37.0 / 127.0,
            expression: None,
            articulation: ElectricGuitarArticulation::RhythmSustained,
        };
        jungle
            .rhythm_guitar()
            .play(playable_note)
            .await
            .map_err(|err| err.to_string())
    }
}

pub struct AdvanceRhythmGuitarBar;

#[effect(id = 202)]
impl Effect<WelcomeEcosystem> for AdvanceRhythmGuitarBar {
    type In = u8;
    type Out = u8;
    type Err = String;

    async fn effect(_jungle: &WelcomeEcosystem, bar: Self::In) -> Result<Self::Out, Self::Err> {
        Ok(bar.saturating_add(1))
    }
}
