use typosaurus::bool::{And, Not, Truthy};
use typosaurus::cmp::Equality;
use typosaurus::cmp::IsEqual;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
    Container,
};
use typosaurus::num::Unsigned;
use typosaurus::traits::fold::Foldable;
use typosaurus::traits::functor::{Map, Mapper};

use super::{Action, Actions, Animal, Animals, Ecosystem, FlowActions, Journey};
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

pub trait ActionMember {}
pub trait AnimalMember {}

pub trait AllFrom<T> {}
impl<T> AllFrom<T> for list::Empty {}
impl<T, A, B> AllFrom<T> for list::List<(A, B)>
where
    T: Into<A>,
    B: AllFrom<T>,
{
}

pub trait StripActionHeaders {
    type Out;
}
impl StripActionHeaders for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> StripActionHeaders for list::List<(Node<K, ()>, Tail)>
where
    Tail: StripActionHeaders<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> StripActionHeaders for list::List<(Node<K, Head>, Tail)>
where
    Head: ActionMember,
    Tail: StripActionHeaders<Out = TailOut>,
{
    type Out = list::List<(Head, TailOut)>;
}

pub trait KeepActionNodes {
    type Out;
}
impl KeepActionNodes for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> KeepActionNodes for list::List<(Node<K, ()>, Tail)>
where
    Tail: KeepActionNodes<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> KeepActionNodes for list::List<(Node<K, Head>, Tail)>
where
    Head: ActionMember,
    Tail: KeepActionNodes<Out = TailOut>,
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

pub type ActionSet<T> = <SPFlatten<<T as Actions>::List> as StripActionHeaders>::Out;
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

pub struct WithActionDependency;
impl<T> Mapper<T> for WithActionDependency
where
    T: Action,
{
    type Out = <T as Action>::Dependency;
}

pub type AnimalActionMembers<T> =
    <SPFlatten<<AnimalSet<T> as CollectAnimalJourneyActions>::Out> as StripActionHeaders>::Out;

pub type AnimalActionDependencies<T> = <(AnimalActionMembers<T>, WithActionDependency) as Map<
    <AnimalActionMembers<T> as Container>::Content,
    WithActionDependency,
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

pub trait AnimalActionDependenciesCompatible<From>: Animals {}
impl<T, From> AnimalActionDependenciesCompatible<From> for T
where
    T: Animals,
    <T as Animals>::List: FlattenNodes,
    SPFlatten<<T as Animals>::List>: StripAnimalHeaders,
    AnimalSet<T>: CollectAnimalJourneyActions,
    <AnimalSet<T> as CollectAnimalJourneyActions>::Out: FlattenNodes,
    SPFlatten<<AnimalSet<T> as CollectAnimalJourneyActions>::Out>: StripActionHeaders,
    AnimalActionMembers<T>: Container,
    (AnimalActionMembers<T>, WithActionDependency):
        Map<<AnimalActionMembers<T> as Container>::Content, WithActionDependency>,
    AnimalActionDependencies<T>: AllFrom<From>,
{
}

pub trait CollectAnimalJourneyActions {
    type Out;
}
impl CollectAnimalJourneyActions for list::Empty {
    type Out = list::Empty;
}
impl<Head, Tail, TailOut> CollectAnimalJourneyActions for list::List<(Head, Tail)>
where
    Head: Animal,
    <Head as Animal>::Journey: Journey,
    <Head as Animal>::Journey: FlowActions,
    <<Head as Animal>::Journey as FlowActions>::List: FlattenNodes,
    SPFlatten<<<Head as Animal>::Journey as FlowActions>::List>: KeepActionNodes,
    Tail: CollectAnimalJourneyActions<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<<Head as Animal>::Journey as FlowActions>::List> as KeepActionNodes>::Out,
        TailOut,
    )>;
}

pub type AnimalActionSet<T> = <SPDedupNodes<
    SPFlatten<<AnimalSet<T> as CollectAnimalJourneyActions>::Out>,
> as StripActionHeaders>::Out;
