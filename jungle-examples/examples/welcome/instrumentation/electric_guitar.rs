use jungle_sdk::prelude::*;
use welcome_audio::{PlayPriority, PlayRequest};

use crate::{
    action::{MergeUnit, Rest as GenericRest},
    effect::{Rest, Sound, SoundInput},
};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

type PostMergeRest<const TICKS: u32, const LANE_ID: u8> =
    GenericRest<ElectricGuitarArticulation, TICKS, LANE_ID>;

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
    /// A sustained lead-guitar chord voice.
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
#[jungle::action]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Action
    for Pick<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<ElectricGuitar>;
    type Input = ();
    type Output = ();

    fn emit(
        state: &ElectricGuitarArticulation,
        _input: Self::Input,
    ) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: *state,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: LANE_ID,
        }
    }

    fn absorb(
        _state: &mut ElectricGuitarArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.map_err(|_err| Failure::from("note playback should succeed"))?;
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
    Step<MergeUnit<ElectricGuitarArticulation>>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);

#[derive(Flow)]
pub struct StrumPair<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u32, const LANE_ID: u8>(
    Join<Step<Pick<NOTE_1, NOTE_TICK, 0, LANE_ID>>, Step<Pick<NOTE_2, NOTE_TICK, 0, LANE_ID>>>,
    Step<MergeUnit<ElectricGuitarArticulation>>,
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
    Step<MergeUnit<ElectricGuitarArticulation>>,
    Step<PostMergeRest<REST_TICK, LANE_ID>>,
);
