use typosaurus::cmp::Equality;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
    Container,
};
use typosaurus::num::Unsigned;
use typosaurus::traits::functor::{Map, Mapper};

use super::{Action, Actions, Animal, Animals, FlowActions, Journey};

/// Newtype wrapper around an Unsigned constant.
pub struct Id<T: Unsigned>(pub T);

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

pub type AnimalActionDependencies<T> =
    <(AnimalActionMembers<T>, WithActionDependency) as Map<
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
    (AnimalSet<T>, WithAnimalState):
        Map<<AnimalSet<T> as Container>::Content, WithAnimalState>,
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
