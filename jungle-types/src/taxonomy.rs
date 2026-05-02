use crate::Animal;
use typosaurus::collections::list::{Empty, List};

/// Taxonomic traits for Jungle entities.
///
/// These traits define the grouping hierarchy used to organize
/// species and animals within the ecosystem.

/// Phylum-level grouping
pub trait Phylum {}

/// Class-level grouping
pub trait Class {}

/// Order-level grouping
pub trait Order {}

/// Family-level grouping
pub trait Family {}

/// Grouping for `Species`
pub trait Genus {}

/// Grouping for `Animal`
pub trait Species {}

impl Genus for Empty {}
impl<T, U> Genus for List<(T, U)>
where
    T: Species,
    U: Genus,
{
}

impl Family for Empty {}
impl<T, U> Family for List<(T, U)>
where
    T: Genus,
    U: Family,
{
}

impl Order for Empty {}
impl<T, U> Order for List<(T, U)>
where
    T: Family,
    U: Order,
{
}

impl Class for Empty {}
impl<T, U> Class for List<(T, U)>
where
    T: Order,
    U: Class,
{
}

impl Phylum for Empty {}
impl<T, U> Phylum for List<(T, U)>
where
    T: Class,
    U: Phylum,
{
}

impl<T> Species for T where T: Animal {}

#[cfg(test)]
mod tests {
    use super::*;
    use typosaurus::{
        bool::True,
        cmp::Equality,
        assert_type_eq,
        collections::list::Flatten,
        list,
        num::consts::{U1, U2},
    };

    #[derive(Default)]
    struct Dog;

    impl Animal for Dog {
        type Id = U1;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    #[derive(Default)]
    struct Wolf;

    impl Animal for Wolf {
        type Id = U2;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    impl<T: Animal> Equality<T> for T {
        type Out = True;
    }

    #[test]
    fn list_of_species_implements_genus() {
        fn assert_genus<T: Genus>() {}
        assert_genus::<list![Dog, Wolf]>();
    }

    #[test]
    fn flat() {
        type X = list![list![Dog, Wolf]];
        type Y = Flatten<X>;

        assert_type_eq!(Y, list![Dog, Wolf]);
    }
}
