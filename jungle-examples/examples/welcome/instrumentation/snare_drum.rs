pub enum SnareDrumArticulation {
    /// A standard, clean strike to the center of the drum head.
    CenterHit,
    /// Striking the center of the head and the metal rim simultaneously.
    /// This is the primary articulation for the massive verse/chorus backbeats.
    Rimshot,
    /// Laying the stick across the head and striking the rim for a woody click.
    /// Useful for low-energy dynamic drops.
    Sidestick,
    /// A very soft, low-velocity hit. Adler uses these to fill the space between backbeats.
    GhostNote,
    /// Two rapid, almost overlapping strikes (one hand trailing the other) to add weight.
    Flam,
}
