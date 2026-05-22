use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::U1;
use std::time::Duration;

use crate::instrumentation::{ElectricGuitarArticulation, Pick, Pluck};

use super::DecrementCounter;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LeadGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
}

impl Default for LeadGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::Sustained,
            riff_loops_remaining: 6,
        }
    }
}

pub type LeadGuitaristSeed = ();
const INTRO_START_DELAY_MS: u64 = 6_829;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &LeadGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        Duration::from_millis(INTRO_START_DELAY_MS)
    }

    fn absorb(
        _state: &mut LeadGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct LeadIntroRiffRemaining;
impl LoopCondition<LeadGuitaristState> for LeadIntroRiffRemaining {
    type Arg = ();

    fn should_continue(state: &LeadGuitaristState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct LeadIntroCadenceNeeded;
impl<In> Condition<(LeadGuitaristState, In)> for LeadIntroCadenceNeeded {
    fn choose(input: &(LeadGuitaristState, In)) -> bool {
        input.0.riff_loops_remaining == 0
    }
}

type RiffLoopCounter = Lens<LeadGuitaristState, U1>;
pub type AdvanceLeadIntroRiff = DecrementCounter<RiffLoopCounter>;

#[derive(Flow)]
pub struct LeadGuitarIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, LeadPrelude>,
    Transparent<IntroSectionMeta, While<LeadIntroRiffRemaining, LeadIntroRiffLoopBody>>,
    Transparent<
        IntroSectionMeta,
        Conditional<LeadIntroCadenceNeeded, LeadIntroCadence, LeadIntroTail>,
    >,
);

#[derive(Flow)]
pub struct LeadPrelude(
    Transparent<IntroSectionMeta, LeadOpeningPads>,
    Transparent<IntroSectionMeta, LeadAscentFigure>,
    Transparent<IntroSectionMeta, LeadPreRiffCadence>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadOpeningPads(
    Step<Pluck<46, 53, 192, 192>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 96, 96>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<56, 96, 96>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<53, 192, 192>>,
    Step<Pluck<58, 65, 192, 192>>,
    Step<Pluck<58, 65, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pluck<39, 46, 192, 192>>,
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadAscentFigure(
    Step<Pick<46, 192, 192>>,
    Step<Pick<48, 192, 192>>,
    Step<Pick<51, 192, 192>>,
    Step<Pick<53, 192, 192>>,
    Step<Pick<56, 192, 192>>,
    Step<Pick<58, 192, 192>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<63, 192, 192>>,
    Step<Pick<68, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadPreRiffCadence(
    Step<Pluck<63, 68, 192, 192>>,
    Step<Pick<61, 192, 192>>,
    Step<Pick<58, 192, 192>>,
);

#[derive(Flow)]
pub struct LeadIntroRiffLoopBody(
    Transparent<IntroSectionMeta, LeadIntroRiffCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceLeadIntroRiff>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroRiffCycle(
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 96, 96>>,
    Step<Pluck<42, 49, 192, 192>>,
    Step<Pluck<41, 48, 192, 192>>,
    Step<Pluck<39, 46, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroCadence(Step<Pluck<39, 46, 192, 192>>);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct LeadIntroTail(Step<Pick<58, 192, 192>>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::LeadGuitarist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn intro_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-lead-guitar-intro-test")
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
            .start_journey::<LeadGuitarist>(seed)
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
