use jungle_sdk::prelude::*;
use std::time::Duration;

use crate::effect::{DecrementCounterEffect, Monad};
use crate::instrumentation::{
    Cymbal, CymbalArticulation, HiHat, HiHatArticulation, KickDrum, KickDrumArticulation,
    SnareDrum, SnareDrumArticulation,
};

pub type DrummerState = ();
pub type DrummerSeed = ();
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

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        Duration::from_millis(INTRO_START_DELAY_MS)
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("intro start delay should complete");
    }
}

pub struct MergeJoinUnits;
#[jungle::act]
impl Act for MergeJoinUnits {
    type Effect = DecrementCounterEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("join merge should succeed");
    }
}

#[derive(Flow)]
pub struct SyncPair<Left, Right>(Join<Left, Right>, Step<MergeJoinUnits>);

pub struct Hat<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Hat<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<HiHat, HiHatArticulation, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        HiHatArticulation::ClosedTip
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("hi-hat playback should succeed");
    }
}

pub struct Boot<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Boot<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<KickDrum, KickDrumArticulation, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        KickDrumArticulation::StandardHit
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("kick playback should succeed");
    }
}

pub struct Snap<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Snap<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<SnareDrum, SnareDrumArticulation, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SnareDrumArticulation::Rimshot
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("snare playback should succeed");
    }
}

pub struct Blast<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Blast<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<Cymbal, CymbalArticulation, NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(_state: &DrummerState, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        CymbalArticulation::StandardCrash
    }

    fn absorb(_state: &mut DrummerState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cymbal playback should succeed");
    }
}

#[derive(Flow)]
pub struct IntroPart01(
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 96, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart02(
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<44, 96, 96>>, Step<Boot<36, 96, 96>>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart03(
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<44, 96, 96>>, Step<Boot<36, 96, 96>>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart04(
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<44, 96, 96>>, Step<Boot<36, 96, 96>>>>,
);

#[derive(Flow)]
pub struct IntroPart05(
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
);

#[derive(Flow)]
pub struct IntroPart06(
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 96, 96>>, Step<Blast<57, 96, 96>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart07(
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 96, 96>>, Step<Boot<36, 96, 96>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart08(
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 96, 96>>, Step<Boot<36, 96, 96>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
);

#[derive(Flow)]
pub struct IntroPart09(
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 96, 96>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
);

#[derive(Flow)]
pub struct IntroPart10(
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Hat<44, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Snap<38, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
);

#[derive(Flow)]
pub struct IntroPart11(
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<35, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Snap<38, 48, 48>>>,
    Transparent<IntroSectionMeta, Step<Snap<38, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Boot<36, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Snap<38, 48, 48>>>,
);

#[derive(Flow)]
pub struct IntroPart12(
    Transparent<IntroSectionMeta, Step<Snap<38, 192, 192>>>,
    Transparent<IntroSectionMeta, Step<Snap<38, 192, 96>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Boot<36, 192, 192>>, Step<Blast<57, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
);

#[derive(Flow)]
pub struct IntroPart13(
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
    Transparent<IntroSectionMeta, Step<Hat<46, 192, 192>>>,
);

#[derive(Flow)]
pub struct IntroPart14(
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Snap<38, 192, 192>>>>,
    Transparent<IntroSectionMeta, SyncPair<Step<Hat<46, 192, 192>>, Step<Boot<36, 192, 192>>>>,
);

#[derive(Flow)]
pub struct DrummerIntro(
    Transparent<IntroSectionMeta, Step<IntroStartDelay>>,
    Transparent<IntroSectionMeta, IntroPart01>,
    Transparent<IntroSectionMeta, IntroPart02>,
    Transparent<IntroSectionMeta, IntroPart03>,
    Transparent<IntroSectionMeta, IntroPart04>,
    Transparent<IntroSectionMeta, IntroPart05>,
    Transparent<IntroSectionMeta, IntroPart06>,
    Transparent<IntroSectionMeta, IntroPart07>,
    Transparent<IntroSectionMeta, IntroPart08>,
    Transparent<IntroSectionMeta, IntroPart09>,
    Transparent<IntroSectionMeta, IntroPart10>,
    Transparent<IntroSectionMeta, IntroPart11>,
    Transparent<IntroSectionMeta, IntroPart12>,
    Transparent<IntroSectionMeta, IntroPart13>,
    Transparent<IntroSectionMeta, IntroPart14>,
);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jungle_sdk::core::JungleWorker;
    use jungle_sdk::prelude::JourneyStatus;
    use jungle_sdk::{JungleClient, LocalClient};

    use super::super::Drums;
    use crate::ecosystem::TheJungle;

    #[tokio::test]
    async fn intro_journey_runs_to_completion_end_to_end() {
        let client = LocalClient::builder()
            .namespace("welcome-drums-intro-test")
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
            .start_journey::<Drums>(seed)
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
