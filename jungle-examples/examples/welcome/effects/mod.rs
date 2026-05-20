use jungle_sdk::effect;

use crate::{
    flow::RhythmGuitarIntroBarSpec,
    instrumentation::{ElectricGuitarArticulation, Instrument, Note},
    WelcomeEcosystem,
};

pub struct PlayRhythmGuitarIntroNote;

#[effect(id = 200)]
impl Effect<WelcomeEcosystem> for PlayRhythmGuitarIntroNote {
    type In = RhythmGuitarIntroBarSpec;
    type Out = RhythmGuitarIntroBarSpec;
    type Err = String;

    async fn effect(
        jungle: &WelcomeEcosystem,
        bar: Self::In,
    ) -> Result<Self::Out, Self::Err> {
        let started_at = jungle.playback_clock().wait_started().await;
        let rhythm_guitar = jungle.rhythm_guitar();
        let bpm = jungle.bpm();
        let seconds_per_beat = 60.0_f32 / bpm;

        for note in &bar.notes {
            let beat_offset = if note.beat_offset_den == 0 {
                0.0
            } else {
                note.beat_offset_num as f32 / note.beat_offset_den as f32
            };
            let absolute_beats = ((note.bar.saturating_sub(1)) as f32 * 4.0)
                + (note.beat.saturating_sub(1)) as f32
                + beat_offset;
            let target_instant =
                started_at + std::time::Duration::from_secs_f32(absolute_beats * seconds_per_beat);
            if target_instant > tokio::time::Instant::now() {
                tokio::time::sleep_until(target_instant).await;
            }

            let note_duration_beats = note.duration_num as f32 / note.duration_den as f32;
            let playable_note = Note {
                n_midi: note.midi,
                amplitude_multiplier: 0.5,
                pan: 0.5,
                duration: std::time::Duration::from_secs_f32(note_duration_beats * seconds_per_beat),
                velocity: 37.0 / 127.0,
                expression: None,
                articulation: ElectricGuitarArticulation::RhythmSustained,
            };

            rhythm_guitar
                .play(playable_note)
                .await
                .map_err(|err| err.to_string())?;
        }

        Ok(bar)
    }
}
