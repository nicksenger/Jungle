mod behavior;
mod error;
mod instinct;
mod meta;
pub use behavior::Action;
pub use error::Error;
pub use instinct::Instinct;
use inception::*;
pub use meta::Id;
pub use meta::{
    ActionMember, ActionSet, AllFrom, AnimalActionSet, AnimalMember, AnimalSet, AnimalStates,
    AnimalStatesCompatible, StripActionHeaders, StripAnimalHeaders,
};
use typosaurus::collections::list::{self, List as TList};
use typosaurus::collections::sp::Node;
use typosaurus::num::consts::U0;

/// A collection of `Animals` which act together as a system.
pub trait Ecosystem {
    type Animals;
}

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// The state of this `Animal` at any given time.
    type State;

    /// The fundamental behavior of this Animal.
    type Instinct;
}

#[inception(property = Ident, types)]
pub trait Identified {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        merge_variant = TList<(<Head as Identified>::Id, <Tail as Identified>::Id)>,
        join = TList<(U0, <Fields as Identified>::Id)>
    )]
    type Id;
}

/// Any collection of [`Animal`]s with a flat type-level list of members.
#[inception(property = JungleAnimals, types)]
pub trait Animals {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        merge_variant = TList<(<Head as Animals>::List, <Tail as Animals>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Animals>::List)> where { Self: Identified }
    )]
    type List;
}

/// Any collection of [`Action`]s with a flat type-level list of members.
#[inception(property = JungleActions, types)]
pub trait Actions {
    #[induce(
        base = list::Empty,
        merge = TList<(<Head as Actions>::List, <Tail as Actions>::List)>,
        merge_variant = TList<(<Head as Actions>::List, <Tail as Actions>::List)>,
        join = TList<(Node<<Self as Identified>::Id, ()>, <Fields as Actions>::List)> where { Self: Identified }
    )]
    type List;
}

/// A single typed phase in a temporal workflow.
///
/// A phase first runs until it yields `Yield`, then accepts `Await` to
/// transition into `Next`.
pub trait Phase {
    /// The value emitted when this phase reaches its next suspension point.
    type Yield;

    /// The input required to resume this workflow after yielding.
    type Await;

    /// The next phase reached after resuming with `Await`.
    type Next;

    /// Run this phase until it yields a value.
    fn run(&mut self) -> Self::Yield;

    /// Resume after a yielded value by providing the expected input.
    fn resume(self, input: Self::Await) -> Self::Next;
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
    fn evoke(
        self,
        input: impl futures::Stream<Item = Self::In>,
    ) -> impl futures::Stream<Item = Self::Out>;
}
