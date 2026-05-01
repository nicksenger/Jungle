mod behavior;
mod taxonomy;

/// An inhabitant of the Jungle.
pub trait Entity {
    /// The result of observing this `Entity`
    type Appearance;

    /// What drives this `Entity` to change its behavior
    type Motivation;

    /// The fundamental behavior of this `Entity`.
    type Instinct;
}

/// An active member of the current Jungle ecosystem.
pub trait Animal: Entity {
    fn observe(&self) -> Self::Appearance;
    fn influence(&self, motive: Self::Motivation);
}

pub trait Ecosystem {
    type Roles;
    type Members;
}

pub trait Niche {}

pub use behavior::{Action, Impulse};
pub use taxonomy::{Genus, Species};
