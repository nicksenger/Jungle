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
        num::consts::{U1, U2, U3, U4, U5},
    };

    /// Blanket self-equality for all animals.
    ///
    /// Rust's orphan rules prevent a direct `impl<T: Animal> Equality<T> for T`
    /// in this crate (since `Equality` is defined externally in `typosaurus`).
    /// Instead, we define a local helper trait that all `Animal`s implement,
    /// then implement `Equality` for that local trait.
    pub trait SelfEq {
        type Out;
    }
    impl<T: Animal> SelfEq for T {
        type Out = True;
    }

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

    // Generate Equality impls for every animal via the local SelfEq trait.
    macro_rules! impl_equality_self {
        ($($t:ty),* $(,)?) => {
            $(
                impl Equality<$t> for $t {
                    type Out = <$t as SelfEq>::Out;
                }
            )*
        };
    }
    impl_equality_self!(Dog, Wolf);

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

    #[derive(Default)]
    struct Cat;

    impl Animal for Cat {
        type Id = U2;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    #[derive(Default)]
    struct Horse;

    impl Animal for Horse {
        type Id = U3;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    #[derive(Default)]
    struct Eagle;

    impl Animal for Eagle {
        type Id = U4;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    #[derive(Default)]
    struct Shark;

    impl Animal for Shark {
        type Id = U5;
        type Form = ();
        type Motivation = ();
        type Instinct = ();
        type Niches = ();
        type Symbionts = ();
    }

    impl_equality_self!(Cat, Horse, Eagle, Shark);

    /// A genus with multiple species: Canis (dog, wolf)
    type Canis = list![Dog, Wolf];

    /// A genus with a single species: Felis (cat)
    type Felis = list![Cat];

    /// A family with multiple genera: Canidae
    type Canidae = list![Canis, Felis];

    /// An order with multiple families
    type Carnivora = list![Canidae];

    /// A class with multiple orders
    type Mammalia = list![Carnivora];

    /// A phylum with multiple classes
    type Chordata = list![Mammalia];

    #[test]
    fn genus_trait() {
        fn assert_genus<T: Genus>() {}
        assert_genus::<list![Dog, Wolf]>();
        assert_genus::<list![Cat]>();
    }

    #[test]
    fn family_trait() {
        fn assert_family<T: Family>() {}
        assert_family::<Canidae>();
    }

    #[test]
    fn order_trait() {
        fn assert_order<T: Order>() {}
        assert_order::<Carnivora>();
    }

    #[test]
    fn class_trait() {
        fn assert_class<T: Class>() {}
        assert_class::<Mammalia>();
    }

    #[test]
    fn phylum_trait() {
        fn assert_phylum<T: Phylum>() {}
        assert_phylum::<Chordata>();
    }

    #[test]
    fn deep_hierarchy_flat() {
        type Nested = list![list![list![list![Dog, Wolf], list![Cat]], list![Horse]]];
        type Flattened = Flatten<Nested>;
        assert_type_eq!(Flattened, list![Dog, Wolf, Cat, Horse]);
    }
}
