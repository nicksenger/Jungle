use jungle_sdk::prelude::*;

use crate::effect::{Dyad, Monad, Triad};

use super::{Instrument, Note};

mod audio;

pub struct ElectricGuitar {
    audio: crate::audio::AudioHandle,
}

impl ElectricGuitar {
    pub fn new(audio: crate::audio::AudioHandle) -> Self {
        Self { audio }
    }

    pub fn audio(&self) -> &crate::audio::AudioHandle {
        &self.audio
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ElectricGuitarArticulation {
    /// Standard picked note with normal sustain and release.
    Sustained,

    /// Restricting the string vibration with the side of the picking hand.
    /// Essential for the driving rhythm fills under the vocals.
    PalmMuted,

    /// A note sounded entirely by the fretting hand striking the fretboard.
    /// Crucial for the fluid, rapid note runs in the main solo.
    HammerOn,

    /// A note sounded by pulling a fretting finger off a string to release a lower note.
    /// Used in tandem with HammerOns for smooth, unpicked legato phrasing.
    PullOff,

    /// Gently touching the string at specific nodes (like the 5th, 7th, or 12th frets)
    /// to get a bell-like chime. Slash uses these for texture.
    NaturalHarmonic,

    /// "Pinch" harmonics. Pressing the thumb of the picking hand against the string
    /// instantly after picking it, forcing a screaming, high-pitched squeal.
    /// Slash peppers these heavily throughout the verses and fills.
    PinchHarmonic,

    /// Sliding into a note from an indefinite lower or higher pitch.
    /// The signature entry mechanism for almost every phrase in the song.
    Slide,

    /// Striking a string that is completely muted by the fretting hand.
    /// This creates a purely rhythmic, percussive "scratch" or "chug" sound
    /// right before a big chord hits.
    RhythmicRake,

    /// A sustained rhythm-guitar chord voice.
    RhythmSustained,

    /// A tightly palm-muted rhythm-guitar chord voice.
    RhythmPalmMuted,

    /// Lifting the fretting hand immediately after striking to choke the chord.
    /// Crucial for the staccato, funky stabs in the verse groove.
    Choked,

    /// Striking strings completely muted by the left hand.
    /// Used heavily during the scratchy intro buildup before the full band kicks in.
    RhythmicScratch,

    /// Sliding an entire chord shape up or down the neck.
    ChordSlide,
}

impl Default for ElectricGuitarArticulation {
    fn default() -> Self {
        Self::RhythmSustained
    }
}

impl Instrument for ElectricGuitar {
    type Articulation = ElectricGuitarArticulation;

    async fn play(&self, note: Note<Self::Articulation>) -> Result<(), super::Error> {
        audio::play(&self.audio, note).await
    }
}

pub struct Pick<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Pick<NOTE, NOTE_TICK, REST_TICK>
{
    type Effect = Monad<ElectricGuitar, ElectricGuitarArticulation, NOTE, NOTE_TICK, REST_TICK>;
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

pub struct Pluck<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Pluck<NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>
{
    type Effect =
        Dyad<ElectricGuitar, ElectricGuitarArticulation, NOTE_1, NOTE_2, NOTE_TICK, REST_TICK>;
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

pub struct Strum<
    const NOTE_1: u8,
    const NOTE_2: u8,
    const NOTE_3: u8,
    const NOTE_TICK: u8,
    const REST_TICK: u8,
>;
#[jungle::act]
impl<const NOTE_1: u8, const NOTE_2: u8, const NOTE_3: u8, const NOTE_TICK: u8, const REST_TICK: u8> Act
    for Strum<NOTE_1, NOTE_2, NOTE_3, NOTE_TICK, REST_TICK>
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
