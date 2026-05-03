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
