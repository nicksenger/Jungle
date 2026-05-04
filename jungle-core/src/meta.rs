use jungle_types::{Actions, Animal, Animals, Ecosystem, Instinct};
use typosaurus::bool::{And, Truthy};
use typosaurus::collections::{list, Container};
use typosaurus::traits::monoid::Mempty;
use typosaurus::traits::semigroup::Mappend;

use crate::Jungle;

trait AnimalActionList {
    type List;
}

impl AnimalActionList for list::Empty {
    type List = list::Empty;
}

impl<Head, Tail> AnimalActionList for list::List<(Head, Tail)>
where
    Head: Animal,
    Head::Instinct: Instinct,
    <Head::Instinct as Instinct>::Actions: Actions,
    Tail: AnimalActionList,
    (
        <<Head::Instinct as Instinct>::Actions as Actions>::List,
        <Tail as AnimalActionList>::List,
    ): Mappend,
{
    type List = <(
        <<Head::Instinct as Instinct>::Actions as Actions>::List,
        <Tail as AnimalActionList>::List,
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

impl<Head, Tail, Superset> IsSubsetOf<Superset> for list::List<(Head, Tail)>
where
    (Superset, Head): list::IsContainedBy,
    Tail: IsSubsetOf<Superset>,
    (<(Superset, Head) as list::IsContainedBy>::Out, <Tail as IsSubsetOf<Superset>>::Out): And,
{
    type Out = <(
        <(Superset, Head) as list::IsContainedBy>::Out,
        <Tail as IsSubsetOf<Superset>>::Out,
    ) as And>::Out;
}

impl<T> Jungle for T
where
    T: Ecosystem,
    T::Animals: Animals,
    T::Actions: Actions,
    <T::Animals as Animals>::List: Container,
    <T::Actions as Actions>::List: Container,
    <T::Animals as Animals>::List: AnimalActionList,
    <<T::Animals as Animals>::List as AnimalActionList>::List: IsSubsetOf<<T::Actions as Actions>::List>,
    <<<T::Animals as Animals>::List as AnimalActionList>::List as IsSubsetOf<
        <T::Actions as Actions>::List,
    >>::Out: Truthy,
{
    type Animals = <T::Animals as Animals>::List;

    async fn manifest(self) -> Result<(), jungle_types::Error> {
        drop(self);
        Ok(())
    }
}
