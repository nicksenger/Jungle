use typosaurus::cmp::Equality;
use typosaurus::collections::{
    list,
    sp::{FlattenNodes, Node, SPDedupNodes, SPFlatten},
};
use typosaurus::num::Unsigned;

use super::{Actions, Animal, Animals, Instinct};

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

pub trait CollectAnimalInstinctActions {
    type Out;
}
impl CollectAnimalInstinctActions for list::Empty {
    type Out = list::Empty;
}
impl<Head, Tail, TailOut> CollectAnimalInstinctActions for list::List<(Head, Tail)>
where
    Head: Animal,
    <Head as Animal>::Instinct: Instinct,
    <<Head as Animal>::Instinct as Instinct>::Actions: Actions,
    <<<Head as Animal>::Instinct as Instinct>::Actions as Actions>::List: FlattenNodes,
    SPFlatten<<<<Head as Animal>::Instinct as Instinct>::Actions as Actions>::List>:
        KeepActionNodes,
    Tail: CollectAnimalInstinctActions<Out = TailOut>,
{
    type Out = list::List<(
        <SPFlatten<<<<Head as Animal>::Instinct as Instinct>::Actions as Actions>::List> as KeepActionNodes>::Out,
        TailOut,
    )>;
}

pub type AnimalActionSet<T> = <SPDedupNodes<
    SPFlatten<<AnimalSet<T> as CollectAnimalInstinctActions>::Out>,
> as StripActionHeaders>::Out;
