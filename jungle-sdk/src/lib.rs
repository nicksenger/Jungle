mod behavior;
pub mod taxonomy;

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

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// The result of observing this `Animal`.
    type Appearance;

    /// What drives this `Animal` to change its behavior.
    type Motivation;

    /// The fundamental behavior of this `Animal`.
    type Instinct;
}

pub trait Ecosystem {
    type Niches;
    type Members;
}

pub trait Niche {}

pub use behavior::{Action, Impulse};
