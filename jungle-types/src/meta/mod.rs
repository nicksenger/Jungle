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
