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
