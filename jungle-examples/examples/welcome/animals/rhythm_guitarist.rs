use jungle_sdk::prelude::*;

use crate::instrumentation::{ElectricGuitarArticulation, Pick, Pluck};

pub type RhythmGuitaristState = ElectricGuitarArticulation;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = Buildup;
}

#[derive(Flow)]
pub struct Buildup(
    Triple<58>,
    Triple<56>,
    Triple<53>,
    Triple<51>,
    Step<Pick<49, 96, 96>>,
    Step<Pick<46, 96, 0>>,
);

#[derive(Flow)]
pub struct Triple<const NOTE: u8>(
    Step<Pluck<{ NOTE }, 96, 96>>,
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
    async fn buildup_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-rhythm-buildup-test")
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
