use jungle_sdk::effect;

use crate::{instrumentation::Instrument, WelcomeEcosystem};

pub struct PlayRhythmGuitarIntroNote;

#[effect(id = 200)]
impl Effect<WelcomeEcosystem> for PlayRhythmGuitarIntroNote {
    type In = u16;
    type Out = ();
    type Err = String;

    async fn effect(
        jungle: &WelcomeEcosystem,
        index: Self::In,
    ) -> Result<Self::Out, Self::Err> {
        let Some(note) = jungle.rhythm_guitar_intro_note(index) else {
            return Ok(());
        };

        let started_at = jungle.playback_clock().wait_started().await;
        let target_instant = started_at + note.at;
        if target_instant > tokio::time::Instant::now() {
            tokio::time::sleep_until(target_instant).await;
        }

        jungle
            .rhythm_guitar()
            .play(note.note)
            .await
            .map_err(|err| err.to_string())
    }
}
