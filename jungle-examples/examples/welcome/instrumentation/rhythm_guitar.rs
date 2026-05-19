pub enum RhythmGuitarArticulation {
    /// A standard, ringing open or barre chord.
    Sustained,
    /// Constant, tight palm-muting to drive the verses.
    PalmMuted,
    /// Lifting the fretting hand immediately after striking to choke the chord.
    /// Crucial for the staccato, funky stabs in the verse groove.
    Choked,
    /// Striking strings completely muted by the left hand.
    /// Used heavily during the scratchy intro buildup before the full band kicks in.
    RhythmicScratch,
    /// Sliding an entire chord shape up or down the neck.
    ChordSlide,
}
