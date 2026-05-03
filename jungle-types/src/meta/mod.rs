use crate::{Action, Actions, Animal, Animals};
use typosaurus::cmp::Equality;
use typosaurus::collections::list;
use typosaurus::num::Unsigned;
use typosaurus::traits::semigroup::Mappend;

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

impl<T> Animals for T
where
    T: Animal,
{
    type List = typosaurus::list![T];
}

impl Animals for list::Empty {
    type List = list::Empty;
}

impl<Head, Tail> Animals for list::List<(Head, Tail)>
where
    Head: Animals,
    Tail: Animals,
    (Head::List, Tail::List): Mappend,
{
    type List = <(Head::List, Tail::List) as Mappend>::Out;
}

impl<Left, Right> Animals for (Left, Right)
where
    Left: Animals,
    Right: Animals,
    (Left::List, Right::List): Mappend,
{
    type List = <(Left::List, Right::List) as Mappend>::Out;
}

impl<T> Actions for T
where
    T: Action,
{
    type List = typosaurus::list![T];
}

impl Actions for list::Empty {
    type List = list::Empty;
}

impl<Head, Tail> Actions for list::List<(Head, Tail)>
where
    Head: Actions,
    Tail: Actions,
    (Head::List, Tail::List): Mappend,
{
    type List = <(Head::List, Tail::List) as Mappend>::Out;
}

impl<Left, Right> Actions for (Left, Right)
where
    Left: Actions,
    Right: Actions,
    (Left::List, Right::List): Mappend,
{
    type List = <(Left::List, Right::List) as Mappend>::Out;
}

#[cfg(test)]
mod tests {
    use crate::{Action, Actions, Animal, Animals, Id, Instinct};
    use typosaurus::assert_type_eq;
    use typosaurus::collections::list::{Atom, DeepFlatten};
    use typosaurus::num::consts::{U0, U1};

    macro_rules! animal {
        ($name:ident, $id:ty) => {
            struct $name;
            impl Animal for $name {
                type Id = Id<$id>;
                type Instinct = ();
            }
        };
    }

    animal!(AnimalA, U0);
    animal!(AnimalB, U1);

    type NestedAnimals = typosaurus::list![
        typosaurus::list![Atom<AnimalA>, Atom<AnimalB>],
        typosaurus::list![Atom<AnimalA>],
        Atom<AnimalB>
    ];
    type FlatAnimals = DeepFlatten<NestedAnimals>;

    assert_type_eq!(
        FlatAnimals,
        typosaurus::list![AnimalA, AnimalB, AnimalA, AnimalB]
    );

    struct Hunt;
    struct Sleep;

    impl Action for Hunt {
        type Id = Id<U0>;
        type State = ();
        type In = ();
        type Out = ();
        type Err = ();

        async fn act(_state: &Self::State, _input: Self::In) -> Result<Self::Out, Self::Err> {
            Ok(())
        }
    }

    impl Action for Sleep {
        type Id = Id<U1>;
        type State = ();
        type In = ();
        type Out = ();
        type Err = ();

        async fn act(_state: &Self::State, _input: Self::In) -> Result<Self::Out, Self::Err> {
            Ok(())
        }
    }

    struct Cat;
    struct Dog;
    struct CatInstinct;
    struct DogInstinct;

    impl Instinct for CatInstinct {
        type Actions = ();
    }

    impl Instinct for DogInstinct {
        type Actions = ();
    }

    impl Animal for Cat {
        type Id = Id<U0>;
        type Instinct = CatInstinct;
    }

    impl Animal for Dog {
        type Id = Id<U1>;
        type Instinct = DogInstinct;
    }

    #[test]
    fn animals_list_is_flat_for_nested_groupings() {
        type Grouping =
            typosaurus::list![Cat, typosaurus::list![Dog], (Cat, typosaurus::list![Dog])];

        assert_type_eq!(
            <Grouping as Animals>::List,
            typosaurus::list![Cat, Dog, Cat, Dog]
        );
    }

    #[test]
    fn actions_list_is_flat_for_nested_groupings() {
        type Grouping = typosaurus::list![
            Hunt,
            typosaurus::list![Sleep],
            (Hunt, typosaurus::list![Sleep])
        ];

        assert_type_eq!(
            <Grouping as Actions>::List,
            typosaurus::list![Hunt, Sleep, Hunt, Sleep]
        );
    }
}
