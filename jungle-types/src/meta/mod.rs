// Newtype wrapper proving that two Animal types are equal when their Ids are equal.
// Works around Rust's orphan rules by keeping the newtype local to this crate.
use core::marker::PhantomData;
use typosaurus::cmp::Equality;

// AnimalEquality newtype used to satisfy orphan rules for Equality impls.
pub struct AnimalEquality<T, U>(PhantomData<(T, U)>)
where
    T: crate::Animal,
    U: crate::Animal;

// Blanket Equality impl: two Animal newtypes are equal iff their Id types are equal.
impl<T, U> Equality<AnimalEquality<T, U>> for AnimalEquality<T, U>
where
    T: crate::Animal,
    U: crate::Animal,
    T::Id: Equality<U::Id>,
{
    type Out = <T::Id as Equality<U::Id>>::Out;
}

#[cfg(test)]
mod tests {
    use super::AnimalEquality;
    use crate::{Animal, Id};
    use typosaurus::bool::{False, True};
    use typosaurus::cmp::IsEqual;
    use typosaurus::num::consts::{U0, U1};
    use typosaurus::assert_type_eq;

    struct AnimalA;
    impl Animal for AnimalA {
        type Id = Id<U0>;
        type Instinct = ();
        type Actions = ();
    }

    struct AnimalB;
    impl Animal for AnimalB {
        type Id = Id<U1>;
        type Instinct = ();
        type Actions = ();
    }

    type SelfEqA = <(AnimalEquality<AnimalA, AnimalA>, AnimalEquality<AnimalA, AnimalA>) as IsEqual>::Out;
    type SelfEqB = <(AnimalEquality<AnimalB, AnimalB>, AnimalEquality<AnimalB, AnimalB>) as IsEqual>::Out;
    type NotEqAB = <(AnimalEquality<AnimalA, AnimalB>, AnimalEquality<AnimalA, AnimalB>) as IsEqual>::Out;

    assert_type_eq!(SelfEqA, True);
    assert_type_eq!(SelfEqB, True);
    assert_type_eq!(NotEqAB, False);
}
