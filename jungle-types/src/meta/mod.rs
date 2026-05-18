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

pub trait AllFrom<T> {}
impl<T> AllFrom<T> for list::Empty {}
impl<T, A, B> AllFrom<T> for list::List<(A, B)>
where
    T: Into<A>,
    B: AllFrom<T>,
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
    ($h:ident) => {
        list::List<($h, list::Empty)>
    };
    ($h:ident, $($rest:ident),+) => {
        list::List<($h, list_chain!($($rest),+))>
    };
}
macro_rules! list_chain_tail {
    ($h:ident ; $tail:ty) => {
        list::List<($h, $tail)>
    };
    ($h:ident, $($rest:ident),+ ; $tail:ty) => {
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
impl<Head, Tail> UniqueAnimalVersions for list::List<(Head, Tail)>
where
    Tail: ContainsAnimalVersion<Head>,
    <Tail as ContainsAnimalVersion<Head>>::Out: Not,
    Tail: UniqueAnimalVersions,
    (
        <<Tail as ContainsAnimalVersion<Head>>::Out as Not>::Out,
        <Tail as UniqueAnimalVersions>::Out,
    ): And,
{
    type Out = <(
        <<Tail as ContainsAnimalVersion<Head>>::Out as Not>::Out,
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
impl<Head, Tail, TailOut> CollectAnimalJourneyEffects for list::List<(Head, Tail)>
where
    Head: BoundAnimal,
    BoundAnimalJourney<Head>: Journey,
    BoundAnimalJourney<Head>: JourneyEffects,
    <BoundAnimalJourney<Head> as JourneyEffects>::List: FlattenNodes,
    SPFlatten<<BoundAnimalJourney<Head> as JourneyEffects>::List>: KeepEffectNodes,
    Tail: CollectAnimalJourneyEffects<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<BoundAnimalJourney<Head> as JourneyEffects>::List> as KeepEffectNodes>::Out,
        TailOut,
    )>;
}
