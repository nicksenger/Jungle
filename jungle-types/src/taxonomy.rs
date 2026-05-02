use crate::Animal;

/// Taxonomic traits for Jungle entities.
///
/// These traits define the grouping hierarchy used to organize
/// species and animals within the ecosystem.

/// Phylum-level grouping
pub trait Phylum {
    type Taxa;
}

/// Class-level grouping
pub trait Class {
    type Taxa;
}

/// Order-level grouping
pub trait Order {
    type Taxa;
}

/// Family-level grouping
pub trait Family {
    type Taxa;
}

/// Grouping for `Species`
pub trait Genus {
    type Taxa;
}

/// Grouping for `Animal`
pub trait Species {}

impl<T> Species for T
where
    T: Animal,
{
}

/// Marker for species-level animals that are atomic leaf types.
/// Types implementing `Leaf` flatten to a singleton list.
pub trait Leaf {}

/// `Empty` (empty list) flattens to `Empty`.
impl Flattened for typosaurus::collections::list::Empty {
    type Out = typosaurus::collections::list::Empty;
}

/// Leaf (species-level) types flatten to a singleton list.
impl<T: Leaf> Flattened for T {
    type Out = typosaurus::collections::list::List<(T, typosaurus::collections::list::Empty)>;
}

/// Genus-level types flatten by recursing into their `Taxa`.
impl<G: Genus> Flattened for G {
    type Out = <G::Taxa as Flattened>::Out;
}

/// A list of flattened elements is the append of each element's flatten.
impl<H, T> Flattened for typosaurus::collections::list::List<(H, T)>
where
    H: Flattened,
    T: Flattened,
    (<H as Flattened>::Out, <T as Flattened>::Out): typosaurus::traits::monoid::Mappend,
{
    type Out = <(<H as Flattened>::Out, <T as Flattened>::Out) as typosaurus::traits::monoid::Mappend>::Out;
}

/// Convenience type alias to flatten any taxonomy entity into a flat `list!` of animals.
pub type Flatten<T> = <T as typosaurus::traits::fold::Foldable<Concatenation>>::Out;

/// Semigroup for concatenating flattened taxonomy types.
pub struct Concatenation;

impl<Lhs, Rhs> typosaurus::traits::monoid::Mappend for (Lhs, Rhs)
where
    Lhs: Flattened,
    Rhs: Flattened,
    (<Lhs as Flattened>::Out, <Rhs as Flattened>::Out): typosaurus::traits::monoid::Mappend,
{
    type Mappend = <(<Lhs as Flattened>::Out, <Rhs as Flattened>::Out) as typosaurus::traits::monoid::Mappend>::Out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use typosaurus::collections::list::Flattened as TyFlattened;
    use typosaurus::assert_type_eq;

    // --- Example from the PR: dog, cat, wolf ---

    struct Dog;
    impl Animal for Dog {
        type Form = ();
        type Motivation = ();
        type Instinct = ();
    }
    impl Leaf for Dog {}

    struct Wolf;
    impl Animal for Wolf {
        type Form = ();
        type Motivation = ();
        type Instinct = ();
    }
    impl Leaf for Wolf {}

    struct Cat;
    impl Animal for Cat {
        type Form = ();
        type Motivation = ();
        type Instinct = ();
    }
    impl Leaf for Cat {}

    struct Canis;
    impl Genus for Canis {
        type Taxa = list![Dog, Wolf];
    }

    #[test]
    fn test_canis_flattens_to_dog_and_wolf() {
        type Flat = <Canis as TyFlattened>::Out;
        assert_type_eq!(Flat, list![Dog, Wolf]);
    }

    #[test]
    fn test_list_canis_cat_flattens() {
        // list![Canis, Cat] → list![Dog, Wolf, Cat]
        type Input = list![Canis, Cat];
        type Result = <Input as TyFlattened>::Out;
        assert_type_eq!(Result, list![Dog, Wolf, Cat]);
    }

    #[test]
    fn test_flatten_alias() {
        type Input = list![Canis, Cat];
        type Result = Flatten<Input>;
        assert_type_eq!(Result, list![Dog, Wolf, Cat]);
    }
}
