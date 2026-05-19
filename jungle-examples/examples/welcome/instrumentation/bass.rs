pub enum BassArticulation {
    /// A hard, aggressive pick strike with normal sustain.
    Picked,
    /// Forcing the string down so hard it clanks against the frets on attack.
    /// Used to accent the downbeats of the chorus.
    AccentedClank,
    /// Muting the string immediately with the fretting hand.
    /// Essential for keeping the fast-moving basslines crisp and preventing mud.
    StaccatoMute,
    /// Sliding from one note down into the next, a classic Duff transition tool.
    SlideDown,
    /// Striking a completely dead string for a purely percussive thud.
    GhostNote,
}
