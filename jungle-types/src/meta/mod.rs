use typosaurus::cmp::Equality;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
    Container,
};
use typosaurus::num::Unsigned;
use typosaurus::traits::functor::{Map, Mapper};

use super::{Actions, Creature, Creatures, FlowActions, Instinct};

/// Newtype wrapper around an Unsigned constant.
pub struct Id<T: Unsigned>(pub T);

/// Type-level empty list used by collection metadata in this crate.
pub type EmptyList = typosaurus::collections::list::Empty;

/// Type-level append trait for metadata list composition.
pub use typosaurus::traits::semigroup::Mappend as ListMappend;

/// Type-level append output of two metadata lists.
pub type Merge<Lhs, Rhs> = <(Lhs, Rhs) as ListMappend>::Out;

/// Blanket impl: `Id<T>` is equal to `Id<U>` iff `T` is equal to `U`.
impl<T, U> Equality<Id<U>> for Id<T>
where
    T: Unsigned + Equality<U>,
    U: Unsigned,
{
    type Out = <T as Equality<U>>::Out;
}

pub trait ActionMember {}
pub trait CreatureMember {}

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

pub trait StripCreatureHeaders {
    type Out;
}
impl StripCreatureHeaders for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> StripCreatureHeaders for list::List<(Node<K, ()>, Tail)>
where
    Tail: StripCreatureHeaders<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> StripCreatureHeaders for list::List<(Node<K, Head>, Tail)>
where
    Head: CreatureMember,
    Tail: StripCreatureHeaders<Out = TailOut>,
{
    type Out = list::List<(Head, TailOut)>;
}

pub type ActionSet<T> = <SPFlatten<<T as Actions>::List> as StripActionHeaders>::Out;
pub type CreatureSet<T> = <SPFlatten<<T as Creatures>::List> as StripCreatureHeaders>::Out;

pub struct WithCreatureState;
impl<T> Mapper<T> for WithCreatureState
where
    T: Creature,
{
    type Out = <T as Creature>::State;
}

pub type CreatureStates<T> = <(CreatureSet<T>, WithCreatureState) as Map<
    <CreatureSet<T> as Container>::Content,
    WithCreatureState,
>>::Out;

pub trait CreatureStatesCompatible<From>: Creatures {}
impl<T, From> CreatureStatesCompatible<From> for T
where
    T: Creatures,
    <T as Creatures>::List: FlattenNodes,
    SPFlatten<<T as Creatures>::List>: StripCreatureHeaders,
    CreatureSet<T>: Container,
    (CreatureSet<T>, WithCreatureState):
        Map<<CreatureSet<T> as Container>::Content, WithCreatureState>,
    CreatureStates<T>: AllFrom<From>,
{
}

pub trait CollectCreatureInstinctActions {
    type Out;
}
impl CollectCreatureInstinctActions for list::Empty {
    type Out = list::Empty;
}
impl<Head, Tail, TailOut> CollectCreatureInstinctActions for list::List<(Head, Tail)>
where
    Head: Creature,
    <Head as Creature>::Instinct: Instinct,
    <Head as Creature>::Instinct: FlowActions,
    <<Head as Creature>::Instinct as FlowActions>::List: FlattenNodes,
    SPFlatten<<<Head as Creature>::Instinct as FlowActions>::List>: KeepActionNodes,
    Tail: CollectCreatureInstinctActions<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<<Head as Creature>::Instinct as FlowActions>::List> as KeepActionNodes>::Out,
        TailOut,
    )>;
}

pub type CreatureActionSet<T> = <SPDedupNodes<
    SPFlatten<<CreatureSet<T> as CollectCreatureInstinctActions>::Out>,
> as StripActionHeaders>::Out;
