use typosaurus::num::Unsigned;

mod behavior;
mod taxonomy;

/// A newtype wrapper around an [`Unsigned`] constant.
///
/// Provides a typed handle for compile-time numeric constants used
/// as type-level identifiers within the Jungle ecosystem.
pub struct Id<T: Unsigned>(pub T);

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this `Animal`.
    type Id;

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
