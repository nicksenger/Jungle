#[cfg(test)]
mod tests {
    use crate::{Animal, Id};
    use typosaurus::collections::list::{Atom, DeepFlatten};
    use typosaurus::num::consts::{U0, U1};
    use typosaurus::assert_type_eq;

    macro_rules! animal {
        ($name:ident, $id:ty) => {
            struct $name;
            impl Animal for $name {
                type Id = Id<$id>;
                type Instinct = ();
                type Actions = ();
            }
        };
    }

    animal!(AnimalA, U0);
    animal!(AnimalB, U1);

    type NestedAnimals = typosaurus::list![
        typosaurus::list![Atom<AnimalA>, Atom<AnimalB>],
        typosaurus::list![Atom<AnimalA>],
        Atom<AnimalB>
    ];
    type FlatAnimals = DeepFlatten<NestedAnimals>;

    assert_type_eq!(FlatAnimals, typosaurus::list![AnimalA, AnimalB, AnimalA, AnimalB]);
}
