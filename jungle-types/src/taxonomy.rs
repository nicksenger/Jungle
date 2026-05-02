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
impl<T, U> Genus for List<(T, U)> where T: Species, U: Genus {}

impl<T> Species for T
where
    T: Animal,
{
}

#[cfg(test)]
mod tests {
    use typosaurus::list;
    use super::*;

    #[derive(Default)]
    struct Dog;

    impl Animal for Dog {
        type Form = ();
        type Motivation = ();
        type Instinct = ();
    }

    #[derive(Default)]
    struct Wolf;

    impl Animal for Wolf {
        type Form = ();
        type Motivation = ();
        type Instinct = ();
    }

    #[test]
    fn list_of_species_implements_genus() {
        fn assert_genus<T: Genus>() {}
        assert_genus::<list![Dog, Wolf]>();
    }
}
