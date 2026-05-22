use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::num::consts::U1;

use crate::effect::Rest;
use crate::instrumentation::{Sing as LaneSing, VocalsArticulation};

use super::DecrementCounter;
use super::LeadVocalist;

const LEAD_VOCALS_LANE_ID: u32 = <<LeadVocalist as Animal>::Id as AnimalIdValue>::U32;
type Sing<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32> =
    LaneSing<NOTE, NOTE_TICK, REST_TICK, LEAD_VOCALS_LANE_ID>;

#[derive(Optic, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LeadVocalistState {
    #[jungle(focus)]
    articulation: VocalsArticulation,
    intro_pickup_remaining: u8,
}

impl Default for LeadVocalistState {
    fn default() -> Self {
        Self {
            articulation: VocalsArticulation::SirenScream,
            intro_pickup_remaining: 1,
        }
    }
}

pub type LeadVocalistSeed = ();
const INTRO_START_DELAY_TICKS: u32 = 20_352;

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
}

pub struct IntroStartDelay;
#[jungle::act]
impl Act for IntroStartDelay {
    type Effect = Rest<LEAD_VOCALS_LANE_ID, INTRO_START_DELAY_TICKS>;
    type Input = ();
    type Output = ();

    fn emit(_state: &LeadVocalistState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut LeadVocalistState,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct IntroPickupRemaining;
impl LoopCondition<LeadVocalistState> for IntroPickupRemaining {
    type Arg = ();

    fn should_continue(state: &LeadVocalistState) -> bool {
        state.intro_pickup_remaining > 0
    }
}

pub struct IntroNeedsPickup;
impl<In> Condition<(LeadVocalistState, In)> for IntroNeedsPickup {
    fn choose(input: &(LeadVocalistState, In)) -> bool {
        input.0.intro_pickup_remaining == 0
    }
}

type IntroPickupCounter = Lens<LeadVocalistState, U1>;
pub type AdvanceIntroPickup = DecrementCounter<IntroPickupCounter>;

#[derive(Flow)]
pub struct LeadVocalIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, IntroBreath>,
    Transparent<IntroSectionMeta, IntroPickupLoop>,
    Transparent<IntroSectionMeta, Conditional<IntroNeedsPickup, IntroRelease, IntroRest>>,
);

#[derive(Flow)]
pub struct IntroBreath(Transparent<IntroSectionMeta, IntroBreathPhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroBreathPhrase(Step<Sing<58, 192, 192>>);

#[derive(Flow)]
pub struct IntroPickupLoop(While<IntroPickupRemaining, IntroPickupBody>);

#[derive(Flow)]
pub struct IntroPickupBody(
    Transparent<IntroSectionMeta, IntroPickupPhrase>,
    Transparent<IntroSectionMeta, Step<AdvanceIntroPickup>>,
);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroPickupPhrase(Step<Sing<58, 192, 192>>);

#[derive(Flow)]
pub struct IntroRest(Transparent<IntroSectionMeta, IntroRestPhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroRestPhrase(Step<Sing<58, 192, 192>>);

#[derive(Flow)]
pub struct IntroRelease(Transparent<IntroSectionMeta, IntroReleasePhrase>);

#[derive(Flow)]
#[jungle(focus = VocalsArticulation)]
pub struct IntroReleasePhrase(Step<Sing<58, 192, 192>>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::LeadVocalist;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn intro_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-lead-vocal-intro-test")
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
            .start_journey::<LeadVocalist>(seed)
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

//3, 0, Title_t, "Lead vocals"
//3, 0, Program_c, 2, 66
//3, 20352, Note_on_c, 2, 58, 37
//3, 20544, Note_off_c, 2, 58, 0
//3, 26880, Note_on_c, 2, 66, 37
//3, 26976, Note_off_c, 2, 66, 0
//3, 26976, Note_on_c, 2, 68, 37
//3, 27264, Note_off_c, 2, 68, 0
//3, 27264, Note_on_c, 2, 68, 37
//3, 27360, Note_off_c, 2, 68, 0
//3, 27360, Note_on_c, 2, 66, 37
//3, 27456, Note_off_c, 2, 66, 0
//3, 27456, Note_on_c, 2, 71, 37
//3, 27840, Note_off_c, 2, 71, 0
//3, 27840, Note_on_c, 2, 68, 37
//3, 28032, Note_off_c, 2, 68, 0
//3, 28416, Note_on_c, 2, 66, 37
//3, 28512, Note_off_c, 2, 66, 0
//3, 28512, Note_on_c, 2, 68, 37
//3, 28800, Note_off_c, 2, 68, 0
//3, 28800, Note_on_c, 2, 68, 37
//3, 28896, Note_off_c, 2, 68, 0
//3, 28896, Note_on_c, 2, 66, 37
//3, 29088, Note_off_c, 2, 66, 0
//3, 29088, Note_on_c, 2, 68, 37
//3, 29376, Note_off_c, 2, 68, 0
//3, 29376, Note_on_c, 2, 66, 37
//3, 29568, Note_off_c, 2, 66, 0
//3, 29952, Note_on_c, 2, 66, 37
//3, 30048, Note_off_c, 2, 66, 0
//3, 30048, Note_on_c, 2, 68, 37
//3, 30336, Note_off_c, 2, 68, 0
//3, 30336, Note_on_c, 2, 68, 37
//3, 30432, Note_off_c, 2, 68, 0
//3, 30432, Note_on_c, 2, 68, 37
//3, 30624, Note_off_c, 2, 68, 0
//3, 30624, Note_on_c, 2, 71, 37
//3, 30912, Note_off_c, 2, 71, 0
//3, 30912, Note_on_c, 2, 68, 37
//3, 31008, Note_off_c, 2, 68, 0
//3, 31008, Note_on_c, 2, 71, 37
//3, 31488, Note_off_c, 2, 71, 0
//3, 31488, Note_on_c, 2, 66, 37
//3, 31680, Note_off_c, 2, 66, 0
//3, 31680, Note_on_c, 2, 68, 37
//3, 31776, Note_off_c, 2, 68, 0
//3, 31776, Note_on_c, 2, 68, 37
//3, 31872, Note_off_c, 2, 68, 0
//3, 31872, Note_on_c, 2, 68, 37
//3, 32064, Note_off_c, 2, 68, 0
//3, 32064, Note_on_c, 2, 66, 37
//3, 32160, Note_off_c, 2, 66, 0
//3, 32160, Note_on_c, 2, 68, 37
//3, 32448, Note_off_c, 2, 68, 0
//3, 32448, Note_on_c, 2, 68, 37
//3, 32640, Note_off_c, 2, 68, 0
//3, 32640, Note_on_c, 2, 68, 37
//3, 32832, Note_off_c, 2, 68, 0
//3, 32832, Note_on_c, 2, 68, 37
//3, 33024, Note_off_c, 2, 68, 0
//3, 33024, Note_on_c, 2, 63, 37
//3, 33120, Note_off_c, 2, 63, 0
//3, 33120, Note_on_c, 2, 63, 37
//3, 33312, Note_off_c, 2, 63, 0
//3, 33312, Note_on_c, 2, 63, 37
//3, 33504, Note_off_c, 2, 63, 0
//3, 33504, Note_on_c, 2, 61, 37
//3, 33696, Note_off_c, 2, 61, 0
//3, 33696, Note_on_c, 2, 63, 37
//3, 34176, Note_off_c, 2, 63, 0
//3, 34464, Note_on_c, 2, 58, 37
//3, 34560, Note_off_c, 2, 58, 0
//3, 34560, Note_on_c, 2, 63, 37
//3, 34656, Note_off_c, 2, 63, 0
//3, 34656, Note_on_c, 2, 63, 37
//3, 34848, Note_off_c, 2, 63, 0
//3, 34848, Note_on_c, 2, 66, 37
//3, 35136, Note_off_c, 2, 66, 0
//3, 35136, Note_on_c, 2, 63, 37
//3, 35328, Note_off_c, 2, 63, 0
//3, 35328, Note_on_c, 2, 63, 37
//3, 35424, Note_off_c, 2, 63, 0
//3, 35424, Note_on_c, 2, 61, 37
//3, 35712, Note_off_c, 2, 61, 0
//3, 36096, Note_on_c, 2, 61, 37
//3, 36288, Note_off_c, 2, 61, 0
//3, 36288, Note_on_c, 2, 63, 37
//3, 36480, Note_off_c, 2, 63, 0
//3, 36480, Note_on_c, 2, 63, 37
//3, 36672, Note_off_c, 2, 63, 0
//3, 36672, Note_on_c, 2, 63, 37
//3, 36864, Note_off_c, 2, 63, 0
//3, 36864, Note_on_c, 2, 66, 37
//3, 36960, Note_off_c, 2, 66, 0
//3, 36960, Note_on_c, 2, 66, 37
//3, 37248, Note_off_c, 2, 66, 0
//3, 37248, Note_on_c, 2, 66, 37
//3, 37344, Note_off_c, 2, 66, 0
//3, 37344, Note_on_c, 2, 66, 37
//3, 37632, Note_off_c, 2, 66, 0
//3, 37632, Note_on_c, 2, 61, 37
//3, 37824, Note_off_c, 2, 61, 0
//3, 37824, Note_on_c, 2, 63, 37
//3, 38016, Note_off_c, 2, 63, 0
//3, 38016, Note_on_c, 2, 63, 37
//3, 38208, Note_off_c, 2, 63, 0
//3, 38208, Note_on_c, 2, 61, 37
//3, 38304, Note_off_c, 2, 61, 0
//3, 38304, Note_on_c, 2, 63, 37
//3, 38592, Note_off_c, 2, 63, 0
//3, 38592, Note_on_c, 2, 61, 37
//3, 38784, Note_off_c, 2, 61, 0
//3, 38784, Note_on_c, 2, 63, 37
//3, 38976, Note_off_c, 2, 63, 0
//3, 38976, Note_on_c, 2, 63, 37
//3, 39072, Note_off_c, 2, 63, 0
//3, 39072, Note_on_c, 2, 66, 37
//3, 39360, Note_off_c, 2, 66, 0
//3, 39360, Note_on_c, 2, 63, 37
//3, 39552, Note_off_c, 2, 63, 0
//3, 39936, Note_on_c, 2, 70, 37
//3, 40128, Note_off_c, 2, 70, 0
//3, 40128, Note_on_c, 2, 70, 37
//3, 40320, Note_off_c, 2, 70, 0
//3, 40320, Note_on_c, 2, 68, 37
//3, 40416, Note_off_c, 2, 68, 0
//3, 40416, Note_on_c, 2, 66, 37
//3, 40512, Note_off_c, 2, 66, 0
//3, 40512, Note_on_c, 2, 70, 37
//3, 40896, Note_off_c, 2, 70, 0
//3, 40896, Note_on_c, 2, 68, 37
//3, 41088, Note_off_c, 2, 68, 0
//3, 41088, Note_on_c, 2, 68, 37
//3, 41280, Note_off_c, 2, 68, 0
//3, 41280, Note_on_c, 2, 66, 37
//3, 41472, Note_off_c, 2, 66, 0
//3, 41472, Note_on_c, 2, 68, 37
//3, 41664, Note_off_c, 2, 68, 0
//3, 41664, Note_on_c, 2, 66, 37
//3, 41856, Note_off_c, 2, 66, 0
//3, 41856, Note_on_c, 2, 68, 37
//3, 42048, Note_off_c, 2, 68, 0
//3, 42048, Note_on_c, 2, 66, 37
//3, 42144, Note_off_c, 2, 66, 0
//3, 42624, Note_on_c, 2, 68, 37
//3, 42720, Note_off_c, 2, 68, 0
//3, 42720, Note_on_c, 2, 68, 37
//3, 42816, Note_off_c, 2, 68, 0
//3, 42816, Note_on_c, 2, 68, 37
//3, 42912, Note_off_c, 2, 68, 0
//3, 42912, Note_on_c, 2, 68, 37
//3, 43008, Note_off_c, 2, 68, 0
//3, 43008, Note_on_c, 2, 68, 37
//3, 43104, Note_off_c, 2, 68, 0
//3, 43104, Note_on_c, 2, 68, 37
//3, 43200, Note_off_c, 2, 68, 0
//3, 43200, Note_on_c, 2, 68, 37
//3, 43296, Note_off_c, 2, 68, 0
//3, 43296, Note_on_c, 2, 68, 37
//3, 43392, Note_off_c, 2, 68, 0
//3, 43392, Note_on_c, 2, 68, 37
//3, 43488, Note_off_c, 2, 68, 0
//3, 43488, Note_on_c, 2, 68, 37
//3, 43584, Note_off_c, 2, 68, 0
//3, 43584, Note_on_c, 2, 68, 37
//3, 43680, Note_off_c, 2, 68, 0
//3, 43680, Note_on_c, 2, 68, 37
//3, 43776, Note_off_c, 2, 68, 0
//3, 43776, Note_on_c, 2, 70, 37
//3, 44160, Note_off_c, 2, 70, 0
//3, 44448, Note_on_c, 2, 70, 37
//3, 44928, Note_off_c, 2, 70, 0
//3, 45504, Note_on_c, 2, 70, 37
//3, 45888, Note_off_c, 2, 70, 0
//3, 46272, Note_on_c, 2, 70, 37
//3, 46656, Note_off_c, 2, 70, 0
//3, 46944, Note_on_c, 2, 73, 37
//3, 47040, Note_off_c, 2, 73, 0
//3, 47040, Note_on_c, 2, 70, 37
//3, 47136, Note_off_c, 2, 70, 0
//3, 47136, Note_on_c, 2, 70, 37
//3, 47232, Note_off_c, 2, 70, 0
//3, 47232, Note_on_c, 2, 68, 37
//3, 47520, Note_off_c, 2, 68, 0
//3, 47520, Note_on_c, 2, 66, 37
//3, 47808, Note_off_c, 2, 66, 0
//3, 47808, Note_on_c, 2, 73, 37
//3, 48384, Note_off_c, 2, 73, 0
//3, 48384, Note_on_c, 2, 66, 37
//3, 48480, Note_off_c, 2, 66, 0
//3, 48480, Note_on_c, 2, 68, 37
//3, 48768, Note_off_c, 2, 68, 0
//3, 48768, Note_on_c, 2, 68, 37
//3, 48864, Note_off_c, 2, 68, 0
//3, 48864, Note_on_c, 2, 66, 37
//3, 48960, Note_off_c, 2, 66, 0
//3, 48960, Note_on_c, 2, 71, 37
//3, 49344, Note_off_c, 2, 71, 0
//3, 49344, Note_on_c, 2, 68, 37
//3, 49536, Note_off_c, 2, 68, 0
//3, 49728, Note_on_c, 2, 59, 37
//3, 49920, Note_off_c, 2, 59, 0
//3, 49920, Note_on_c, 2, 66, 37
//3, 50016, Note_off_c, 2, 66, 0
//3, 50016, Note_on_c, 2, 68, 37
//3, 50304, Note_off_c, 2, 68, 0
//3, 50304, Note_on_c, 2, 68, 37
//3, 50400, Note_off_c, 2, 68, 0
//3, 50400, Note_on_c, 2, 66, 37
//3, 50592, Note_off_c, 2, 66, 0
//3, 50592, Note_on_c, 2, 68, 37
//3, 50880, Note_off_c, 2, 68, 0
//3, 50880, Note_on_c, 2, 66, 37
//3, 51072, Note_off_c, 2, 66, 0
//3, 51456, Note_on_c, 2, 66, 37
//3, 51552, Note_off_c, 2, 66, 0
//3, 51552, Note_on_c, 2, 68, 37
//3, 51840, Note_off_c, 2, 68, 0
//3, 51840, Note_on_c, 2, 68, 37
//3, 51936, Note_off_c, 2, 68, 0
//3, 51936, Note_on_c, 2, 68, 37
//3, 52128, Note_off_c, 2, 68, 0
//3, 52128, Note_on_c, 2, 71, 37
//3, 52416, Note_off_c, 2, 71, 0
//3, 52416, Note_on_c, 2, 68, 37
//3, 52512, Note_off_c, 2, 68, 0
//3, 52512, Note_on_c, 2, 71, 37
//3, 52992, Note_off_c, 2, 71, 0
//3, 52992, Note_on_c, 2, 66, 37
//3, 53184, Note_off_c, 2, 66, 0
//3, 53184, Note_on_c, 2, 68, 37
//3, 53280, Note_off_c, 2, 68, 0
//3, 53280, Note_on_c, 2, 68, 37
//3, 53376, Note_off_c, 2, 68, 0
//3, 53376, Note_on_c, 2, 68, 37
//3, 53568, Note_off_c, 2, 68, 0
//3, 53568, Note_on_c, 2, 66, 37
//3, 53664, Note_off_c, 2, 66, 0
//3, 53664, Note_on_c, 2, 68, 37
//3, 53952, Note_off_c, 2, 68, 0
//3, 53952, Note_on_c, 2, 68, 37
//3, 54144, Note_off_c, 2, 68, 0
//3, 54144, Note_on_c, 2, 68, 37
//3, 54336, Note_off_c, 2, 68, 0
//3, 54336, Note_on_c, 2, 68, 37
//3, 54528, Note_off_c, 2, 68, 0
//3, 54528, Note_on_c, 2, 63, 37
//3, 54624, Note_off_c, 2, 63, 0
//3, 54624, Note_on_c, 2, 63, 37
//3, 54816, Note_off_c, 2, 63, 0
//3, 54816, Note_on_c, 2, 63, 37
//3, 55008, Note_off_c, 2, 63, 0
//3, 55008, Note_on_c, 2, 61, 37
//3, 55200, Note_off_c, 2, 61, 0
//3, 55200, Note_on_c, 2, 63, 37
//3, 55680, Note_off_c, 2, 63, 0
//3, 55968, Note_on_c, 2, 58, 37
//3, 56064, Note_off_c, 2, 58, 0
//3, 56064, Note_on_c, 2, 63, 37
//3, 56160, Note_off_c, 2, 63, 0
//3, 56160, Note_on_c, 2, 63, 37
//3, 56352, Note_off_c, 2, 63, 0
//3, 56352, Note_on_c, 2, 66, 37
//3, 56640, Note_off_c, 2, 66, 0
//3, 56640, Note_on_c, 2, 63, 37
//3, 56832, Note_off_c, 2, 63, 0
//3, 56832, Note_on_c, 2, 63, 37
//3, 57216, Note_off_c, 2, 63, 0
//3, 57600, Note_on_c, 2, 61, 37
//3, 57792, Note_off_c, 2, 61, 0
//3, 57792, Note_on_c, 2, 63, 37
//3, 57984, Note_off_c, 2, 63, 0
//3, 57984, Note_on_c, 2, 63, 37
//3, 58176, Note_off_c, 2, 63, 0
//3, 58176, Note_on_c, 2, 63, 37
//3, 58368, Note_off_c, 2, 63, 0
//3, 58368, Note_on_c, 2, 66, 37
//3, 58464, Note_off_c, 2, 66, 0
//3, 58464, Note_on_c, 2, 66, 37
//3, 58752, Note_off_c, 2, 66, 0
//3, 58752, Note_on_c, 2, 66, 37
//3, 58848, Note_off_c, 2, 66, 0
//3, 58848, Note_on_c, 2, 66, 37
//3, 59136, Note_off_c, 2, 66, 0
//3, 59136, Note_on_c, 2, 61, 37
//3, 59328, Note_off_c, 2, 61, 0
//3, 59328, Note_on_c, 2, 63, 37
//3, 59520, Note_off_c, 2, 63, 0
//3, 59520, Note_on_c, 2, 63, 37
//3, 59712, Note_off_c, 2, 63, 0
//3, 59712, Note_on_c, 2, 61, 37
//3, 59808, Note_off_c, 2, 61, 0
//3, 59808, Note_on_c, 2, 63, 37
//3, 60096, Note_off_c, 2, 63, 0
//3, 60096, Note_on_c, 2, 61, 37
//3, 60288, Note_off_c, 2, 61, 0
//3, 60288, Note_on_c, 2, 63, 37
//3, 60480, Note_off_c, 2, 63, 0
//3, 60480, Note_on_c, 2, 63, 37
//3, 60576, Note_off_c, 2, 63, 0
//3, 60576, Note_on_c, 2, 66, 37
//3, 60864, Note_off_c, 2, 66, 0
//3, 60864, Note_on_c, 2, 63, 37
//3, 61056, Note_off_c, 2, 63, 0
//3, 61440, Note_on_c, 2, 70, 37
//3, 61632, Note_off_c, 2, 70, 0
//3, 61632, Note_on_c, 2, 70, 37
//3, 61824, Note_off_c, 2, 70, 0
//3, 61824, Note_on_c, 2, 68, 37
//3, 61920, Note_off_c, 2, 68, 0
//3, 61920, Note_on_c, 2, 66, 37
//3, 62016, Note_off_c, 2, 66, 0
//3, 62016, Note_on_c, 2, 70, 37
//3, 62400, Note_off_c, 2, 70, 0
//3, 62400, Note_on_c, 2, 68, 37
//3, 62592, Note_off_c, 2, 68, 0
//3, 62784, Note_on_c, 2, 68, 37
//3, 63168, Note_off_c, 2, 68, 0
//3, 63168, Note_on_c, 2, 66, 37
//3, 63360, Note_off_c, 2, 66, 0
//3, 64032, Note_on_c, 2, 68, 37
//3, 64128, Note_off_c, 2, 68, 0
//3, 64800, Note_on_c, 2, 68, 37
//3, 64896, Note_off_c, 2, 68, 0
//3, 65280, Note_on_c, 2, 70, 37
//3, 65664, Note_off_c, 2, 70, 0
//3, 65952, Note_on_c, 2, 70, 37
//3, 66432, Note_off_c, 2, 70, 0
//3, 67008, Note_on_c, 2, 70, 37
//3, 67392, Note_off_c, 2, 70, 0
//3, 67776, Note_on_c, 2, 70, 37
//3, 68160, Note_off_c, 2, 70, 0
//3, 68448, Note_on_c, 2, 73, 37
//3, 68544, Note_off_c, 2, 73, 0
//3, 68544, Note_on_c, 2, 70, 37
//3, 68640, Note_off_c, 2, 70, 0
//3, 68640, Note_on_c, 2, 70, 37
//3, 68736, Note_off_c, 2, 70, 0
//3, 68736, Note_on_c, 2, 68, 37
//3, 69024, Note_off_c, 2, 68, 0
//3, 69024, Note_on_c, 2, 66, 37
//3, 69312, Note_off_c, 2, 66, 0
//3, 69312, Note_on_c, 2, 73, 37
//3, 69696, Note_off_c, 2, 73, 0
//3, 82176, Note_on_c, 2, 66, 37
//3, 82368, Note_off_c, 2, 66, 0
//3, 82368, Note_on_c, 2, 68, 37
//3, 82560, Note_off_c, 2, 68, 0
//3, 82560, Note_on_c, 2, 68, 37
//3, 82656, Note_off_c, 2, 68, 0
//3, 82656, Note_on_c, 2, 66, 37
//3, 82752, Note_off_c, 2, 66, 0
//3, 82752, Note_on_c, 2, 71, 37
//3, 83136, Note_off_c, 2, 71, 0
//3, 83136, Note_on_c, 2, 68, 37
//3, 83328, Note_off_c, 2, 68, 0
//3, 83520, Note_on_c, 2, 63, 37
//3, 83712, Note_off_c, 2, 63, 0
//3, 83712, Note_on_c, 2, 66, 37
//3, 83808, Note_off_c, 2, 66, 0
//3, 83808, Note_on_c, 2, 68, 37
//3, 84096, Note_off_c, 2, 68, 0
//3, 84096, Note_on_c, 2, 68, 37
//3, 84192, Note_off_c, 2, 68, 0
//3, 84192, Note_on_c, 2, 66, 37
//3, 84384, Note_off_c, 2, 66, 0
//3, 84384, Note_on_c, 2, 68, 37
//3, 84672, Note_off_c, 2, 68, 0
//3, 84672, Note_on_c, 2, 66, 37
//3, 84864, Note_off_c, 2, 66, 0
//3, 85056, Note_on_c, 2, 63, 37
//3, 85248, Note_off_c, 2, 63, 0
//3, 85248, Note_on_c, 2, 66, 37
//3, 85440, Note_off_c, 2, 66, 0
//3, 85440, Note_on_c, 2, 68, 37
//3, 85536, Note_off_c, 2, 68, 0
//3, 85536, Note_on_c, 2, 68, 37
//3, 85824, Note_off_c, 2, 68, 0
//3, 85824, Note_on_c, 2, 68, 37
//3, 85920, Note_off_c, 2, 68, 0
//3, 85920, Note_on_c, 2, 68, 37
//3, 86016, Note_off_c, 2, 68, 0
//3, 86016, Note_on_c, 2, 71, 37
//3, 86208, Note_off_c, 2, 71, 0
//3, 86208, Note_on_c, 2, 68, 37
//3, 86304, Note_off_c, 2, 68, 0
//3, 86304, Note_on_c, 2, 71, 37
//3, 86592, Note_off_c, 2, 71, 0
//3, 86592, Note_on_c, 2, 63, 37
//3, 86688, Note_off_c, 2, 63, 0
//3, 86688, Note_on_c, 2, 63, 37
//3, 86784, Note_off_c, 2, 63, 0
//3, 86784, Note_on_c, 2, 66, 37
//3, 86976, Note_off_c, 2, 66, 0
//3, 86976, Note_on_c, 2, 68, 37
//3, 87072, Note_off_c, 2, 68, 0
//3, 87072, Note_on_c, 2, 68, 37
//3, 87168, Note_off_c, 2, 68, 0
//3, 87168, Note_on_c, 2, 68, 37
//3, 87360, Note_off_c, 2, 68, 0
//3, 87360, Note_on_c, 2, 66, 37
//3, 87456, Note_off_c, 2, 66, 0
//3, 87456, Note_on_c, 2, 68, 37
//3, 87744, Note_off_c, 2, 68, 0
//3, 87744, Note_on_c, 2, 68, 37
//3, 87936, Note_off_c, 2, 68, 0
//3, 87936, Note_on_c, 2, 68, 37
//3, 88064, Note_off_c, 2, 68, 0
//3, 88064, Note_on_c, 2, 68, 37
//3, 88193, Note_on_c, 2, 68, 37
//3, 88193, Note_off_c, 2, 68, 0
//3, 88321, Note_off_c, 2, 68, 0
//3, 88321, Note_on_c, 2, 63, 37
//3, 88417, Note_off_c, 2, 63, 0
//3, 88417, Note_on_c, 2, 63, 37
//3, 88609, Note_off_c, 2, 63, 0
//3, 88609, Note_on_c, 2, 63, 37
//3, 88801, Note_off_c, 2, 63, 0
//3, 88801, Note_on_c, 2, 61, 37
//3, 88993, Note_off_c, 2, 61, 0
//3, 88993, Note_on_c, 2, 63, 37
//3, 89473, Note_off_c, 2, 63, 0
//3, 89761, Note_on_c, 2, 58, 37
//3, 89857, Note_off_c, 2, 58, 0
//3, 89857, Note_on_c, 2, 63, 37
//3, 89953, Note_off_c, 2, 63, 0
//3, 89953, Note_on_c, 2, 63, 37
//3, 90145, Note_off_c, 2, 63, 0
//3, 90145, Note_on_c, 2, 66, 37
//3, 90433, Note_off_c, 2, 66, 0
//3, 90433, Note_on_c, 2, 63, 37
//3, 90625, Note_off_c, 2, 63, 0
//3, 90625, Note_on_c, 2, 63, 37
//3, 91009, Note_off_c, 2, 63, 0
//3, 91393, Note_on_c, 2, 61, 37
//3, 91585, Note_off_c, 2, 61, 0
//3, 91585, Note_on_c, 2, 63, 37
//3, 91777, Note_off_c, 2, 63, 0
//3, 91777, Note_on_c, 2, 63, 37
//3, 91969, Note_off_c, 2, 63, 0
//3, 91969, Note_on_c, 2, 63, 37
//3, 92161, Note_off_c, 2, 63, 0
//3, 92161, Note_on_c, 2, 66, 37
//3, 92257, Note_off_c, 2, 66, 0
//3, 92257, Note_on_c, 2, 66, 37
//3, 92545, Note_off_c, 2, 66, 0
//3, 92545, Note_on_c, 2, 66, 37
//3, 92641, Note_off_c, 2, 66, 0
//3, 92641, Note_on_c, 2, 66, 37
//3, 92929, Note_off_c, 2, 66, 0
//3, 92929, Note_on_c, 2, 61, 37
//3, 93121, Note_off_c, 2, 61, 0
//3, 93121, Note_on_c, 2, 63, 37
//3, 93313, Note_off_c, 2, 63, 0
//3, 93313, Note_on_c, 2, 63, 37
//3, 93505, Note_off_c, 2, 63, 0
//3, 93505, Note_on_c, 2, 61, 37
//3, 93601, Note_off_c, 2, 61, 0
//3, 93601, Note_on_c, 2, 63, 37
//3, 93889, Note_off_c, 2, 63, 0
//3, 93889, Note_on_c, 2, 61, 37
//3, 94081, Note_off_c, 2, 61, 0
//3, 94081, Note_on_c, 2, 63, 37
//3, 94273, Note_off_c, 2, 63, 0
//3, 94273, Note_on_c, 2, 63, 37
//3, 94369, Note_off_c, 2, 63, 0
//3, 94369, Note_on_c, 2, 66, 37
//3, 94657, Note_off_c, 2, 66, 0
//3, 94657, Note_on_c, 2, 63, 37
//3, 94849, Note_off_c, 2, 63, 0
//3, 95233, Note_on_c, 2, 70, 37
//3, 95425, Note_off_c, 2, 70, 0
//3, 95425, Note_on_c, 2, 70, 37
//3, 95617, Note_off_c, 2, 70, 0
//3, 95617, Note_on_c, 2, 68, 37
//3, 95713, Note_off_c, 2, 68, 0
//3, 95713, Note_on_c, 2, 66, 37
//3, 95809, Note_off_c, 2, 66, 0
//3, 95809, Note_on_c, 2, 70, 37
//3, 96193, Note_off_c, 2, 70, 0
//3, 96193, Note_on_c, 2, 68, 37
//3, 96385, Note_off_c, 2, 68, 0
//3, 96577, Note_on_c, 2, 68, 37
//3, 96961, Note_off_c, 2, 68, 0
//3, 96961, Note_on_c, 2, 66, 37
//3, 97153, Note_off_c, 2, 66, 0
//3, 97921, Note_on_c, 2, 68, 37
//3, 98017, Note_off_c, 2, 68, 0
//3, 98017, Note_on_c, 2, 68, 37
//3, 98113, Note_off_c, 2, 68, 0
//3, 98113, Note_on_c, 2, 68, 37
//3, 98209, Note_off_c, 2, 68, 0
//3, 98209, Note_on_c, 2, 68, 37
//3, 98305, Note_off_c, 2, 68, 0
//3, 98305, Note_on_c, 2, 68, 37
//3, 98401, Note_off_c, 2, 68, 0
//3, 98401, Note_on_c, 2, 68, 37
//3, 98497, Note_off_c, 2, 68, 0
//3, 98497, Note_on_c, 2, 68, 37
//3, 98593, Note_off_c, 2, 68, 0
//3, 98593, Note_on_c, 2, 68, 37
//3, 98689, Note_off_c, 2, 68, 0
//3, 98689, Note_on_c, 2, 68, 37
//3, 98785, Note_off_c, 2, 68, 0
//3, 98785, Note_on_c, 2, 68, 37
//3, 98881, Note_off_c, 2, 68, 0
//3, 98881, Note_on_c, 2, 68, 37
//3, 98977, Note_off_c, 2, 68, 0
//3, 98977, Note_on_c, 2, 68, 37
//3, 99073, Note_off_c, 2, 68, 0
//3, 99073, Note_on_c, 2, 70, 37
//3, 99457, Note_off_c, 2, 70, 0
//3, 99745, Note_on_c, 2, 70, 37
//3, 100225, Note_off_c, 2, 70, 0
//3, 100801, Note_on_c, 2, 73, 37
//3, 101377, Note_off_c, 2, 73, 0
//3, 102145, Note_on_c, 2, 70, 37
//3, 102337, Note_off_c, 2, 70, 0
//3, 102337, Note_on_c, 2, 68, 37
//3, 102433, Note_off_c, 2, 68, 0
//3, 102433, Note_on_c, 2, 68, 37
//3, 102529, Note_off_c, 2, 68, 0
//3, 102529, Note_on_c, 2, 66, 37
//3, 102721, Note_off_c, 2, 66, 0
//3, 102721, Note_on_c, 2, 66, 37
//3, 102913, Note_off_c, 2, 66, 0
//3, 102913, Note_on_c, 2, 70, 37
//3, 103297, Note_off_c, 2, 70, 0
//3, 110401, Note_on_c, 2, 60, 37
//3, 110593, Note_off_c, 2, 60, 0
//3, 110593, Note_on_c, 2, 65, 37
//3, 110785, Note_off_c, 2, 65, 0
//3, 110785, Note_on_c, 2, 68, 37
//3, 111169, Note_off_c, 2, 68, 0
//3, 111169, Note_on_c, 2, 66, 37
//3, 111361, Note_off_c, 2, 66, 0
//3, 111361, Note_on_c, 2, 63, 37
//3, 111745, Note_off_c, 2, 63, 0
//3, 111745, Note_on_c, 2, 61, 37
//3, 111937, Note_off_c, 2, 61, 0
//3, 111937, Note_on_c, 2, 63, 37
//3, 112033, Note_off_c, 2, 63, 0
//3, 112033, Note_on_c, 2, 61, 37
//3, 112321, Note_off_c, 2, 61, 0
//3, 112321, Note_on_c, 2, 61, 37
//3, 112705, Note_off_c, 2, 61, 0
//3, 113281, Note_on_c, 2, 65, 37
//3, 113473, Note_off_c, 2, 65, 0
//3, 113473, Note_on_c, 2, 63, 37
//3, 113665, Note_off_c, 2, 63, 0
//3, 113665, Note_on_c, 2, 65, 37
//3, 113857, Note_off_c, 2, 65, 0
//3, 113857, Note_on_c, 2, 63, 37
//3, 114049, Note_off_c, 2, 63, 0
//3, 114049, Note_on_c, 2, 61, 37
//3, 114241, Note_off_c, 2, 61, 0
//3, 114241, Note_on_c, 2, 61, 37
//3, 115201, Note_off_c, 2, 61, 0
//3, 115585, Note_on_c, 2, 59, 37
//3, 115777, Note_off_c, 2, 59, 0
//3, 115969, Note_on_c, 2, 59, 37
//3, 116161, Note_off_c, 2, 59, 0
//3, 116161, Note_on_c, 2, 58, 37
//3, 116737, Note_off_c, 2, 58, 0
//3, 117121, Note_on_c, 2, 59, 37
//3, 117313, Note_off_c, 2, 59, 0
//3, 117505, Note_on_c, 2, 59, 37
//3, 117697, Note_off_c, 2, 59, 0
//3, 117697, Note_on_c, 2, 58, 37
//3, 118273, Note_off_c, 2, 58, 0
//3, 118657, Note_on_c, 2, 59, 37
//3, 118849, Note_off_c, 2, 59, 0
//3, 119041, Note_on_c, 2, 63, 37
//3, 119809, Note_off_c, 2, 63, 0
//3, 119809, Note_on_c, 2, 68, 37
//3, 120193, Note_off_c, 2, 68, 0
//3, 120577, Note_on_c, 2, 75, 37
//3, 122113, Note_off_c, 2, 75, 0
//3, 122113, Note_on_c, 2, 75, 37
//3, 122881, Note_off_c, 2, 75, 0
//3, 122881, Note_on_c, 2, 75, 37
//3, 123073, Note_off_c, 2, 75, 0
//3, 123073, Note_on_c, 2, 70, 37
//3, 123265, Note_off_c, 2, 70, 0
//3, 123265, Note_on_c, 2, 70, 37
//3, 123457, Note_off_c, 2, 70, 0
//3, 123457, Note_on_c, 2, 75, 37
//3, 123649, Note_off_c, 2, 75, 0
//3, 123649, Note_on_c, 2, 70, 37
//3, 123841, Note_off_c, 2, 70, 0
//3, 123841, Note_on_c, 2, 73, 37
//3, 124033, Note_off_c, 2, 73, 0
//3, 124033, Note_on_c, 2, 70, 37
//3, 124225, Note_off_c, 2, 70, 0
//3, 124225, Note_on_c, 2, 70, 37
//3, 124417, Note_off_c, 2, 70, 0
//3, 124417, Note_on_c, 2, 68, 37
//3, 124609, Note_off_c, 2, 68, 0
//3, 124609, Note_on_c, 2, 66, 37
//3, 124801, Note_off_c, 2, 66, 0
//3, 124801, Note_on_c, 2, 70, 37
//3, 124993, Note_off_c, 2, 70, 0
//3, 124993, Note_on_c, 2, 70, 37
//3, 125185, Note_off_c, 2, 70, 0
//3, 125185, Note_on_c, 2, 68, 37
//3, 125377, Note_off_c, 2, 68, 0
//3, 125377, Note_on_c, 2, 66, 37
//3, 125569, Note_off_c, 2, 66, 0
//3, 125569, Note_on_c, 2, 70, 37
//3, 125761, Note_off_c, 2, 70, 0
//3, 125761, Note_on_c, 2, 70, 37
//3, 125953, Note_off_c, 2, 70, 0
//3, 125953, Note_on_c, 2, 68, 37
//3, 126145, Note_off_c, 2, 68, 0
//3, 126337, Note_on_c, 2, 70, 37
//3, 126721, Note_off_c, 2, 70, 0
//3, 166849, Note_on_c, 2, 73, 37
//3, 167041, Note_off_c, 2, 73, 0
//3, 167041, Note_on_c, 2, 73, 37
//3, 167169, Note_on_c, 2, 73, 37
//3, 167169, Note_off_c, 2, 73, 0
//3, 167298, Note_off_c, 2, 73, 0
//3, 167298, Note_on_c, 2, 73, 37
//3, 167426, Note_on_c, 2, 73, 37
//3, 167426, Note_off_c, 2, 73, 0
//3, 167810, Note_off_c, 2, 73, 0
//3, 168578, Note_on_c, 2, 71, 37
//3, 168706, Note_on_c, 2, 73, 37
//3, 168706, Note_off_c, 2, 71, 0
//3, 168835, Note_off_c, 2, 73, 0
//3, 168835, Note_on_c, 2, 70, 37
//3, 168963, Note_on_c, 2, 75, 37
//3, 168963, Note_off_c, 2, 70, 0
//3, 169155, Note_off_c, 2, 75, 0
//3, 169155, Note_on_c, 2, 68, 37
//3, 169347, Note_off_c, 2, 68, 0
//3, 169347, Note_on_c, 2, 70, 37
//3, 169539, Note_off_c, 2, 70, 0
//3, 169539, Note_on_c, 2, 73, 37
//3, 169731, Note_off_c, 2, 73, 0
//3, 170691, Note_on_c, 2, 73, 37
//3, 170883, Note_off_c, 2, 73, 0
//3, 170883, Note_on_c, 2, 73, 37
//3, 171075, Note_off_c, 2, 73, 0
//3, 171075, Note_on_c, 2, 70, 37
//3, 171267, Note_off_c, 2, 70, 0
//3, 171267, Note_on_c, 2, 73, 37
//3, 171651, Note_off_c, 2, 73, 0
//3, 171651, Note_on_c, 2, 66, 37
//3, 175107, Note_off_c, 2, 66, 0
//3, 175107, Note_on_c, 2, 73, 37
//3, 175491, Note_off_c, 2, 73, 0
//3, 175491, Note_on_c, 2, 71, 37
//3, 175587, Note_off_c, 2, 71, 0
//3, 175587, Note_on_c, 2, 70, 37
//3, 175779, Note_off_c, 2, 70, 0
//3, 175779, Note_on_c, 2, 73, 37
//3, 176067, Note_off_c, 2, 73, 0
//3, 176067, Note_on_c, 2, 71, 37
//3, 176259, Note_off_c, 2, 71, 0
//3, 176643, Note_on_c, 2, 75, 37
//3, 176835, Note_off_c, 2, 75, 0
//3, 176835, Note_on_c, 2, 75, 37
//3, 177027, Note_off_c, 2, 75, 0
//3, 177027, Note_on_c, 2, 73, 37
//3, 177123, Note_off_c, 2, 73, 0
//3, 177123, Note_on_c, 2, 71, 37
//3, 177219, Note_off_c, 2, 71, 0
//3, 177219, Note_on_c, 2, 75, 37
//3, 177603, Note_off_c, 2, 75, 0
//3, 177603, Note_on_c, 2, 73, 37
//3, 177795, Note_off_c, 2, 73, 0
//3, 177795, Note_on_c, 2, 73, 37
//3, 177987, Note_off_c, 2, 73, 0
//3, 177987, Note_on_c, 2, 72, 37
//3, 178179, Note_off_c, 2, 72, 0
//3, 178179, Note_on_c, 2, 73, 37
//3, 178371, Note_off_c, 2, 73, 0
//3, 178371, Note_on_c, 2, 72, 37
//3, 178563, Note_off_c, 2, 72, 0
//3, 178563, Note_on_c, 2, 73, 37
//3, 178755, Note_off_c, 2, 73, 0
//3, 178755, Note_on_c, 2, 72, 37
//3, 178851, Note_off_c, 2, 72, 0
//3, 179331, Note_on_c, 2, 73, 37
//3, 179427, Note_off_c, 2, 73, 0
//3, 179427, Note_on_c, 2, 73, 37
//3, 179523, Note_off_c, 2, 73, 0
//3, 179523, Note_on_c, 2, 73, 37
//3, 179619, Note_off_c, 2, 73, 0
//3, 179619, Note_on_c, 2, 73, 37
//3, 179715, Note_off_c, 2, 73, 0
//3, 179715, Note_on_c, 2, 73, 37
//3, 179811, Note_off_c, 2, 73, 0
//3, 179811, Note_on_c, 2, 73, 37
//3, 179907, Note_off_c, 2, 73, 0
//3, 179907, Note_on_c, 2, 73, 37
//3, 180003, Note_off_c, 2, 73, 0
//3, 180003, Note_on_c, 2, 73, 37
//3, 180099, Note_off_c, 2, 73, 0
//3, 180099, Note_on_c, 2, 73, 37
//3, 180195, Note_off_c, 2, 73, 0
//3, 180195, Note_on_c, 2, 73, 37
//3, 180291, Note_off_c, 2, 73, 0
//3, 180291, Note_on_c, 2, 73, 37
//3, 180387, Note_off_c, 2, 73, 0
//3, 180387, Note_on_c, 2, 73, 37
//3, 180483, Note_off_c, 2, 73, 0
//3, 180483, Note_on_c, 2, 75, 37
//3, 180867, Note_off_c, 2, 75, 0
//3, 181155, Note_on_c, 2, 75, 37
//3, 181635, Note_off_c, 2, 75, 0
//3, 181635, Note_on_c, 2, 68, 37
//3, 181731, Note_off_c, 2, 68, 0
//3, 181731, Note_on_c, 2, 66, 37
//3, 181923, Note_off_c, 2, 66, 0
//3, 181923, Note_on_c, 2, 68, 37
//3, 182211, Note_off_c, 2, 68, 0
//3, 182211, Note_on_c, 2, 66, 37
//3, 182403, Note_off_c, 2, 66, 0
//3, 182787, Note_on_c, 2, 70, 37
//3, 182979, Note_off_c, 2, 70, 0
//3, 182979, Note_on_c, 2, 70, 37
//3, 183171, Note_off_c, 2, 70, 0
//3, 183171, Note_on_c, 2, 70, 37
//3, 183267, Note_off_c, 2, 70, 0
//3, 183267, Note_on_c, 2, 73, 37
//3, 183363, Note_off_c, 2, 73, 0
//3, 183363, Note_on_c, 2, 70, 37
//3, 183747, Note_off_c, 2, 70, 0
//3, 183747, Note_on_c, 2, 68, 37
//3, 183939, Note_off_c, 2, 68, 0
//3, 184131, Note_on_c, 2, 68, 37
//3, 184515, Note_off_c, 2, 68, 0
//3, 184515, Note_on_c, 2, 66, 37
//3, 184707, Note_off_c, 2, 66, 0
//3, 185187, Note_on_c, 2, 70, 37
//3, 185475, Note_off_c, 2, 70, 0
//3, 185475, Note_on_c, 2, 70, 37
//3, 185667, Note_off_c, 2, 70, 0
//3, 186147, Note_on_c, 2, 70, 37
//3, 186435, Note_off_c, 2, 70, 0
//3, 186819, Note_on_c, 2, 69, 37
//3, 187011, Note_off_c, 2, 69, 0
//3, 187011, Note_on_c, 2, 68, 37
//3, 187107, Note_off_c, 2, 68, 0
//3, 187107, Note_on_c, 2, 66, 37
//3, 187299, Note_off_c, 2, 66, 0
//3, 187299, Note_on_c, 2, 70, 37
//3, 187779, Note_off_c, 2, 70, 0
//3, 188163, Note_on_c, 2, 68, 37
//3, 188355, Note_off_c, 2, 68, 0
//3, 188355, Note_on_c, 2, 66, 37
//3, 188547, Note_off_c, 2, 66, 0
//3, 188931, Note_on_c, 2, 70, 37
//3, 189123, Note_off_c, 2, 70, 0
//3, 189123, Note_on_c, 2, 70, 37
//3, 189315, Note_off_c, 2, 70, 0
//3, 189315, Note_on_c, 2, 70, 37
//3, 189411, Note_off_c, 2, 70, 0
//3, 189411, Note_on_c, 2, 73, 37
//3, 189507, Note_off_c, 2, 73, 0
//3, 189507, Note_on_c, 2, 70, 37
//3, 189891, Note_off_c, 2, 70, 0
//3, 189891, Note_on_c, 2, 68, 37
//3, 190083, Note_off_c, 2, 68, 0
//3, 190083, Note_on_c, 2, 68, 37
//3, 190275, Note_off_c, 2, 68, 0
//3, 190275, Note_on_c, 2, 66, 37
//3, 190467, Note_off_c, 2, 66, 0
//3, 190467, Note_on_c, 2, 68, 37
//3, 190659, Note_off_c, 2, 68, 0
//3, 190659, Note_on_c, 2, 66, 37
//3, 190851, Note_off_c, 2, 66, 0
//3, 190851, Note_on_c, 2, 68, 37
//3, 191043, Note_off_c, 2, 68, 0
//3, 191043, Note_on_c, 2, 66, 37
//3, 191235, Note_off_c, 2, 66, 0
//3, 191619, Note_on_c, 2, 68, 37
//3, 191715, Note_off_c, 2, 68, 0
//3, 191715, Note_on_c, 2, 68, 37
//3, 191811, Note_off_c, 2, 68, 0
//3, 191811, Note_on_c, 2, 68, 37
//3, 191907, Note_off_c, 2, 68, 0
//3, 191907, Note_on_c, 2, 68, 37
//3, 192003, Note_off_c, 2, 68, 0
//3, 192003, Note_on_c, 2, 68, 37
//3, 192099, Note_off_c, 2, 68, 0
//3, 192099, Note_on_c, 2, 68, 37
//3, 192195, Note_off_c, 2, 68, 0
//3, 192195, Note_on_c, 2, 68, 37
//3, 192291, Note_off_c, 2, 68, 0
//3, 192291, Note_on_c, 2, 68, 37
//3, 192387, Note_off_c, 2, 68, 0
//3, 192387, Note_on_c, 2, 68, 37
//3, 192483, Note_off_c, 2, 68, 0
//3, 192483, Note_on_c, 2, 68, 37
//3, 192579, Note_off_c, 2, 68, 0
//3, 192579, Note_on_c, 2, 68, 37
//3, 192675, Note_off_c, 2, 68, 0
//3, 192675, Note_on_c, 2, 68, 37
//3, 192771, Note_off_c, 2, 68, 0
//3, 192771, Note_on_c, 2, 70, 37
//3, 193155, Note_off_c, 2, 70, 0
//3, 193443, Note_on_c, 2, 70, 37
//3, 193731, Note_off_c, 2, 70, 0
//3, 193731, Note_on_c, 2, 70, 37
//3, 193923, Note_off_c, 2, 70, 0
//3, 193923, Note_on_c, 2, 68, 37
//3, 194019, Note_off_c, 2, 68, 0
//3, 194019, Note_on_c, 2, 66, 37
//3, 194211, Note_off_c, 2, 66, 0
//3, 194211, Note_on_c, 2, 68, 37
//3, 194499, Note_off_c, 2, 68, 0
//3, 194499, Note_on_c, 2, 66, 37
//3, 194691, Note_off_c, 2, 66, 0
//3, 195075, Note_on_c, 2, 70, 37
//3, 195267, Note_off_c, 2, 70, 0
//3, 195267, Note_on_c, 2, 70, 37
//3, 195459, Note_off_c, 2, 70, 0
//3, 195459, Note_on_c, 2, 70, 37
//3, 195555, Note_off_c, 2, 70, 0
//3, 195555, Note_on_c, 2, 73, 37
//3, 195651, Note_off_c, 2, 73, 0
//3, 195651, Note_on_c, 2, 70, 37
//3, 196035, Note_off_c, 2, 70, 0
//3, 196035, Note_on_c, 2, 68, 37
//3, 196227, Note_off_c, 2, 68, 0
//3, 196227, Note_on_c, 2, 68, 37
//3, 196419, Note_off_c, 2, 68, 0
//3, 196419, Note_on_c, 2, 66, 37
//3, 196611, Note_off_c, 2, 66, 0
//3, 196611, Note_on_c, 2, 68, 37
//3, 196803, Note_off_c, 2, 68, 0
//3, 196803, Note_on_c, 2, 66, 37
//3, 196995, Note_off_c, 2, 66, 0
//3, 196995, Note_on_c, 2, 68, 37
//3, 197187, Note_off_c, 2, 68, 0
//3, 197187, Note_on_c, 2, 66, 37
//3, 197379, Note_off_c, 2, 66, 0
//3, 197667, Note_on_c, 2, 75, 37
//3, 197859, Note_off_c, 2, 75, 0
//3, 197859, Note_on_c, 2, 75, 37
//3, 197955, Note_off_c, 2, 75, 0
//3, 197955, Note_on_c, 2, 70, 37
//3, 198243, Note_off_c, 2, 70, 0
//3, 198243, Note_on_c, 2, 73, 37
//3, 198531, Note_off_c, 2, 73, 0
//3, 198531, Note_on_c, 2, 70, 37
//3, 198915, Note_off_c, 2, 70, 0
//3, 199011, Note_on_c, 2, 72, 37
//3, 199299, Note_off_c, 2, 72, 0
//3, 199875, Note_on_c, 2, 73, 37
//3, 200067, Note_off_c, 2, 73, 0
//3, 200067, End_track
