use crate::Animal;

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
