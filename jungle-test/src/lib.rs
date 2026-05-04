#[cfg(test)]
mod tests {
    use inception::{primitive, Inception};
    use jungle_types::{
        Action, Actions, Animal, Animals, Ecosystem, Id, Instinct, JungleActions, JungleAnimals,
    };
    use typosaurus::assert_type_eq;
    use typosaurus::list;
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
                type List = typosaurus::collections::list::List<(
                    $name,
                    typosaurus::collections::list::Empty,
                )>;
            }
        };
    }

    define_action!(Eat, U0);
    define_action!(Sleep, U1);
    define_action!(Forage, U2);
    define_action!(Drink, U3);
    define_action!(Hunt, U4);
    define_action!(Flee, U5);

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct BasicNeeds(Eat, Sleep, Forage, Drink);

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct Predation(Hunt);

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct Predator(BasicNeeds, Predation);

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct Prey(BasicNeeds, Flee);

    macro_rules! define_animal {
        ($name:ident, $id:ty, $instinct:ty) => {
            struct $name;

            impl Animal for $name {
                type Id = Id<$id>;
                type Instinct = $instinct;
            }

            #[primitive(property = JungleAnimals)]
            impl Animals for $name {
                type List = typosaurus::collections::list::List<(
                    $name,
                    typosaurus::collections::list::Empty,
                )>;
            }
        };
    }

    struct ApeInstinct;
    impl Instinct for ApeInstinct {
        type Actions = Prey;
    }

    struct CatInstinct;
    impl Instinct for CatInstinct {
        type Actions = Predator;
    }

    struct AnacondaInstinct;
    impl Instinct for AnacondaInstinct {
        type Actions = Predator;
    }

    struct GrazerInstinct;
    impl Instinct for GrazerInstinct {
        type Actions = Prey;
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
    struct Apes(Gorilla, Chimpanzee);

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct Cats(Tiger, Jaguar);

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct Predators(Cats, Anaconda);

    #[derive(Inception)]
    #[inception(properties = [JungleAnimals])]
    struct AllAnimals(Cats, Apes, Anaconda, Hippo, Elephant);

    #[derive(Inception)]
    #[inception(properties = [JungleActions])]
    struct AllActions(Predator, Prey);

    struct Zoo;
    impl Ecosystem for Zoo {
        type Actions = AllActions;
        type Animals = AllAnimals;
    }

    #[test]
    fn composite_actions() {
        type BasicList = list![Eat, Sleep, Forage, Drink];
        assert_type_eq!(<BasicNeeds as Actions>::List, BasicList);

        type PredatorList = typosaurus::list![Eat, Sleep, Forage, Drink, Hunt];
        //assert_type_eq!(<Predator as Actions>::List, PredatorList);
    }

    #[test]
    fn composite_animals() {
        type ApeList = list![Gorilla, Chimpanzee];
        assert_type_eq!(<Apes as Animals>::List, ApeList);

        type PredatorList = list![Jaguar, Tiger, Anaconda];
        //assert_type_eq!(<Predators as Animals>::List, PredatorList);
    }
}
