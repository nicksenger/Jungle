use jungle_sdk::prelude::*;

use crate::effect::{Dyad, Monad, Triad};
use crate::instrumentation::{ElectricGuitar, ElectricGuitarArticulation};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct RhythmGuitaristState {
    articulation: ElectricGuitarArticulation,
    riff_loops_remaining: u8,
    transition_loops_remaining: u8,
}

impl Default for RhythmGuitaristState {
    fn default() -> Self {
        Self {
            articulation: ElectricGuitarArticulation::default(),
            riff_loops_remaining: 5,
            transition_loops_remaining: 3,
        }
    }
}

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

pub struct AdvanceRiffLoop;
#[jungle::act]
impl Act for AdvanceRiffLoop {
    type Effect = StubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitaristState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("advance riff loop should succeed");
        state.riff_loops_remaining = state.riff_loops_remaining.saturating_sub(1);
    }
}

pub struct AdvanceTransitionLoop;
#[jungle::act]
impl Act for AdvanceTransitionLoop {
    type Effect = StubEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &RhythmGuitaristState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("advance transition loop should succeed");
        state.transition_loops_remaining = state.transition_loops_remaining.saturating_sub(1);
    }
}

pub struct StubEffect;
#[jungle::effect(id = 512)]
impl<J> Effect<J> for StubEffect {
    type In = ();
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        std::future::ready(Ok(()))
    }
}

pub struct RPick<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for RPick<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<ElectricGuitar, ElectricGuitarArticulation, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        state.articulation
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct RPluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for RPluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect =
        Dyad<ElectricGuitar, ElectricGuitarArticulation, NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        state.articulation
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct RStrum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>;
#[jungle::act]
impl<
        const NOTE_1: u8,
        const NOTE_2: u8,
        const NOTE_3: u8,
        const NOTE_TICK: u8,
        const REST_TICK: u8,
    > Act for RStrum<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
{
    type Effect = Triad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        NOTE_1,
        NOTE_2,
        NOTE_3,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &RhythmGuitaristState,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        state.articulation
    }

    fn absorb(
        _state: &mut RhythmGuitaristState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

#[derive(Flow)]
pub struct Intro(
    Transparent<IntroSectionMeta, IntroPrelude>,
    Transparent<IntroSectionMeta, While<RiffLoopRemaining, IntroRiffLoopBody>>,
    Transparent<IntroSectionMeta, While<TransitionLoopRemaining, IntroTransitionLoopBody>>,
    Transparent<
        IntroSectionMeta,
        Conditional<IntroSustainNeeded, IntroSustainSection, IntroCadence>,
    >,
);

#[derive(Flow)]
pub struct IntroPrelude(PreludeRake, PreludeHold, IntroRiffCycle);

#[derive(Flow)]
pub struct PreludeRake(
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct PreludeHold(
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroRiffLoopBody(IntroRiffCycle, Step<AdvanceRiffLoop>);

#[derive(Flow)]
pub struct IntroRiffCycle(
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPick<58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<56, 56, 96, 96>>,
    Step<RPick<56, 96, 96>>,
    Step<RPluck<56, 56, 96, 96>>,
    Step<RPluck<53, 53, 96, 96>>,
    Step<RPick<53, 96, 96>>,
    Step<RPluck<53, 53, 96, 96>>,
    Step<RPluck<51, 51, 96, 96>>,
    Step<RPick<51, 96, 96>>,
    Step<RPluck<51, 51, 96, 96>>,
    Step<RPluck<49, 49, 96, 96>>,
    Step<RPick<49, 96, 96>>,
    Step<RStrum<46, 46, 49, 96, 96>>,
    Step<RPluck<46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroTransitionLoopBody(IntroTransitionCycle, Step<AdvanceTransitionLoop>);

#[derive(Flow)]
pub struct IntroTransitionCycle(
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<58, 58, 96, 96>>,
    Step<RPluck<56, 56, 96, 96>>,
    Step<RPluck<56, 56, 96, 96>>,
    Step<RPluck<56, 56, 96, 96>>,
    Step<RPluck<53, 53, 96, 96>>,
    Step<RPluck<53, 53, 96, 96>>,
    Step<RPluck<53, 53, 96, 96>>,
    Step<RPluck<51, 51, 96, 96>>,
    Step<RPluck<51, 51, 96, 96>>,
    Step<RPluck<51, 51, 96, 96>>,
    Step<RPluck<49, 49, 96, 96>>,
    Step<RPluck<49, 49, 96, 96>>,
    Step<RStrum<46, 46, 49, 96, 96>>,
    Step<RStrum<44, 46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroSustainSection(IntroSustainBody, IntroCadence);

#[derive(Flow)]
pub struct IntroSustainBody(
    Step<RPluck<49, 56, 192, 192>>,
    Step<RPluck<49, 56, 192, 192>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<46, 58, 192, 192>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPick<42, 192, 192>>,
);

#[derive(Flow)]
pub struct IntroCadence(
    Step<RPluck<44, 49, 96, 96>>,
    Step<RPluck<44, 49, 96, 192>>,
    Step<RPluck<49, 54, 96, 96>>,
    Step<RPick<42, 96, 192>>,
    Step<RPick<41, 96, 192>>,
    Step<RPick<44, 192, 192>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<44, 49, 96, 96>>,
    Step<RPluck<44, 49, 96, 96>>,
    Step<RPick<42, 192, 192>>,
    Step<RPluck<44, 51, 96, 96>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<49, 54, 96, 96>>,
    Step<RPick<42, 96, 192>>,
    Step<RPick<41, 96, 192>>,
    Step<RPick<44, 192, 192>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<44, 49, 96, 96>>,
    Step<RPluck<44, 49, 96, 96>>,
    Step<RPick<42, 192, 192>>,
    Step<RPluck<44, 51, 96, 96>>,
    Step<RPluck<44, 51, 192, 192>>,
    Step<RPluck<49, 54, 96, 96>>,
    Step<RPick<42, 96, 192>>,
    Step<RPick<41, 96, 192>>,
    Step<RPick<44, 192, 0>>,
);

#[derive(Flow)]
pub struct Buildup(
    BuildupTriplet<58>,
    BuildupTriplet<56>,
    BuildupTriplet<53>,
    BuildupTriplet<51>,
    Step<RPick<49, 96, 96>>,
    Step<RPick<46, 96, 0>>,
);

#[derive(Flow)]
pub struct BuildupTriplet<const NOTE: u8>(
    Step<RPluck<{ NOTE }, { NOTE }, 96, 96>>,
    Step<RPick<{ NOTE }, 96, 96>>,
    Step<RPick<{ NOTE }, 96, 96>>,
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
