use typosaurus::bool::{And, Not, Truthy};
use typosaurus::cmp::Equality;
use typosaurus::cmp::IsEqual;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPFlatten},
    Container,
};
use typosaurus::num::{Max, Unsigned};
use typosaurus::traits::fold::Foldable;
use typosaurus::traits::functor::{Map, Mapper};

use super::{
    Animal, Animals, BoundAnimal, BoundAnimalJourney, Ecosystem, Effect, EffectSchema, Effects,
    Journey, JourneyEffects,
};
use core::marker::PhantomData;

/// Newtype wrapper around an Unsigned constant.
pub struct Id<T: Unsigned>(pub T);

pub trait IdValue {
    type Value: Unsigned;
}
impl<T> IdValue for Id<T>
where
    T: Unsigned,
{
    type Value = T;
}

pub trait AnimalIdValue {
    const U32: u32;
}
impl<T> AnimalIdValue for Id<T>
where
    T: Unsigned,
{
    const U32: u32 = <T as Unsigned>::U32;
}

/// Blanket impl: `Id<T>` is equal to `Id<U>` iff `T` is equal to `U`.
impl<T, U> Equality<Id<U>> for Id<T>
where
    T: Unsigned + Equality<U>,
    U: Unsigned,
{
    type Out = <T as Equality<U>>::Out;
}

pub trait EffectIdentity {
    type Id;
}
pub trait EffectMember: EffectIdentity {}
pub trait AnimalMember {}

impl<T> EffectIdentity for T
where
    T: EffectSchema,
{
    type Id = <T as EffectSchema>::Id;
}
impl<T> EffectMember for T where T: EffectIdentity {}
impl<T> AnimalMember for T where T: Animal {}

macro_rules! all_from_list_chain {
    ($h:ident) => {
        list::List<($h, list::Empty)>
    };
    ($h:ident, $($rest:ident),+) => {
        list::List<($h, all_from_list_chain!($($rest),+))>
    };
}
macro_rules! all_from_list_chain_tail {
    ($h:ident ; $tail:ty) => {
        list::List<($h, $tail)>
    };
    ($h:ident, $($rest:ident),+ ; $tail:ty) => {
        list::List<($h, all_from_list_chain_tail!($($rest),+ ; $tail))>
    };
}

pub trait AllFrom<T> {}
impl<T> AllFrom<T> for list::Empty {}
macro_rules! all_from_len_impl {
    ($h0:ident) => {
        impl<T, $h0> AllFrom<T> for all_from_list_chain!($h0)
        where
            T: Into<$h0>,
        {
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<T, $h0, $($rest,)+> AllFrom<T> for all_from_list_chain!($h0, $($rest),+)
        where
            T: Into<$h0>,
            all_from_list_chain!($($rest),+): AllFrom<T>,
        {
        }
    };
}
all_from_len_impl!(A0);
all_from_len_impl!(A0; A1);
all_from_len_impl!(A0; A1, A2);
all_from_len_impl!(A0; A1, A2, A3);
all_from_len_impl!(A0; A1, A2, A3, A4);
all_from_len_impl!(A0; A1, A2, A3, A4, A5);
all_from_len_impl!(A0; A1, A2, A3, A4, A5, A6);
impl<T, A0, A1, A2, A3, A4, A5, A6, A7, Tail> AllFrom<T>
    for list::List<(A0, all_from_list_chain_tail!(A1, A2, A3, A4, A5, A6, A7 ; Tail))>
where
    T: Into<A0>,
    T: Into<A1>,
    T: Into<A2>,
    T: Into<A3>,
    T: Into<A4>,
    T: Into<A5>,
    T: Into<A6>,
    T: Into<A7>,
    Tail: AllFrom<T>,
{
}

pub trait StripEffectHeaders {
    type Out;
}
impl StripEffectHeaders for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> StripEffectHeaders for list::List<(Node<K, ()>, Tail)>
where
    Tail: StripEffectHeaders<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> StripEffectHeaders for list::List<(Node<K, Head>, Tail)>
where
    Head: EffectMember,
    Tail: StripEffectHeaders<Out = TailOut>,
{
    type Out = list::List<(Head, TailOut)>;
}

pub trait KeepEffectNodes {
    type Out;
}
impl KeepEffectNodes for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> KeepEffectNodes for list::List<(Node<K, ()>, Tail)>
where
    Tail: KeepEffectNodes<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> KeepEffectNodes for list::List<(Node<K, Head>, Tail)>
where
    Head: EffectMember,
    Tail: KeepEffectNodes<Out = TailOut>,
{
    type Out = list::List<(Node<K, Head>, TailOut)>;
}

pub trait StripAnimalHeaders {
    type Out;
}
impl StripAnimalHeaders for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> StripAnimalHeaders for list::List<(Node<K, ()>, Tail)>
where
    Tail: StripAnimalHeaders<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> StripAnimalHeaders for list::List<(Node<K, Head>, Tail)>
where
    Head: AnimalMember,
    Tail: StripAnimalHeaders<Out = TailOut>,
{
    type Out = list::List<(Head, TailOut)>;
}

pub type EffectSet<T> = <SPFlatten<<T as Effects>::List> as StripEffectHeaders>::Out;
pub type AnimalSet<T> = <SPFlatten<<T as Animals>::List> as StripAnimalHeaders>::Out;

pub struct AnimalVersion<AnimalId, Generation>(PhantomData<(AnimalId, Generation)>);
impl<IdA, GenA, IdB, GenB> Equality<AnimalVersion<IdB, GenB>> for AnimalVersion<IdA, GenA>
where
    IdA: Equality<IdB>,
    GenA: Equality<GenB>,
    (<IdA as Equality<IdB>>::Out, <GenA as Equality<GenB>>::Out): And,
{
    type Out = <(<IdA as Equality<IdB>>::Out, <GenA as Equality<GenB>>::Out) as And>::Out;
}

pub struct WithAnimalVersion;
impl<T> Mapper<T> for WithAnimalVersion
where
    T: Animal,
{
    type Out = AnimalVersion<<T as Animal>::Id, <T as Animal>::Generation>;
}

pub type AnimalVersions<T> = <(AnimalSet<T>, WithAnimalVersion) as Map<
    <AnimalSet<T> as Container>::Content,
    WithAnimalVersion,
>>::Out;

pub trait ContainsAnimalVersion<Target> {
    type Out;
}
impl<Target> ContainsAnimalVersion<Target> for list::Empty {
    type Out = typosaurus::bool::False;
}
macro_rules! list_chain {
    ($h:ty) => {
        list::List<($h, list::Empty)>
    };
    ($h:ty, $($rest:ty),+) => {
        list::List<($h, list_chain!($($rest),+))>
    };
}
macro_rules! list_chain_tail {
    ($h:ty ; $tail:ty) => {
        list::List<($h, $tail)>
    };
    ($h:ty, $($rest:ty),+ ; $tail:ty) => {
        list::List<($h, list_chain_tail!($($rest),+ ; $tail))>
    };
}
macro_rules! contains_animal_version_len_impl {
    ($h0:ident) => {
        impl<$h0, Target> ContainsAnimalVersion<Target> for list_chain!($h0)
        where
            $h0: Equality<Target>,
        {
            type Out = <$h0 as Equality<Target>>::Out;
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+ Target> ContainsAnimalVersion<Target> for list_chain!($h0, $($rest),+)
        where
            $h0: Equality<Target>,
            list_chain!($($rest),+): ContainsAnimalVersion<Target>,
            (
                <$h0 as Equality<Target>>::Out,
                <list_chain!($($rest),+) as ContainsAnimalVersion<Target>>::Out,
            ): typosaurus::bool::Or,
        {
            type Out = <(
                <$h0 as Equality<Target>>::Out,
                <list_chain!($($rest),+) as ContainsAnimalVersion<Target>>::Out,
            ) as typosaurus::bool::Or>::Out;
        }
    };
}
contains_animal_version_len_impl!(H0);
contains_animal_version_len_impl!(H0; H1);
contains_animal_version_len_impl!(H0; H1, H2);
contains_animal_version_len_impl!(H0; H1, H2, H3);
contains_animal_version_len_impl!(H0; H1, H2, H3, H4);
contains_animal_version_len_impl!(H0; H1, H2, H3, H4, H5);
contains_animal_version_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail, Target> ContainsAnimalVersion<Target>
    for list::List<(H0, list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail))>
where
    H0: Equality<Target>,
    list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail): ContainsAnimalVersion<Target>,
    (
        <H0 as Equality<Target>>::Out,
        <list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<Target>>::Out,
    ): typosaurus::bool::Or,
{
    type Out = <(
        <H0 as Equality<Target>>::Out,
        <list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<Target>>::Out,
    ) as typosaurus::bool::Or>::Out;
}

pub trait UniqueAnimalVersions {
    type Out;
}
impl UniqueAnimalVersions for list::Empty {
    type Out = typosaurus::bool::True;
}
macro_rules! unique_animal_versions_len_impl {
    ($h0:ident) => {
        impl<$h0> UniqueAnimalVersions for list_chain!($h0) {
            type Out = typosaurus::bool::True;
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+> UniqueAnimalVersions for list_chain!($h0, $($rest),+)
        where
            list_chain!($($rest),+): ContainsAnimalVersion<$h0>,
            <list_chain!($($rest),+) as ContainsAnimalVersion<$h0>>::Out: Not,
            list_chain!($($rest),+): UniqueAnimalVersions,
            (
                <<list_chain!($($rest),+) as ContainsAnimalVersion<$h0>>::Out as Not>::Out,
                <list_chain!($($rest),+) as UniqueAnimalVersions>::Out,
            ): And,
        {
            type Out = <(
                <<list_chain!($($rest),+) as ContainsAnimalVersion<$h0>>::Out as Not>::Out,
                <list_chain!($($rest),+) as UniqueAnimalVersions>::Out,
            ) as And>::Out;
        }
    };
}
unique_animal_versions_len_impl!(H0);
unique_animal_versions_len_impl!(H0; H1);
unique_animal_versions_len_impl!(H0; H1, H2);
unique_animal_versions_len_impl!(H0; H1, H2, H3);
unique_animal_versions_len_impl!(H0; H1, H2, H3, H4);
unique_animal_versions_len_impl!(H0; H1, H2, H3, H4, H5);
unique_animal_versions_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail> UniqueAnimalVersions
    for list::List<(H0, list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail))>
where
    list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail): ContainsAnimalVersion<H0>,
    <list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out: Not,
    list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail): ContainsAnimalVersion<H1>,
    <list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out: Not,
    list_chain_tail!(H3, H4, H5, H6, H7 ; Tail): ContainsAnimalVersion<H2>,
    <list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out: Not,
    list_chain_tail!(H4, H5, H6, H7 ; Tail): ContainsAnimalVersion<H3>,
    <list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out: Not,
    list_chain_tail!(H5, H6, H7 ; Tail): ContainsAnimalVersion<H4>,
    <list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out: Not,
    list_chain_tail!(H6, H7 ; Tail): ContainsAnimalVersion<H5>,
    <list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out: Not,
    list_chain_tail!(H7 ; Tail): ContainsAnimalVersion<H6>,
    <list_chain_tail!(H7 ; Tail) as ContainsAnimalVersion<H6>>::Out: Not,
    Tail: ContainsAnimalVersion<H7>,
    <Tail as ContainsAnimalVersion<H7>>::Out: Not,
    Tail: UniqueAnimalVersions,
    (
        <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
        <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
    ): And,
    (
        <(
            <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
            <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
        ) as And>::Out,
        <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
            ) as And>::Out,
            <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
        ) as And>::Out,
        <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <(
                    <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                    <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
            ) as And>::Out,
            <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
        ) as And>::Out,
        <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <(
                    <(
                        <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                        <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                    ) as And>::Out,
                    <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
            ) as And>::Out,
            <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
        ) as And>::Out,
        <<list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <(
                    <(
                        <(
                            <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                            <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                        ) as And>::Out,
                        <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
                    ) as And>::Out,
                    <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
            ) as And>::Out,
            <<list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out as Not>::Out,
        ) as And>::Out,
        <<list_chain_tail!(H7 ; Tail) as ContainsAnimalVersion<H6>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <(
                    <(
                        <(
                            <(
                                <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                                <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                            ) as And>::Out,
                            <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
                        ) as And>::Out,
                        <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
                    ) as And>::Out,
                    <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out as Not>::Out,
            ) as And>::Out,
            <<list_chain_tail!(H7 ; Tail) as ContainsAnimalVersion<H6>>::Out as Not>::Out,
        ) as And>::Out,
        <<Tail as ContainsAnimalVersion<H7>>::Out as Not>::Out,
    ): And,
    (
        <(
            <(
                <(
                    <(
                        <(
                            <(
                                <(
                                    <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                                    <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                                ) as And>::Out,
                                <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
                            ) as And>::Out,
                            <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
                        ) as And>::Out,
                        <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
                    ) as And>::Out,
                    <<list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H7 ; Tail) as ContainsAnimalVersion<H6>>::Out as Not>::Out,
            ) as And>::Out,
            <<Tail as ContainsAnimalVersion<H7>>::Out as Not>::Out,
        ) as And>::Out,
        <Tail as UniqueAnimalVersions>::Out,
    ): And,
{
    type Out = <(
        <(
            <(
                <(
                    <(
                        <(
                            <(
                                <(
                                    <<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H0>>::Out as Not>::Out,
                                    <<list_chain_tail!(H2, H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H1>>::Out as Not>::Out,
                                ) as And>::Out,
                                <<list_chain_tail!(H3, H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H2>>::Out as Not>::Out,
                            ) as And>::Out,
                            <<list_chain_tail!(H4, H5, H6, H7 ; Tail) as ContainsAnimalVersion<H3>>::Out as Not>::Out,
                        ) as And>::Out,
                        <<list_chain_tail!(H5, H6, H7 ; Tail) as ContainsAnimalVersion<H4>>::Out as Not>::Out,
                    ) as And>::Out,
                    <<list_chain_tail!(H6, H7 ; Tail) as ContainsAnimalVersion<H5>>::Out as Not>::Out,
                ) as And>::Out,
                <<list_chain_tail!(H7 ; Tail) as ContainsAnimalVersion<H6>>::Out as Not>::Out,
            ) as And>::Out,
            <<Tail as ContainsAnimalVersion<H7>>::Out as Not>::Out,
        ) as And>::Out,
        <Tail as UniqueAnimalVersions>::Out,
    ) as And>::Out;
}

pub trait AnimalVersionIdentitiesUnique: Animals {}
impl<T> AnimalVersionIdentitiesUnique for T
where
    T: Animals,
    <T as Animals>::List: FlattenNodes,
    SPFlatten<<T as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T>: Container,
    (AnimalSet<T>, WithAnimalVersion): Map<<AnimalSet<T> as Container>::Content, WithAnimalVersion>,
    AnimalVersions<T>: UniqueAnimalVersions,
    <AnimalVersions<T> as UniqueAnimalVersions>::Out: Truthy,
{
}

pub struct WithGenerationFor<AnimalId>(PhantomData<AnimalId>);
impl<T, AnimalId> Mapper<T> for WithGenerationFor<AnimalId>
where
    T: Animal,
    (<T as Animal>::Id, AnimalId): IsEqual,
{
    type Out = (
        <T as Animal>::Generation,
        <(<T as Animal>::Id, AnimalId) as IsEqual>::Out,
    );
}

pub type GenerationsForAnimals<T, AnimalId> =
    <<(AnimalSet<T>, WithGenerationFor<AnimalId>) as Map<
        <AnimalSet<T> as Container>::Content,
        WithGenerationFor<AnimalId>,
    >>::Out as Foldable<list::Filter>>::Out;

pub type Generations<E, AnimalId> = GenerationsForAnimals<<E as Ecosystem>::Animals, AnimalId>;

pub trait MaxGeneration {
    type Out;
}
impl MaxGeneration for list::Empty {
    type Out = typosaurus::num::consts::U0;
}
macro_rules! max_generation_len_impl {
    ($h0:ident) => {
        impl<$h0> MaxGeneration for list_chain!($h0)
        where
            $h0: Max<typosaurus::num::consts::U0>,
        {
            type Out = <$h0 as Max<typosaurus::num::consts::U0>>::Output;
        }
    };
    ($h0:ident ; $h1:ident $(, $rest:ident)*) => {
        impl<$h0, $h1, $($rest,)*> MaxGeneration for list_chain!($h0, $h1 $(, $rest)*)
        where
            $h0: Max<$h1>,
            list_chain!($h1 $(, $rest)*): MaxGeneration,
            <$h0 as Max<$h1>>::Output:
                Max<<list_chain!($h1 $(, $rest)*) as MaxGeneration>::Out>,
        {
            type Out = <<$h0 as Max<$h1>>::Output as Max<
                <list_chain!($h1 $(, $rest)*) as MaxGeneration>::Out,
            >>::Output;
        }
    };
}
max_generation_len_impl!(H0);
max_generation_len_impl!(H0; H1);
max_generation_len_impl!(H0; H1, H2);
max_generation_len_impl!(H0; H1, H2, H3);
max_generation_len_impl!(H0; H1, H2, H3, H4);
max_generation_len_impl!(H0; H1, H2, H3, H4, H5);
max_generation_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail> MaxGeneration
    for list::List<(H0, list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail))>
where
    H0: Max<H1>,
    list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail): MaxGeneration,
    <H0 as Max<H1>>::Output:
        Max<<list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as MaxGeneration>::Out>,
{
    type Out = <<H0 as Max<H1>>::Output as Max<
        <list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail) as MaxGeneration>::Out,
    >>::Output;
}

pub type HighestGenerationForAnimals<T, AnimalId> =
    <GenerationsForAnimals<T, AnimalId> as MaxGeneration>::Out;
pub type HighestGeneration<E, AnimalId> =
    HighestGenerationForAnimals<<E as Ecosystem>::Animals, AnimalId>;

pub struct WithAnimalState;
impl<T> Mapper<T> for WithAnimalState
where
    T: Animal,
{
    type Out = <T as Animal>::State;
}

pub type AnimalStates<T> = <(AnimalSet<T>, WithAnimalState) as Map<
    <AnimalSet<T> as Container>::Content,
    WithAnimalState,
>>::Out;

pub type AnimalEffectMembers<T> =
    <SPFlatten<<AnimalSet<T> as CollectAnimalJourneyEffects>::Out> as StripEffectHeaders>::Out;

pub trait AnimalStatesCompatible<From>: Animals {}
impl<T, From> AnimalStatesCompatible<From> for T
where
    T: Animals,
    <T as Animals>::List: FlattenNodes,
    SPFlatten<<T as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T>: Container,
    (AnimalSet<T>, WithAnimalState): Map<<AnimalSet<T> as Container>::Content, WithAnimalState>,
    AnimalStates<T>: AllFrom<From>,
{
}

pub struct WithEffectFor<Context>(PhantomData<fn() -> Context>);
impl<T, Context> Mapper<T> for WithEffectFor<Context>
where
    T: Effect<Context>,
{
    type Out = ();
}

pub trait AnimalEffectCompatible<Context>: Animals {}
impl<T, Context> AnimalEffectCompatible<Context> for T
where
    T: Animals,
    <T as Animals>::List: FlattenNodes,
    SPFlatten<<T as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T>: CollectAnimalJourneyEffects,
    <AnimalSet<T> as CollectAnimalJourneyEffects>::Out: FlattenNodes,
    SPFlatten<<AnimalSet<T> as CollectAnimalJourneyEffects>::Out>: StripEffectHeaders,
    AnimalEffectMembers<T>: Container,
    (AnimalEffectMembers<T>, WithEffectFor<Context>):
        Map<<AnimalEffectMembers<T> as Container>::Content, WithEffectFor<Context>>,
{
}

pub trait CollectAnimalJourneyEffects {
    type Out;
}
impl CollectAnimalJourneyEffects for list::Empty {
    type Out = list::Empty;
}
macro_rules! collect_animal_journey_effects_len_impl {
    ($h0:ident) => {
        impl<$h0> CollectAnimalJourneyEffects for list_chain!($h0)
        where
            $h0: BoundAnimal,
            BoundAnimalJourney<$h0>: Journey,
            BoundAnimalJourney<$h0>: JourneyEffects,
            <BoundAnimalJourney<$h0> as JourneyEffects>::List: FlattenNodes,
            SPFlatten<<BoundAnimalJourney<$h0> as JourneyEffects>::List>: KeepEffectNodes,
        {
            type Out = list_chain!(
                <SPFlatten<<BoundAnimalJourney<$h0> as JourneyEffects>::List> as KeepEffectNodes>::Out
            );
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<$h0, $($rest,)+> CollectAnimalJourneyEffects for list_chain!($h0, $($rest),+)
        where
            $h0: BoundAnimal,
            BoundAnimalJourney<$h0>: Journey,
            BoundAnimalJourney<$h0>: JourneyEffects,
            <BoundAnimalJourney<$h0> as JourneyEffects>::List: FlattenNodes,
            SPFlatten<<BoundAnimalJourney<$h0> as JourneyEffects>::List>: KeepEffectNodes,
            list_chain!($($rest),+): CollectAnimalJourneyEffects,
        {
            type Out = list_chain_tail!(
                <SPFlatten<<BoundAnimalJourney<$h0> as JourneyEffects>::List> as KeepEffectNodes>::Out ;
                <list_chain!($($rest),+) as CollectAnimalJourneyEffects>::Out
            );
        }
    };
}
collect_animal_journey_effects_len_impl!(H0);
collect_animal_journey_effects_len_impl!(H0; H1);
collect_animal_journey_effects_len_impl!(H0; H1, H2);
collect_animal_journey_effects_len_impl!(H0; H1, H2, H3);
collect_animal_journey_effects_len_impl!(H0; H1, H2, H3, H4);
collect_animal_journey_effects_len_impl!(H0; H1, H2, H3, H4, H5);
collect_animal_journey_effects_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<H0, H1, H2, H3, H4, H5, H6, H7, Tail> CollectAnimalJourneyEffects
    for list::List<(H0, list_chain_tail!(H1, H2, H3, H4, H5, H6, H7 ; Tail))>
where
    H0: BoundAnimal,
    BoundAnimalJourney<H0>: Journey,
    BoundAnimalJourney<H0>: JourneyEffects,
    <BoundAnimalJourney<H0> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H0> as JourneyEffects>::List>: KeepEffectNodes,
    H1: BoundAnimal,
    BoundAnimalJourney<H1>: Journey,
    BoundAnimalJourney<H1>: JourneyEffects,
    <BoundAnimalJourney<H1> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H1> as JourneyEffects>::List>: KeepEffectNodes,
    H2: BoundAnimal,
    BoundAnimalJourney<H2>: Journey,
    BoundAnimalJourney<H2>: JourneyEffects,
    <BoundAnimalJourney<H2> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H2> as JourneyEffects>::List>: KeepEffectNodes,
    H3: BoundAnimal,
    BoundAnimalJourney<H3>: Journey,
    BoundAnimalJourney<H3>: JourneyEffects,
    <BoundAnimalJourney<H3> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H3> as JourneyEffects>::List>: KeepEffectNodes,
    H4: BoundAnimal,
    BoundAnimalJourney<H4>: Journey,
    BoundAnimalJourney<H4>: JourneyEffects,
    <BoundAnimalJourney<H4> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H4> as JourneyEffects>::List>: KeepEffectNodes,
    H5: BoundAnimal,
    BoundAnimalJourney<H5>: Journey,
    BoundAnimalJourney<H5>: JourneyEffects,
    <BoundAnimalJourney<H5> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H5> as JourneyEffects>::List>: KeepEffectNodes,
    H6: BoundAnimal,
    BoundAnimalJourney<H6>: Journey,
    BoundAnimalJourney<H6>: JourneyEffects,
    <BoundAnimalJourney<H6> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H6> as JourneyEffects>::List>: KeepEffectNodes,
    H7: BoundAnimal,
    BoundAnimalJourney<H7>: Journey,
    BoundAnimalJourney<H7>: JourneyEffects,
    <BoundAnimalJourney<H7> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<H7> as JourneyEffects>::List>: KeepEffectNodes,
    Tail: CollectAnimalJourneyEffects,
{
    type Out = list_chain_tail!(
        <SPFlatten<<BoundAnimalJourney<H0> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H1> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H2> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H3> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H4> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H5> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H6> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        <SPFlatten<<BoundAnimalJourney<H7> as JourneyEffects>::List> as KeepEffectNodes>::Out ;
        <Tail as CollectAnimalJourneyEffects>::Out
    );
}
