use typosaurus::bool::{And, Not, Truthy};
use typosaurus::cmp::Equality;
use typosaurus::cmp::IsEqual;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
    Container,
};
use typosaurus::num::{Max, Unsigned};
use typosaurus::traits::fold::Foldable;
use typosaurus::traits::functor::{Map, Mapper};

use super::{Animal, Animals, Ecosystem, Effect, Effects, FlowEffects, Journey};
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

pub trait EffectMember {}
pub trait AnimalMember {}

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
impl<Head, Tail, Target> ContainsAnimalVersion<Target> for list::List<(Head, Tail)>
where
    Head: Equality<Target>,
    Tail: ContainsAnimalVersion<Target>,
    (
        <Head as Equality<Target>>::Out,
        <Tail as ContainsAnimalVersion<Target>>::Out,
    ): typosaurus::bool::Or,
{
    type Out = <(
        <Head as Equality<Target>>::Out,
        <Tail as ContainsAnimalVersion<Target>>::Out,
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
impl<Head, Tail> MaxGeneration for list::List<(Head, Tail)>
where
    Tail: MaxGeneration,
    Head: Max<<Tail as MaxGeneration>::Out>,
{
    type Out = <Head as Max<<Tail as MaxGeneration>::Out>>::Output;
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

pub struct WithEffectDependency;
impl<T> Mapper<T> for WithEffectDependency
where
    T: Effect,
{
    type Out = <T as Effect>::Dependency;
}

pub type AnimalEffectMembers<T> =
    <SPFlatten<<AnimalSet<T> as CollectAnimalJourneyEffects>::Out> as StripEffectHeaders>::Out;

pub type AnimalEffectDependencies<T> = <(AnimalEffectMembers<T>, WithEffectDependency) as Map<
    <AnimalEffectMembers<T> as Container>::Content,
    WithEffectDependency,
>>::Out;

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

pub trait AnimalEffectDependenciesCompatible<From>: Animals {}
impl<T, From> AnimalEffectDependenciesCompatible<From> for T
where
    T: Animals,
    <T as Animals>::List: FlattenNodes,
    SPFlatten<<T as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T>: CollectAnimalJourneyEffects,
    <AnimalSet<T> as CollectAnimalJourneyEffects>::Out: FlattenNodes,
    SPFlatten<<AnimalSet<T> as CollectAnimalJourneyEffects>::Out>: StripEffectHeaders,
    AnimalEffectMembers<T>: Container,
    (AnimalEffectMembers<T>, WithEffectDependency):
        Map<<AnimalEffectMembers<T> as Container>::Content, WithEffectDependency>,
    AnimalEffectDependencies<T>: AllFrom<From>,
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
    Head: Animal,
    <Head as Animal>::Journey: Journey,
    <Head as Animal>::Journey: FlowEffects,
    <<Head as Animal>::Journey as FlowEffects>::List: FlattenNodes,
    SPFlatten<<<Head as Animal>::Journey as FlowEffects>::List>: KeepEffectNodes,
    Tail: CollectAnimalJourneyEffects<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<<Head as Animal>::Journey as FlowEffects>::List> as KeepEffectNodes>::Out,
        TailOut,
    )>;
}

pub type AnimalEffectSet<T> = <SPDedupNodes<
    SPFlatten<<AnimalSet<T> as CollectAnimalJourneyEffects>::Out>,
> as StripEffectHeaders>::Out;
