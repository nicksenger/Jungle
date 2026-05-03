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

    /// How this `Animal` may appear to observers.
    type Form;

    /// What drives this `Animal` to change its behavior.
    type Motivation;

    /// Organisms that live in close association with this `Animal`.
    type Symbionts;

    /// The actions this `Animal` can take.
    type Actions;
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


/// An entity that can be observed, revealing its appearance.
pub trait Observe {
    /// The appearance revealed when this entity is observed.
    type Appearance;

    /// A stream that yields appearances over time.
    type Stream: futures::Stream<Item = Self::Appearance>;

    fn observe(&self) -> Self::Appearance;
}

/// An entity that can be influenced by an external motive.
pub trait Influence {
    /// The motive used to influence this entity.
    type Motive;

    /// A sink that accepts motives.
    type Sink: futures::Sink<Self::Motive>;

    fn influence(&self, motive: Self::Motive);
}
