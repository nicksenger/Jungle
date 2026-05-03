use core::marker::PhantomData;
use typosaurus::cmp::Equality;
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

/// A collection of Jungle `Niche`s and the `Animal`s that fill them.
pub trait Ecosystem {
    type Niches;
    type Animals;
}

// --- Type-level helpers for the Jungle constraint ---

/// Maps an `Animal` type to its `Niches` list.
pub struct NichesOf;
impl<T> typosaurus::traits::functor::Mapper<T> for NichesOf
where
    T: Animal,
{
    type Out = <T as Animal>::Niches;
}

/// Maps a `Niche` type to its `Id` type.
pub struct NicheIdExtractor;
impl<T> typosaurus::traits::functor::Mapper<T> for NicheIdExtractor
where
    T: Niche,
{
    type Out = <T as Niche>::Id;
}

/// Extract the `Id` types from a list of `Niche`s.
pub type NicheIds<Niches> =
    <(Niches, NicheIdExtractor) as typosaurus::traits::functor::Map<Niches, NicheIdExtractor>>::Out;

/// The flattened, deduplicated set of all niche `Id`s across every
/// animal in the ecosystem.
pub type AllAnimalNicheIdsSet<Ec: Ecosystem> = typosaurus::collections::list::Dedup<
    <(
        <(Ec::Animals, NichesOf)
            as typosaurus::traits::functor::Map<Ec::Animals, NichesOf>>::Out,
        NicheIdExtractor,
    ) as typosaurus::traits::functor::Map<
        <(Ec::Animals, NichesOf)
            as typosaurus::traits::functor::Map<Ec::Animals, NichesOf>>::Out,
        NicheIdExtractor,
    >>::Out,
>;

/// The deduplicated set of niche `Id`s declared on the ecosystem itself.
pub type EcosystemNicheIdsSet<Ec: Ecosystem> =
    typosaurus::collections::list::Dedup<NicheIds<Ec::Niches>>;

/// A marker trait for ecosystems where the niches occupied by every
/// animal are a subset of the niches declared on the ecosystem.
///
/// This is verified at the type level: an ecosystem is a jungle
/// **iff** all niche `Id`s from all its animals appear in the
/// ecosystem's own `Niches` list.
pub trait Jungle<Ec: Ecosystem> {}

impl<Ec> Jungle<Ec> for ()
where
    Ec: Ecosystem,
    AllAnimalNicheIdsSet<Ec>: typosaurus::collections::list::Dedup<typosaurus::collections::list::Empty>,
    <(AllAnimalNicheIdsSet<Ec>, EcosystemNicheIdsSet<Ec>) as typosaurus::collections::set::Difference>::Out:
        typosaurus::collections::set::IsEmpty,
    <<(AllAnimalNicheIdsSet<Ec>, EcosystemNicheIdsSet<Ec>) as typosaurus::collections::set::Difference>::Out
        as typosaurus::collections::set::IsEmpty>::Out: typosaurus::bool::Truthy,
{
}

pub use behavior::{Action, Impulse, Niche};
pub use taxonomy::{Class, Family, Genus, Order, Phylum, Species};
