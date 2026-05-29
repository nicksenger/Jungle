use jungle_sdk::prelude::*;
use welcome_audio::{PlayPriority, PlayRequest};

use crate::effect::{Sound, SoundInput};

use super::{amplitude_gain, Error, Instrument, Note, SynthHandle};

pub struct Bass {
    audio: welcome_audio::AudioHandle,
    synth: SynthHandle,
}

impl Bass {
    pub fn new(audio: welcome_audio::AudioHandle, synth: SynthHandle) -> Self {
        Self { audio, synth }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum BassArticulation {
    /// A hard, aggressive pick strike with normal sustain.
    Picked,
}

impl Default for BassArticulation {
    fn default() -> Self {
        Self::Picked
    }
}

impl Instrument for Bass {
    type Articulation = BassArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), Error> {
        let (pcm, gain, playback_rate) = self.synth.bass(note).await?;

        let mut request = PlayRequest::new(pcm, 1, welcome_audio::dsp::SAMPLE_RATE);
        request.gain = gain * amplitude_gain(&note);
        request.playback_rate = playback_rate;
        request.pan = 0.0;
        request.priority = PlayPriority::Normal;
        self.audio
            .play(request)
            .await
            .map_err(|_| Error::Submission)
    }
}

#[allow(unused)]
pub struct Thump<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u32, const REST_TICK: u32, const LANE_ID: u8> Act
    for Thump<NOTE, NOTE_TICK, REST_TICK, LANE_ID>
{
    type Effect = Sound<Bass>;
    type Input = ();
    type Output = ();

    fn emit(state: &BassArticulation, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        SoundInput {
            articulation: *state,
            note: NOTE,
            note_ticks: NOTE_TICK,
            rest_ticks: REST_TICK,
            lane_id: LANE_ID,
        }
    }

    fn absorb(
        _state: &mut BassArticulation,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.expect("note playback should succeed");
    }
}
