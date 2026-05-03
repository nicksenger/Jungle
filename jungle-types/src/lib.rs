mod behavior;
mod meta;
pub use behavior::{Action, Instinct};
pub use meta::Id;
use typosaurus::collections::list;
use typosaurus::traits::semigroup::Mappend;

/// A collection of Jungle entities and the Animals that fill them.
///
/// This is defined before [`Animal`] so downstream types can reference it
/// when specifying associated collections.
pub trait Ecosystem {
    type Actions;
    type Animals;
}

/// A living creature within the Jungle ecosystem.
pub trait Animal {
    /// A type-level identifier for this Animal.
    type Id;

    /// The fundamental behavior of this Animal.
    type Instinct;

    /// The actions this Animal can take.
    type Actions;
}

/// An organism that hosts symbionts.
pub trait Host {
    /// Organisms that live in close association with this Host.
    type Symbionts;
}

/// A trait that transforms a stream of inputs into a stream of outputs.
pub trait Evoke {
    /// The input type accepted by this evoke.
    type In;

    /// The output type produced by this evoke.
    type Out;

    /// Process a stream of inputs, yielding a stream of outputs.
    fn evoke(self, input: impl futures::Stream<Item = Self::In>) -> impl futures::Stream<Item = Self::Out>;
}

/// Any collection of [`Animal`]s with a flat type-level list of members.
pub trait Animals {
    type List;
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

/// Any collection of [`Action`]s with a flat type-level list of members.
pub trait Actions {
    type List;
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
    use super::{Action, Actions, Animal, Animals, Id, Instinct};
    use typosaurus::assert_type_eq;
    use typosaurus::num::consts::{U0, U1};

    struct Hunt;
    struct Sleep;

    impl Action for Hunt {
        type Id = Id<U0>;
        type In = ();
        type Out = ();
        type Err = ();

        async fn act(_input: Self::In) -> Result<Self::Out, Self::Err> {
            Ok(())
        }
    }

    impl Action for Sleep {
        type Id = Id<U1>;
        type In = ();
        type Out = ();
        type Err = ();

        async fn act(_input: Self::In) -> Result<Self::Out, Self::Err> {
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
        type Actions = Hunt;
    }

    impl Animal for Dog {
        type Id = Id<U1>;
        type Instinct = DogInstinct;
        type Actions = Sleep;
    }

    #[test]
    fn animals_list_is_flat_for_nested_groupings() {
        type Grouping = typosaurus::list![
            Cat,
            typosaurus::list![Dog],
            (Cat, typosaurus::list![Dog])
        ];

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
