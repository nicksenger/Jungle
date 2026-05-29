use jungle_sdk::prelude::*;
use welcome_audio::{PlayPriority, PlayRequest};

use crate::effect::{Sound, Rest};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct ElectricGuitar {
    audio: welcome_audio::AudioHandle,
    synth: SynthHandle,
}

impl ElectricGuitar {
    pub fn new(audio: welcome_audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }

    pub fn audio(&self) -> &welcome_audio::AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ElectricGuitarArticulation {
    /// Standard picked note with normal sustain and release.
    Sustained,
    /// A sustained rhythm-guitar chord voice.
    RhythmSustained,
}

impl Default for ElectricGuitarArticulation {
    fn default() -> Self {
        Self::RhythmSustained
    }
}

impl Instrument for ElectricGuitar {
    type Articulation = ElectricGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate, pan) = self.synth.electric_guitar(note).await?;

        let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = pan;
        request.priority = PlayPriority::Low;

        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}

pub struct Pick<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Act
    for Pick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<ElectricGuitar, LANE_ID, NOTE, NOTE_TICK, REST_TICK>;
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

pub struct MergeUnit;
#[jungle::act]
impl Act for MergeUnit {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("join merge should complete");
    }
}

pub struct PostMergeRest<const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::act]
impl<const REST_TICK: u32, const LANE_ID: u8> Act for PostMergeRest<REST_TICK, LANE_ID> {
    type Effect = Rest<LANE_ID, REST_TICK>;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        ()
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("post-merge rest should complete");
    }
}

#[derive(Flow)]
pub struct Pluck<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u8,
>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0, LANE_ID>>, Step<Pick<NOTE_2, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);

#[derive(Flow)]
pub struct StrumPair<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const LANE_ID: u8>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0, LANE_ID>>, Step<Pick<NOTE_2, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit>,
);

#[derive(Flow)]
pub struct Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u32,
    const REST_TICK: u32,
    const LANE_ID: u8,
>(
    Join<StrumPair<NOTE_1, NOTE_2, NOTE_TICK, LANE_ID>, Step<Pick<NOTE_3, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);
