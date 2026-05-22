use jungle_sdk::prelude::*;

use crate::effect::{DecrementCounterEffect, Monad};
use crate::instrumentation::{
    Cymbal, CymbalArticulation, HiHat, HiHatArticulation, KickDrum, KickDrumArticulation,
    SnareDrum, SnareDrumArticulation,
};

pub type DrummerState = ();
pub type DrummerSeed = ();

pub struct IntroSectionMeta;
impl NodeMetadata for IntroSectionMeta {
    const METADATA: &'static str = "section";
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
