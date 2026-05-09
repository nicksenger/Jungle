use typosaurus::cmp::Equality;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
    Container,
};
use typosaurus::num::Unsigned;
use typosaurus::traits::functor::{Map, Mapper};

use super::{Action, Actions, Anima, Animas, FlowActions, Instinct};

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
pub trait AnimaMember {}

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

pub trait StripAnimaHeaders {
    type Out;
}
impl StripAnimaHeaders for list::Empty {
    type Out = list::Empty;
}
impl<K, Tail, TailOut> StripAnimaHeaders for list::List<(Node<K, ()>, Tail)>
where
    Tail: StripAnimaHeaders<Out = TailOut>,
{
    type Out = TailOut;
}
impl<K, Head, Tail, TailOut> StripAnimaHeaders for list::List<(Node<K, Head>, Tail)>
where
    Head: AnimaMember,
    Tail: StripAnimaHeaders<Out = TailOut>,
{
    type Out = list::List<(Head, TailOut)>;
}

pub type ActionSet<T> = <SPFlatten<<T as Actions>::List> as StripActionHeaders>::Out;
pub type AnimaSet<T> = <SPFlatten<<T as Animas>::List> as StripAnimaHeaders>::Out;

pub struct WithAnimaState;
impl<T> Mapper<T> for WithAnimaState
where
    T: Anima,
{
    type Out = <T as Anima>::State;
}

pub type AnimaStates<T> = <(AnimaSet<T>, WithAnimaState) as Map<
    <AnimaSet<T> as Container>::Content,
    WithAnimaState,
>>::Out;

pub struct WithActionDependency;
impl<T> Mapper<T> for WithActionDependency
where
    T: Action,
{
    type Out = <T as Action>::Dependency;
}

pub type AnimaActionMembers<T> =
    <SPFlatten<<AnimaSet<T> as CollectAnimaInstinctActions>::Out> as StripActionHeaders>::Out;

pub type AnimaActionDependencies<T> =
    <(AnimaActionMembers<T>, WithActionDependency) as Map<
        <AnimaActionMembers<T> as Container>::Content,
        WithActionDependency,
    >>::Out;

pub trait AnimaStatesCompatible<From>: Animas {}
impl<T, From> AnimaStatesCompatible<From> for T
where
    T: Animas,
    <T as Animas>::List: FlattenNodes,
    SPFlatten<<T as Animas>::List>: StripAnimaHeaders,
    AnimaSet<T>: Container,
    (AnimaSet<T>, WithAnimaState):
        Map<<AnimaSet<T> as Container>::Content, WithAnimaState>,
    AnimaStates<T>: AllFrom<From>,
{
}

pub trait AnimaActionDependenciesCompatible<From>: Animas {}
impl<T, From> AnimaActionDependenciesCompatible<From> for T
where
    T: Animas,
    <T as Animas>::List: FlattenNodes,
    SPFlatten<<T as Animas>::List>: StripAnimaHeaders,
    AnimaSet<T>: CollectAnimaInstinctActions,
    <AnimaSet<T> as CollectAnimaInstinctActions>::Out: FlattenNodes,
    SPFlatten<<AnimaSet<T> as CollectAnimaInstinctActions>::Out>: StripActionHeaders,
    AnimaActionMembers<T>: Container,
    (AnimaActionMembers<T>, WithActionDependency):
        Map<<AnimaActionMembers<T> as Container>::Content, WithActionDependency>,
    AnimaActionDependencies<T>: AllFrom<From>,
{
}

pub trait CollectAnimaInstinctActions {
    type Out;
}
impl CollectAnimaInstinctActions for list::Empty {
    type Out = list::Empty;
}
impl<Head, Tail, TailOut> CollectAnimaInstinctActions for list::List<(Head, Tail)>
where
    Head: Anima,
    <Head as Anima>::Instinct: Instinct,
    <Head as Anima>::Instinct: FlowActions,
    <<Head as Anima>::Instinct as FlowActions>::List: FlattenNodes,
    SPFlatten<<<Head as Anima>::Instinct as FlowActions>::List>: KeepActionNodes,
    Tail: CollectAnimaInstinctActions<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<<Head as Anima>::Instinct as FlowActions>::List> as KeepActionNodes>::Out,
        TailOut,
    )>;
}

pub type AnimaActionSet<T> = <SPDedupNodes<
    SPFlatten<<AnimaSet<T> as CollectAnimaInstinctActions>::Out>,
> as StripActionHeaders>::Out;
