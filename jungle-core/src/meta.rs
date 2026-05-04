use jungle_types::{Action, Actions, Animal, Animals, Ecosystem, Instinct};
use typosaurus::bool::{And, Or, Truthy, False};
use typosaurus::cmp::IsEqual;
use typosaurus::collections::{list, Container};
use typosaurus::traits::functor::{Map, Mapper};
use typosaurus::traits::monoid::Mempty;
use typosaurus::traits::semigroup::Mappend;

use crate::Jungle;

struct ActionIdOf;

impl<T> Mapper<T> for ActionIdOf
where
    T: Action,
{
    type Out = <T as Action>::Id;
}

trait ActionIdList {
    type List;
}

impl<T> ActionIdList for T
where
    T: Actions,
    <T as Actions>::List: Container,
    (<T as Actions>::List, ActionIdOf): Map<<<T as Actions>::List as Container>::Content, ActionIdOf>,
{
    type List =
        <(<T as Actions>::List, ActionIdOf) as Map<<<T as Actions>::List as Container>::Content, ActionIdOf>>::Out;
}

trait AnimalActionIdList {
    type List;
}

impl AnimalActionIdList for list::Empty {
    type List = list::Empty;
}

impl<Head> AnimalActionIdList for list::List<(Head, list::Empty)>
where
    Head: Animal,
    Head::Instinct: Instinct,
    <Head::Instinct as Instinct>::Actions: ActionIdList,
{
    type List = <<Head::Instinct as Instinct>::Actions as ActionIdList>::List;
}

impl<Head1, Head2, Tail> AnimalActionIdList for list::List<(Head1, list::List<(Head2, Tail)>)>
where
    Head1: Animal,
    Head1::Instinct: Instinct,
    <Head1::Instinct as Instinct>::Actions: ActionIdList,
    Head2: Animal,
    Head2::Instinct: Instinct,
    <Head2::Instinct as Instinct>::Actions: ActionIdList,
    Tail: AnimalActionIdList,
    (
        <<Head1::Instinct as Instinct>::Actions as ActionIdList>::List,
        <<Head2::Instinct as Instinct>::Actions as ActionIdList>::List,
    ): Mappend,
    (
        <(
            <<Head1::Instinct as Instinct>::Actions as ActionIdList>::List,
            <<Head2::Instinct as Instinct>::Actions as ActionIdList>::List,
        ) as Mappend>::Out,
        <Tail as AnimalActionIdList>::List,
    ): Mappend,
{
    type List = <(
        <(
            <<Head1::Instinct as Instinct>::Actions as ActionIdList>::List,
            <<Head2::Instinct as Instinct>::Actions as ActionIdList>::List,
        ) as Mappend>::Out,
        <Tail as AnimalActionIdList>::List,
    ) as Mappend>::Out;
}

trait IsSubsetOf<Superset> {
    type Out;
}

impl<Superset> IsSubsetOf<Superset> for list::Empty
where
    typosaurus::bool::monoid::Both: Mempty,
{
    type Out = <typosaurus::bool::monoid::Both as Mempty>::Out;
}

impl<Head, Superset> IsSubsetOf<Superset> for list::List<(Head, list::Empty)>
where
    Superset: IsContainedByEq<Head>,
{
    type Out = <Superset as IsContainedByEq<Head>>::Out;
}

impl<Head1, Head2, Tail, Superset> IsSubsetOf<Superset>
    for list::List<(Head1, list::List<(Head2, Tail)>)>
where
    Superset: IsContainedByEq<Head1>,
    Superset: IsContainedByEq<Head2>,
    Tail: IsSubsetOf<Superset>,
    (<Superset as IsContainedByEq<Head1>>::Out, <Superset as IsContainedByEq<Head2>>::Out): And,
    (
        <(
            <Superset as IsContainedByEq<Head1>>::Out,
            <Superset as IsContainedByEq<Head2>>::Out,
        ) as And>::Out,
        <Tail as IsSubsetOf<Superset>>::Out,
    ): And,
{
    type Out = <(
        <(
            <Superset as IsContainedByEq<Head1>>::Out,
            <Superset as IsContainedByEq<Head2>>::Out,
        ) as And>::Out,
        <Tail as IsSubsetOf<Superset>>::Out,
    ) as And>::Out;
}

trait IsContainedByEq<T> {
    type Out;
}

impl<T> IsContainedByEq<T> for list::Empty {
    type Out = False;
}

impl<Head, Tail, T> IsContainedByEq<T> for list::List<(Head, Tail)>
where
    (T, Head): IsEqual,
    Tail: IsContainedByEq<T>,
    (<(T, Head) as IsEqual>::Out, <Tail as IsContainedByEq<T>>::Out): Or,
{
    type Out = <(<(T, Head) as IsEqual>::Out, <Tail as IsContainedByEq<T>>::Out) as Or>::Out;
}

impl<T> Jungle for T
where
    T: Ecosystem,
    T::Animals: Animals,
    T::Actions: Actions,
    <T::Animals as Animals>::List: Container,
    <T::Actions as Actions>::List: Container,
    <T::Animals as Animals>::List: AnimalActionIdList,
    T::Actions: ActionIdList,
    <<T::Animals as Animals>::List as AnimalActionIdList>::List:
        IsSubsetOf<<T::Actions as ActionIdList>::List>,
    <<<T::Animals as Animals>::List as AnimalActionIdList>::List as IsSubsetOf<
        <T::Actions as ActionIdList>::List,
    >>::Out: Truthy,
{
    type Animals = <T::Animals as Animals>::List;

    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
