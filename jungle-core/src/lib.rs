/// An inhabitant of the Jungle.
pub trait Entity {
    /// The fundamental behavior of this `Entity`.
    type Instinct;
}

pub trait Jungle {}

/// A worker carries out the will of the Jungle.
pub trait Worker<T: Jungle> {}
