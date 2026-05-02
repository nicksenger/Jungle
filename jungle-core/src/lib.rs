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

pub trait Jungle {}

/// A vessel is a container for an entity within the Jungle ecosystem.
pub trait Vessel<T: Jungle> {}
