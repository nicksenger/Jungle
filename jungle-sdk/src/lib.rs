mod behavior;
pub mod taxonomy;

pub use jungle_core::Entity;

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// The result of observing this `Animal`.
    type Form;

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
