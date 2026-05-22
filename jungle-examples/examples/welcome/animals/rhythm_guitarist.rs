use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::{U1, U2};

use crate::effect::Rest;
use crate::instrumentation::{ElectricGuitarArticulation, Pick, Pluck, Strum};

use super::DecrementCounter;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RhythmGuitaristState {
    #[jungle(focus)]
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
    transition_loops_remaining: u8,
    sustain_loops_remaining: u8,
}

impl Default for RhythmGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::default(),
            riff_loops_remaining: 5,
            transition_loops_remaining: 3,
            sustain_loops_remaining: 1,
        }
    }
}

pub type RhythmGuitaristSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 0;

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

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct RiffLoopRemaining;
impl LoopCondition<RhythmGuitaristState> for RiffLoopRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitaristState) -> bool {
        state.riff_loops_remaining > 0
    }
}

pub struct TransitionLoopRemaining;
impl LoopCondition<RhythmGuitaristState> for TransitionLoopRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitaristState) -> bool {
        state.transition_loops_remaining > 0
    }
}

pub struct IntroSustainNeeded;
impl<In> Condition<(RhythmGuitaristState, In)> for IntroSustainNeeded {
    fn choose(input: &(RhythmGuitaristState, In)) -> bool {
        input.0.transition_loops_remaining == 0
    }
}

type RiffLoopCounter = Lens<RhythmGuitaristState, U1>;
type TransitionLoopCounter = Lens<RhythmGuitaristState, U2>;
type SustainLoopCounter = Lens<RhythmGuitaristState, jungle_sdk::typosaurus::num::consts::U3>;

pub type AdvanceRiffLoop = DecrementCounter<RiffLoopCounter>;
pub type AdvanceTransitionLoop = DecrementCounter<TransitionLoopCounter>;
pub type AdvanceSustainLoop = DecrementCounter<SustainLoopCounter>;

#[derive(Flow)]
pub struct Intro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, IntroPrelude>,
    Transparent<IntroSectionMeta, While<RiffLoopRemaining, IntroRiffLoopBody>>,
    Transparent<IntroSectionMeta, While<TransitionLoopRemaining, IntroTransitionLoopBody>>,
    Transparent<
        IntroSectionMeta,
        Conditional<IntroSustainNeeded, IntroSustainSection, IntroCadence>,
    >,
);

#[derive(Flow)]
pub struct IntroPrelude(
    Transparent<IntroSectionMeta, PreludeRake>,
    Transparent<IntroSectionMeta, PreludeHold>,
    Transparent<IntroSectionMeta, IntroRiffCycle>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
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
#[jungle(focus = ElectricGuitarArticulation)]
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
pub struct IntroRiffLoopBody(
    Transparent<IntroSectionMeta, IntroRiffCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceRiffLoop>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
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
pub struct IntroTransitionLoopBody(
    Transparent<IntroSectionMeta, IntroTransitionCycle>,
    Transparent<IntroSectionMeta, Step<AdvanceTransitionLoop>>,
);

#[derive(Flow)]
pub struct IntroTransitionCycle(
    Transparent<IntroSectionMeta, TransitionBlock58>,
    Transparent<IntroSectionMeta, TransitionBlock56>,
    Transparent<IntroSectionMeta, TransitionBlock53>,
    Transparent<IntroSectionMeta, TransitionEnding>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct TransitionBlock58(
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<58, 58, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct TransitionBlock56(
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<56, 56, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<53, 53, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct TransitionBlock53(
    Step<Pluck<53, 53, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
    Step<Pluck<51, 51, 96, 96>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct TransitionEnding(
    Step<Pluck<49, 49, 96, 96>>,
    Step<Pluck<49, 49, 96, 96>>,
    Step<Strum<46, 46, 49, 96, 96>>,
    Step<Strum<44, 46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroSustainSection(
    Transparent<IntroSectionMeta, SustainHead>,
    Transparent<IntroSectionMeta, While<SustainMiddleRemaining, SustainMiddleLoopBody>>,
    Transparent<IntroSectionMeta, SustainTail>,
    Transparent<IntroSectionMeta, IntroCadence>,
);

pub struct SustainMiddleRemaining;
impl LoopCondition<RhythmGuitaristState> for SustainMiddleRemaining {
    type Arg = ();

    fn should_continue(state: &RhythmGuitaristState) -> bool {
        state.sustain_loops_remaining > 0
    }
}

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct SustainHead(
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<49, 56, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct SustainMiddleBars(
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
);

#[derive(Flow)]
pub struct SustainMiddleLoopBody(
    Transparent<IntroSectionMeta, SustainMiddleBars>,
    Transparent<IntroSectionMeta, Step<AdvanceSustainLoop>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct SustainTail(
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<46, 58, 192, 192>>,
    Step<Pluck<44, 51, 192, 192>>,
    Step<Pick<42, 192, 192>>,
);

#[derive(Flow)]
pub struct IntroCadence(
    Transparent<IntroSectionMeta, CadencePhrase>,
    Transparent<IntroSectionMeta, CadencePhrase>,
    Transparent<IntroSectionMeta, CadencePhraseFinal>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct CadencePhrase(
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
    Step<Pick<44, 192, 192>>,
);

#[derive(Flow)]
#[jungle(focus = ElectricGuitarArticulation)]
pub struct CadencePhraseFinal(
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
    Step<Pick<44, 192, 0>>,
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
