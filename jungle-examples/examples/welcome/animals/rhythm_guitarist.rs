use jungle_sdk::effect;
use jungle_sdk::prelude::*;

use crate::ecosystem::WelcomeEcosystem;
use crate::effects::Monad;
use crate::flow;
use crate::instrumentation::{ElectricGuitar, ElectricGuitarArticulation};

const TICKS_PER_SECOND: f32 = 787.2;

pub type RhythmGuitaristState = flow::RhythmGuitarIntroState;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = Buildup;
}

#[derive(Flow)]
pub struct TestProbe(Step<Probe>, Step<Probe>);

#[derive(Flow)]
pub struct Buildup(
    Triple<58>,
    Step<Rest<96>>,
    Triple<56>,
    Step<Rest<96>>,
    Triple<53>,
    Step<Rest<96>>,
    Triple<51>,
    Step<Rest<96>>,
    Step<Pick<49, 96>>,
    Step<Rest<96>>,
    Step<Pick<46, 96>>,
);

#[derive(Flow)]
pub struct Triple<const NOTE: u8>(
    Step<Pick<{ NOTE }, 96>>,
    Step<Rest<96>>,
    Step<Pick<{ NOTE }, 96>>,
    Step<Rest<96>>,
    Step<Pick<{ NOTE }, 96>>,
    Step<Rest<96>>,
);

pub struct Pick<const NOTE: u8, const D_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const D_TICK: u8> Act for Pick<NOTE, D_TICK> {
    type Effect = Monad<ElectricGuitar, ElectricGuitarArticulation, NOTE, D_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitaristState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct Rest<const D_TICK: u8>;
#[jungle::act]
impl<const D_TICK: u8> Act for Rest<D_TICK> {
    type Effect = Pause<D_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitaristState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct Pause<const D_TICK: u8>;
#[effect(id = 502)]
impl<const D_TICK: u8> Effect<WelcomeEcosystem> for Pause<D_TICK> {
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(_jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        tokio::time::sleep(std::time::Duration::from_secs_f32(
            D_TICK as f32 / TICKS_PER_SECOND,
        ))
        .await;

        Ok(())
    }
}

pub struct Probe;
#[jungle::act]
impl Act for Probe {
    type Effect = EProbe;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitaristState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("probe should succeed");
    }
}

pub struct EProbe;
#[effect(id = 503)]
impl Effect<WelcomeEcosystem> for EProbe {
    type In = ();
    type Out = ();
    type Err = String;

    async fn effect(_jungle: &WelcomeEcosystem, _note: Self::In) -> Result<Self::Out, Self::Err> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::RhythmGuitarist;
    use crate::ecosystem::WelcomeEcosystem;
    use crate::PlaybackClock;

    #[tokio::test]
    async fn buildup_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-rhythm-buildup-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
        let playback_clock = PlaybackClock::default();
        let _ = playback_clock.start_now();
        let ecosystem = WelcomeEcosystem::new(audio_handle, 123.0, playback_clock);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<RhythmGuitarist>(seed)
            .await
            .expect("journey should start");

        let completion = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let status = client
                    .journey_details(journey_id)
                    .await
                    .expect("journey details should be available");
                match status {
                    JourneyStatus::Completed => break,
                    JourneyStatus::Dead | JourneyStatus::Stopped => {
                        panic!("journey reached terminal non-complete status: {status:?}");
                    }
                    JourneyStatus::Created | JourneyStatus::Alive => {}
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(
            completion.is_ok(),
            "buildup journey did not complete before timeout"
        );

        worker_handle.abort();
        let _ = worker_handle.await;
    }
}
