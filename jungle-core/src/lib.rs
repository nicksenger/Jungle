/// An inhabitant of the Jungle.
pub trait Entity {
    /// The result of observing this `Entity`.
    type Appearance;

    /// What drives this `Entity` to change its behavior.
    type Motivation;

    /// The fundamental behavior of this `Entity`.
    type Instinct;

    /// Observe this entity and return its visual representation.
    fn observe(&self) -> Self::Appearance;

    /// Influence this entity's behavior given a motivation.
    fn influence(&self, motive: Self::Motivation);
}

/// A trait for entities that embody a Wild ecosystem — a collective whole greater
/// than the sum of its parts, representing the will of the ecosystem as one.
pub trait Wild {}
