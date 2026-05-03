use core::marker::PhantomData;
use typosaurus::cmp::Equality;
use typosaurus::num::Unsigned;


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

    /// The actions this `Animal` can take.
    type Actions;
}

/// An organism that hosts symbionts.
pub trait Host {
    /// Organisms that live in close association with this `Host`.
    type Symbionts;
}

/// A newtype wrapper proving that two `Animal` types are equal
/// when their `Id`s are equal.
///
/// This type exists to work around Rust's orphan rules: implementing
/// `Equality<U>` directly for a generic `Animal` type would be disallowed
/// because neither the `Equality` trait nor `T` is defined locally.
///
/// Instead, this newtype is local to this crate, so the orphan rule is
/// satisfied. Equality is derived from the `Id` associated type.
pub struct AnimalEquality<T, U>(PhantomData<(T, U)>)
where
    T: Animal,
    U: Animal;

/// Blanket `Equality` for any two `Animal` newtypes, based on their `Id` types.
/// Two `Animal` types are equal iff their `Id`s are equal.
impl<T, U> Equality<AnimalEquality<T, U>> for AnimalEquality<T, U>
where
    T: Animal,
    U: Animal,
    T::Id: Equality<U::Id>,
{
    type Out = <T::Id as Equality<U::Id>>::Out;
}

/// A collection of Jungle entities and the `Animal`s that fill them.
pub trait Ecosystem {
    type Actions;
    type Animals;
}


/// A channel that transforms a stream of inputs into a stream of outputs.
pub trait Channel {
    /// The input type accepted by this channel.
    type In;

    /// The output type produced by this channel.
    type Out;

    /// Process a stream of inputs, yielding a stream of outputs.
    fn channel(self, input: impl futures::Stream<Item = Self::In>) -> impl futures::Stream<Item = Self::Out>;
}
