#![recursion_limit = "512"]

//! Shared test utilities crate for the Jungle workspace.

#[cfg(test)]
mod tests {
    use inception::{primitive, Inception};
    use jungle_core::Jungle;
    use jungle_types::{
        Action, Actions, Animal, Animals, Ecosystem, Id, Instinct, JungleAction, JungleAnimal,
    };
    use typosaurus::assert_type_eq;
    use typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};

    macro_rules! define_action {
        ($name:ident, $id:ty) => {
            struct $name;

            impl Action for $name {
                type Id = Id<$id>;
                type State = ();
                type In = ();
                type Out = ();
                type Err = ();

                fn act(
                    _state: &Self::State,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    std::future::ready(Ok(()))
                }
            }

            #[primitive(property = JungleAction)]
            impl Actions for $name {
                type List =
                    typosaurus::collections::list::List<($name, typosaurus::collections::list::Empty)>;
            }
        };
    }

    define_action!(Eat, U0);
    define_action!(Sleep, U1);
    define_action!(Forage, U2);
    define_action!(Drink, U3);
    define_action!(Hunt, U4);

    #[derive(Inception)]
    #[inception(properties = [JungleAction])]
    struct BasicNeeds {
        eat: Eat,
        sleep: Sleep,
        forage: Forage,
        drink: Drink,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleAction])]
    struct Predation {
        hunt: Hunt,
    }

    struct NoActions;
    impl Actions for NoActions {
        type List = typosaurus::collections::list::Empty;
    }

    macro_rules! define_animal {
        ($name:ident, $id:ty, $instinct:ty) => {
            struct $name;

            impl Animal for $name {
                type Id = Id<$id>;
                type Instinct = $instinct;
            }

            #[primitive(property = JungleAnimal)]
            impl Animals for $name {
                type List =
                    typosaurus::collections::list::List<($name, typosaurus::collections::list::Empty)>;
            }
        };
    }

    struct ApeInstinct;
    impl Instinct for ApeInstinct {
        type Actions = NoActions;
    }

    struct CatInstinct;
    impl Instinct for CatInstinct {
        type Actions = NoActions;
    }

    struct AnacondaInstinct;
    impl Instinct for AnacondaInstinct {
        type Actions = NoActions;
    }

    struct GrazerInstinct;
    impl Instinct for GrazerInstinct {
        type Actions = NoActions;
    }

    define_animal!(Gorilla, U0, ApeInstinct);
    define_animal!(Chimpanzee, U1, ApeInstinct);
    define_animal!(Tiger, U2, CatInstinct);
    define_animal!(Jaguar, U3, CatInstinct);
    define_animal!(Anaconda, U4, AnacondaInstinct);
    define_animal!(Hippo, U5, GrazerInstinct);
    define_animal!(Elephant, U6, GrazerInstinct);

    #[derive(Inception)]
    #[inception(properties = [JungleAnimal])]
    struct Apes {
        gorilla: Gorilla,
        chimpanzee: Chimpanzee,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleAnimal])]
    struct Cats {
        tiger: Tiger,
        jaguar: Jaguar,
    }

    struct ZooAnimals;
    #[primitive(property = JungleAnimal)]
    impl Animals for ZooAnimals {
        type List = typosaurus::list![Gorilla, Chimpanzee, Tiger, Jaguar, Anaconda, Hippo, Elephant];
    }

    struct ZooActions;
    #[primitive(property = JungleAction)]
    impl Actions for ZooActions {
        type List = typosaurus::list![Eat, Sleep, Forage, Drink, Hunt];
    }

    struct Zoo;
    impl Ecosystem for Zoo {
        type Actions = ZooActions;
        type Animals = ZooAnimals;
    }

    #[test]
    fn zoo_jungle_animals_contains_every_configured_animal() {
        fn assert_jungle<T: Jungle>() {}
        assert_jungle::<Zoo>();

        type ZooAnimalList = <ZooAnimals as Animals>::List;
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U0>, Gorilla);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U1>, Chimpanzee);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U2>, Tiger);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U3>, Jaguar);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U4>, Anaconda);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U5>, Hippo);
        assert_type_eq!(typosaurus::collections::list::Idx<ZooAnimalList, U6>, Elephant);
    }
}
