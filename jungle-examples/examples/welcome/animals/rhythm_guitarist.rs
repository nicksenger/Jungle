use jungle_sdk::prelude::*;

use crate::instrumentation::{ElectricGuitarArticulation, Pick, Pluck, Strum};

pub type RhythmGuitaristState = ElectricGuitarArticulation;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = Intro;
}

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

#[derive(Flow)]
pub struct Intro(
    Transparent<IntroSectionMeta, IntroPrelude>,
    Transparent<IntroSectionMeta, IntroRiffSection>,
    Transparent<IntroSectionMeta, IntroTransitionSection>,
    Transparent<IntroSectionMeta, IntroSustainSection>,
    Transparent<IntroSectionMeta, IntroCadence>,
);

#[derive(Flow)]
pub struct IntroPrelude(PreludeRake, PreludeHold, IntroRiffCycle);

#[derive(Flow)]
pub struct PreludeRake(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct PreludeHold(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroRiffSection(
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
);

#[derive(Flow)]
pub struct IntroRiffCycle(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 46, 49, 96, 96>>,
    Step<Pluck<46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroTransitionSection(
    IntroTransitionCycle,
    IntroTransitionCycle,
    IntroTransitionCycle,
);

#[derive(Flow)]
pub struct IntroTransitionCycle(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 46, 49, 96, 96>>,
    Step<Strum<44, 46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroSustainSection(
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pick<42, 192, 192>>,
);

#[derive(Flow)]
pub struct IntroCadence(
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<44, 49, 96, 192>>,
    Step<Pluck<49, 54, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<49, 54, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pluck<44, 49, 96, 96>>,
    Step<Pick<42, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<49, 54, 96, 96>>,
    Step<Pick<42, 96, 192>>,
    Step<Pick<41, 96, 192>>,
    Step<Pick<44, 192, 0>>,
);

#[derive(Flow)]
pub struct Buildup(
    BuildupTriplet<58>,
    BuildupTriplet<56>,
    BuildupTriplet<53>,
    BuildupTriplet<51>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<46, 96, 0>>,
);

#[derive(Flow)]
pub struct BuildupTriplet<const NOTE: u8>(
    Step<Pluck<{ NOTE }, { NOTE }, 96, 96>>,
    Step<Pick<{ NOTE }, 96, 96>>,
    Step<Pick<{ NOTE }, 96, 96>>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::RhythmGuitarist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn intro_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-rhythm-intro-test")
            .build()
            .await
            .expect("local client should build");

        let (audio_handle, _audio_keep_alive) = crate::audio::AudioHandle::stub();
        let ecosystem = TheJungle::new(audio_handle, 123.0);

        let worker = JungleWorker::new(ecosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&()).expect("seed should serialize");
        let journey_id = client
            .start_journey::<RhythmGuitarist>(seed)
            .await
            .expect("journey should start");

        let completion = tokio::time::timeout(Duration::from_secs(40), async {
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
            "intro journey did not complete before timeout"
        );

        worker_handle.abort();
        let _ = worker_handle.await;
    }
}
