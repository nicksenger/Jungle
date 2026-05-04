//! Shared test utilities crate for the Jungle workspace.
#![recursion_limit = "2048"]

#[cfg(test)]
mod tests {
    use inception::{primitive, Inception};
    use jungle_types::{
        Action, Actions, Animal, Animals, Ecosystem, Id, Instinct, JungleActions, JungleAnimals,
    };
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

            #[primitive(property = JungleActions)]
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
    #[inception(properties = [JungleActions])]
    struct BasicNeeds {
        eat: Eat,
        sleep: Sleep,
        forage: Forage,
        drink: Drink,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct Predation {
        hunt: Hunt,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct Predator {
        basic_needs: BasicNeeds,
        predation: Predation,
    }

    struct ApeActions;
    impl Actions for ApeActions {
        type List = typosaurus::list![Eat, Sleep, Forage, Drink];
    }

    struct PredatorActions;
    impl Actions for PredatorActions {
        type List = typosaurus::list![Hunt];
    }

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct SinglePredator {
        hunt: Hunt,
    }

    macro_rules! define_animal {
        ($name:ident, $id:ty, $instinct:ty) => {
            struct $name;

            impl Animal for $name {
                type Id = Id<$id>;
                type Instinct = $instinct;
            }

            #[primitive(property = JungleAnimals)]
            impl Animals for $name {
                type List =
                    typosaurus::collections::list::List<($name, typosaurus::collections::list::Empty)>;
            }
        };
    }

    struct ApeInstinct;
    impl Instinct for ApeInstinct {
        type Actions = ApeActions;
    }

    struct CatInstinct;
    impl Instinct for CatInstinct {
        type Actions = PredatorActions;
    }

    struct AnacondaInstinct;
    impl Instinct for AnacondaInstinct {
        type Actions = SinglePredator;
    }

    struct GrazerInstinct;
    impl Instinct for GrazerInstinct {
        type Actions = ApeActions;
    }

    define_animal!(Gorilla, U0, ApeInstinct);
    define_animal!(Chimpanzee, U1, ApeInstinct);
    define_animal!(Tiger, U2, CatInstinct);
    define_animal!(Jaguar, U3, CatInstinct);
    define_animal!(Anaconda, U4, AnacondaInstinct);
    define_animal!(Hippo, U5, GrazerInstinct);
    define_animal!(Elephant, U6, GrazerInstinct);

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct Apes {
        gorilla: Gorilla,
        chimpanzee: Chimpanzee,
    }

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct Cats {
        tiger: Tiger,
        jaguar: Jaguar,
    }

    struct ZooAnimals;
    #[primitive(property = JungleAnimals)]
    impl Animals for ZooAnimals {
        type List = typosaurus::list![Gorilla, Chimpanzee, Tiger, Jaguar, Anaconda, Hippo, Elephant];
    }

    struct ZooActions;
    #[primitive(property = JungleActions)]
    impl Actions for ZooActions {
        type List = typosaurus::list![Eat, Sleep, Forage, Drink, Hunt];
    }

    struct Zoo;
    impl Ecosystem for Zoo {
        type Actions = ZooActions;
        type Animals = ZooAnimals;
    }

    #[test]
    fn instinct_actions_accepts_derived_inception_type() {
        fn assert_instinct<T: Instinct<Actions = SinglePredator>>() {}
        assert_instinct::<AnacondaInstinct>();
    }

    #[test]
    fn derived_actions_type_implements_actions_trait() {
        fn assert_actions<T: Actions>() {}
        assert_actions::<SinglePredator>();
    }
}
