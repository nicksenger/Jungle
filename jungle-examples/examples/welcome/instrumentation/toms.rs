pub enum TomsArticulation {
    /// A clean, resonant strike to the center of the tom.
    StandardHit,
    /// An extra-powerful strike maximizing shell resonance.
    AccentedHit,
    /// Striking two different toms simultaneously (e.g., Rack Tom 2 and Floor Tom).
    /// Used for the massive downbeat punctuation marks in the breakdown.
    DoubleHit,
}
