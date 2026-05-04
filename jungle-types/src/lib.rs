#![recursion_limit = "512"]

mod behavior;
mod meta;
pub use behavior::{Action, Instinct};
pub use meta::Id;
use inception::*;
use typosaurus::collections::list;
use typosaurus::traits::semigroup::Mappend;

/// A collection of Jungle entities and the Animals that fill them.
///
/// This is defined before [`Animal`] so downstream types can reference it
/// when specifying associated collections.
pub trait Ecosystem {
    type Actions;
    type Animals;
}

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// The fundamental behavior of this Animal.
    type Instinct;
}

/// Any collection of [`Animal`]s with a flat type-level list of members.
#[inception(property = JungleAnimal, types)]
pub trait Animals {
    #[induce(
        base = list::Empty,
        merge = <(<Head as Animals>::List, <Tail as Animals>::List) as Mappend>::Out where { (<Head as Animals>::List, <Tail as Animals>::List): Mappend },
        merge_variant = <(<Head as Animals>::List, <Tail as Animals>::List) as Mappend>::Out where { (<Head as Animals>::List, <Tail as Animals>::List): Mappend },
        join = <Fields as Animals>::List
    )]
    type List;
}

/// Any collection of [`Action`]s with a flat type-level list of members.
#[inception(property = JungleAction, types)]
pub trait Actions {
    #[induce(
        base = list::Empty,
        merge = <(<Head as Actions>::List, <Tail as Actions>::List) as Mappend>::Out where { (<Head as Actions>::List, <Tail as Actions>::List): Mappend },
        merge_variant = <(<Head as Actions>::List, <Tail as Actions>::List) as Mappend>::Out where { (<Head as Actions>::List, <Tail as Actions>::List): Mappend },
        join = <Fields as Actions>::List
    )]
    type List;
}

/// An organism that hosts symbionts.
pub trait Host {
    /// Organisms that live in close association with this Host.
    type Symbionts;
}

/// A trait that transforms a stream of inputs into a stream of outputs.
pub trait Evoke {
    /// The input type accepted by this evoke.
    type In;

    /// The output type produced by this evoke.
    type Out;

    /// Process a stream of inputs, yielding a stream of outputs.
    fn evoke(self, input: impl futures::Stream<Item = Self::In>) -> impl futures::Stream<Item = Self::Out>;
}
