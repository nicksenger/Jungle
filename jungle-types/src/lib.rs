mod behavior;
pub mod taxonomy;

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// How this `Animal` may appear to observers.
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

pub use behavior::{Action, Impulse};
pub use taxonomy::{Class, Family, Genus, Order, Phylum, Species};
