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
