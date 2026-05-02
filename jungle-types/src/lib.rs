mod behavior;
mod taxonomy;

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// The fundamental behavior of this `Animal`.
    type Instinct;

    /// How this `Animal` may appear to observers.
    type Form;

    /// What drives this `Animal` to change its behavior.
    type Motivation;

    /// Organisms that live in close association with this `Animal`.
    type Symbionts;

    /// The ecological roles this `Animal` interacts with.
    type Niches;
}

/// A collection of Jungle `Niche`s and the `Animal`s that fill them.
pub trait Ecosystem {
    type Niches;
    type Animals;
}

pub use behavior::{Action, Impulse, Niche};
pub use taxonomy::{Class, Family, Genus, Order, Phylum, Species};
