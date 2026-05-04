use typosaurus::cmp::Equality;
use typosaurus::num::Unsigned;

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

#[cfg(test)]
mod tests {
    use crate::{Action, Actions, Animal, Animals, Id, Instinct, JungleActions, JungleAnimals};
    use inception::{primitive, Inception};
    use typosaurus::assert_type_eq;
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

    struct Hunt;
    struct Sleep;

    impl Action for Hunt {
        type Id = Id<U0>;
        type State = ();
        type In = ();
        type Out = ();
        type Err = ();

        fn act(
            _state: &Self::State,
            _input: Self::In,
        ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
            std::future::ready(Ok(()))
        }
    }

    impl Action for Sleep {
        type Id = Id<U1>;
        type State = ();
        type In = ();
        type Out = ();
        type Err = ();

        fn act(
            _state: &Self::State,
            _input: Self::In,
        ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
            std::future::ready(Ok(()))
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

    #[primitive(property = JungleAnimals)]
    impl Animals for Cat {
        type List = typosaurus::list![Cat];
    }

    #[primitive(property = JungleAnimals)]
    impl Animals for Dog {
        type List = typosaurus::list![Dog];
    }

    #[primitive(property = JungleActions)]
    impl Actions for Hunt {
        type List = typosaurus::list![Hunt];
    }

    #[primitive(property = JungleActions)]
    impl Actions for Sleep {
        type List = typosaurus::list![Sleep];
    }

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct PairGroup {
        left: Cat,
        right: Dog,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct ActionPair {
        hunt: Hunt,
        sleep: Sleep,
    }

    #[test]
    fn animals_list_is_flat_for_derived_groupings() {
        assert_type_eq!(<PairGroup as Animals>::List, typosaurus::list![Cat, Dog]);
    }

    #[test]
    fn actions_list_is_flat_for_derived_groupings() {
        assert_type_eq!(<ActionPair as Actions>::List, typosaurus::list![Hunt, Sleep]);
    }
}
