use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::{U1, U2};

use crate::effect::Rest;
use crate::instrumentation::{BassArticulation, Thump as LaneThump};

use super::{Bass, DecrementCounter};

const BASS_LANE_ID: u32 = <<Bass as Animal>::Id as AnimalIdValue>::U32;
type Thump<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> =
    LaneThump<NOTE, NOTE_TICK, REST_TICK, BASS_LANE_ID>;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BassistState {
    #[jungle(focus)]
    articulation: BassArticulation,
    ostinato_loops_remaining: u8,
    riff_loops_remaining: u8,
}

impl Default for BassistState {
    fn default() -> Self {
        Self {
            articulation: BassArticulation::Picked,
            ostinato_loops_remaining: 1,
            riff_loops_remaining: 3,
        }
    }
}

pub type BassistSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 5_376;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<BASS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &BassistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(_state: &mut BassistState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct OstinatoLoopsRemaining;
impl LoopCondition<BassistState> for OstinatoLoopsRemaining {
    type Arg = ();

    fn should_continue(state: &BassistState) -> bool {
        state.ostinato_loops_remaining > 0
    }
}

pub struct RiffLoopsRemaining;
impl LoopCondition<BassistState> for RiffLoopsRemaining {
    type Arg = ();

    fn should_continue(state: &BassistState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct IntroTailNeeded;
impl<In> Condition<(BassistState, In)> for IntroTailNeeded {
    fn choose(input: &(BassistState, In)) -> bool {
        input.0.riff_loops_remaining == 0
    }
}

type OstinatoLoopCounter = Lens<BassistState, U1>;
type RiffLoopCounter = Lens<BassistState, U2>;

pub type AdvanceOstinatoLoop = DecrementCounter<OstinatoLoopCounter>;
pub type AdvanceRiffLoop = DecrementCounter<RiffLoopCounter>;

#[derive(Flow)]
pub struct BassIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, BassPrelude>,
    Transparent<IntroSectionMeta, While<OstinatoLoopsRemaining, BassOstinatoLoopBody>>,
    Transparent<IntroSectionMeta, BassTransition>,
    Transparent<IntroSectionMeta, While<RiffLoopsRemaining, BassRiffLoopBody>>,
    Transparent<IntroSectionMeta, Conditional<IntroTailNeeded, BassTail, BassRelease>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassPrelude(
    Step<Thump<46, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<37, 96, 96>>,
    Step<Thump<38, 96, 96>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
pub struct BassOstinatoLoopBody(
    Transparent<IntroSectionMeta, BassOstinatoCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceOstinatoLoop>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassOstinatoCycle(
    Step<Thump<44, 96, 96>>,
    Step<Thump<45, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<46, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<44, 96, 96>>,
    Step<Thump<43, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<39, 96, 96>>,
    Step<Thump<34, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassTransition(
    Step<Thump<37, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<34, 192, 192>>,
    Step<Thump<41, 192, 192>>,
    Step<Thump<44, 192, 192>>,
    Step<Thump<45, 192, 192>>,
    Step<Thump<46, 192, 192>>,
    Step<Thump<27, 192, 192>>,
    Step<Thump<39, 192, 192>>,
);

#[derive(Flow)]
pub struct BassRiffLoopBody(
    Transparent<IntroSectionMeta, BassRiffCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceRiffLoop>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassRiffCycle(
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassTail(
    Step<Thump<32, 192, 192>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<32, 192, 192>>,
    Step<Thump<27, 96, 96>>,
    Step<Thump<30, 192, 192>>,
    Step<Thump<29, 192, 192>>,
    Step<Thump<27, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = BassArticulation)]
pub struct BassRelease(Step<Thump<27, 192, 192>>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::Bass;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn intro_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-bass-intro-test")
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
            .start_journey::<Bass>(seed)
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
