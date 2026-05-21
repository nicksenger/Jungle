use jungle_sdk::prelude::*;

use crate::ecosystem::TheJungle;
use crate::instrumentation::{
    ElectricGuitar, ElectricGuitarArticulation, Instrument, Note, Pick, Pluck, Strum,
};

const TICKS_PER_BEAT: u32 = 384;
const MIN_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(20);
const MAX_LATE_NOTE_DROP_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(120);
const RHYTHM_AMPLITUDE_MULTIPLIER: f32 = 0.5;
const RHYTHM_PAN: f32 = 0.5;
const RHYTHM_VELOCITY: f32 = 37.0 / 127.0;

pub type RhythmGuitaristState = ElectricGuitarArticulation;
pub type RhythmGuitaristSeed = ();

pub struct RhythmGuitarist;

#[jungle::animal(id = 2, generation = 0)]
impl Animal for RhythmGuitarist {
    type State = RhythmGuitaristState;
    type Seed = RhythmGuitaristSeed;
    type Journey = Intro;
}

#[derive(Flow)]
pub struct Intro(
    IntroPrelude,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroRiffCycle,
    IntroTransitionCycle,
    IntroTransitionCycle,
    IntroTransitionCycle,
    IntroSustainPairs,
    IntroCadence,
);

#[derive(Flow)]
pub struct IntroPrelude(PreludeRake, PreludeHold, IntroRiffCycle);

#[derive(Flow)]
pub struct PreludeRake(
    Step<Pluck<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct PreludeHold(
    Step<Pluck<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroRiffCycle(
    Step<Pluck<58, 96, 96>>,
    Step<Pick<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<56, 96, 96>>,
    Step<Pick<56, 96, 96>>,
    Step<Pluck<56, 96, 96>>,
    Step<Pluck<53, 96, 96>>,
    Step<Pick<53, 96, 96>>,
    Step<Pluck<53, 96, 96>>,
    Step<Pluck<51, 96, 96>>,
    Step<Pick<51, 96, 96>>,
    Step<Pluck<51, 96, 96>>,
    Step<Pluck<49, 96, 96>>,
    Step<Pick<49, 96, 96>>,
    Step<Strum<46, 46, 49, 96, 96>>,
    Step<Chord2<46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroTransitionCycle(
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<58, 96, 96>>,
    Step<Pluck<56, 96, 96>>,
    Step<Pluck<56, 96, 96>>,
    Step<Pluck<56, 96, 96>>,
    Step<Pluck<53, 96, 96>>,
    Step<Pluck<53, 96, 96>>,
    Step<Pluck<53, 96, 96>>,
    Step<Pluck<51, 96, 96>>,
    Step<Pluck<51, 96, 96>>,
    Step<Pluck<51, 96, 96>>,
    Step<Pluck<49, 96, 96>>,
    Step<Pluck<49, 96, 96>>,
    Step<Strum<46, 46, 49, 96, 96>>,
    Step<Strum<44, 46, 49, 96, 96>>,
);

#[derive(Flow)]
pub struct IntroSustainPairs(
    Step<Chord2Long<49, 56, 768, 768>>,
    Step<Chord2Long<44, 51, 768, 768>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 192, 192>>,
    Step<Chord2Long<46, 58, 384, 384>>,
    Step<Chord2Long<46, 58, 384, 384>>,
    Step<Chord2Long<44, 51, 384, 384>>,
    Step<PickLong<42, 192, 192>>,
);

#[derive(Flow)]
pub struct IntroCadence(
    Step<Chord2<44, 49, 96, 96>>,
    Step<Chord2<44, 49, 96, 192>>,
    Step<Chord2<49, 54, 96, 96>>,
    Step<PickLong<42, 96, 192>>,
    Step<PickLong<41, 96, 192>>,
    Step<PickLong<44, 192, 192>>,
    Step<Chord2Long<44, 51, 192, 192>>,
    Step<Chord2<44, 49, 96, 96>>,
    Step<Chord2<44, 49, 96, 96>>,
    Step<PickLong<42, 192, 192>>,
    Step<Chord2<44, 51, 96, 96>>,
    Step<Chord2Long<44, 51, 192, 192>>,
    Step<Chord2<49, 54, 96, 96>>,
    Step<PickLong<42, 96, 192>>,
    Step<PickLong<41, 96, 192>>,
    Step<PickLong<44, 192, 192>>,
    Step<Chord2Long<44, 51, 192, 192>>,
    Step<Chord2<44, 49, 96, 96>>,
    Step<Chord2<44, 49, 96, 96>>,
    Step<PickLong<42, 192, 192>>,
    Step<Chord2<44, 51, 96, 96>>,
    Step<Chord2Long<44, 51, 192, 192>>,
    Step<Chord2<49, 54, 96, 96>>,
    Step<PickLong<42, 96, 192>>,
    Step<PickLong<41, 96, 192>>,
    Step<PickLong<44, 192, 0>>,
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
    Step<Pluck<{ NOTE }, 96, 96>>,
    Step<Pick<{ NOTE }, 96, 96>>,
    Step<Pick<{ NOTE }, 96, 96>>,
);

pub struct Chord2<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Chord2<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect = crate::effect::Dyad<
        ElectricGuitar,
        ElectricGuitarArticulation,
        NOTE_1,
        NOTE_2,
        NOTE_TICK,
        REST_TICK,
    >;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct PickLong<const NOTE: u8, const NOTE_TICK: u16, const REST_TICK: u16>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u16, const REST_TICK: u16> Act
    for PickLong<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = LongMonad<NOTE, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct Chord2Long<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u16,
    const REST_TICK: u16,
>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u16, const REST_TICK: u16> Act
    for Chord2Long<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect = LongDyad<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        *state
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}

pub struct LongMonad<const NOTE: u8, const NOTE_TICK: u16, const REST_TICK: u16>;

#[jungle_sdk::effect(id = 507)]
impl<const NOTE: u8, const NOTE_TICK: u16, const REST_TICK: u16> Effect<TheJungle>
    for LongMonad<NOTE, NOTE_TICK, REST_TICK>
{
    type In = ElectricGuitarArticulation;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = long_rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play {
            let note = rhythm_note::<NOTE>(timing.note_duration, articulation);
            jungle
                .rhythm_guitar()
                .play(note)
                .await
                .map_err(|err| err.to_string())?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

pub struct LongDyad<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u16, const REST_TICK: u16>;

#[jungle_sdk::effect(id = 508)]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u16, const REST_TICK: u16>
    Effect<TheJungle> for LongDyad<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type In = ElectricGuitarArticulation;
    type Out = ();
    type Err = String;

    async fn effect(jungle: &TheJungle, articulation: Self::In) -> Result<Self::Out, Self::Err> {
        let timing = long_rhythm_timing(jungle, NOTE_TICK, REST_TICK);
        if timing.should_play {
            let note_1 = rhythm_note::<NOTE_1>(timing.note_duration, articulation);
            let note_2 = rhythm_note::<NOTE_2>(timing.note_duration, articulation);
            let (first, second) = tokio::join!(
                jungle.rhythm_guitar().play(note_1),
                jungle.rhythm_guitar().play(note_2)
            );
            first.map_err(|err| err.to_string())?;
            second.map_err(|err| err.to_string())?;
        }
        timing.sleep_until_next_cycle().await;
        Ok(())
    }
}

fn rhythm_note<const NOTE: u8>(
    duration: std::time::Duration,
    articulation: ElectricGuitarArticulation,
) -> Note<ElectricGuitarArticulation> {
    Note {
        n_midi: NOTE,
        amplitude_multiplier: RHYTHM_AMPLITUDE_MULTIPLIER,
        pan: RHYTHM_PAN,
        duration,
        velocity: RHYTHM_VELOCITY,
        expression: None,
        articulation,
    }
}

struct LongRhythmTiming {
    note_duration: std::time::Duration,
    rest_duration: std::time::Duration,
    phase_offset: std::time::Duration,
    should_play: bool,
}

impl LongRhythmTiming {
    async fn sleep_until_next_cycle(&self) {
        let sleep_for = self.rest_duration.saturating_sub(self.phase_offset);
        if !sleep_for.is_zero() {
            tokio::time::sleep(sleep_for).await;
        }
    }
}

fn long_rhythm_timing(jungle: &TheJungle, note_ticks: u16, rest_ticks: u16) -> LongRhythmTiming {
    let metronome = jungle.metronome();
    let note_duration = metronome.duration_for_ticks(TICKS_PER_BEAT, note_ticks as u32);
    let rest_duration = metronome.duration_for_ticks(TICKS_PER_BEAT, rest_ticks as u32);
    let phase_offset = metronome.phase_offset(rest_duration);
    let late_note_drop_threshold = metronome
        .late_note_drop_threshold(MIN_LATE_NOTE_DROP_THRESHOLD, MAX_LATE_NOTE_DROP_THRESHOLD);

    LongRhythmTiming {
        note_duration,
        rest_duration,
        phase_offset,
        should_play: phase_offset <= late_note_drop_threshold,
    }
}

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
