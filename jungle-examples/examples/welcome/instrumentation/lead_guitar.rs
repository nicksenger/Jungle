use crate::audio::AudioHandle;

pub struct LeadGuitar {
    audio: AudioHandle,
}

impl LeadGuitar {
    pub fn new(audio: AudioHandle) -> Self {
        Self { audio }
    }

    pub fn audio(&self) -> &AudioHandle {
        &self.audio
    }
}

pub enum LeadGuitarArticulation {
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
}
