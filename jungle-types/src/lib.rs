use typosaurus::num::Unsigned;

mod meta;

/// Newtype wrapper around an Unsigned constant.
pub struct Id<T: Unsigned>(pub T);

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// The fundamental behavior of this Animal.
    type Instinct;

    /// The actions this Animal can take.
    type Actions;
}

/// An organism that hosts symbionts.
pub trait Host {
    /// Organisms that live in close association with this Host.
    type Symbionts;
}

/// A collection of Jungle entities and the Animals that fill them.
pub trait Ecosystem {
    type Actions;
    type Animals;
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
