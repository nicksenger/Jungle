#[cfg(test)]
mod tests {
    use inception::{primitive, Inception};
    use jungle_types::{
        Action, ActionMember, ActionSet, Actions, Animal, AnimalMember, AnimalSet, Animals,
        Ecosystem, Id, Ident, Identified, Instinct, JungleActions, JungleAnimals,
    };
    use typosaurus::assert_type_eq;
    use typosaurus::collections::list;
    use typosaurus::collections::sp::{Node, SPFlatten};
    use typosaurus::list;
    use typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};

    macro_rules! action {
        ($name:ident, $id:ty) => {
            struct $name;
            impl ActionMember for $name {}

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
                type List = Node<$id, $name>;
            }

            #[primitive(property = Ident)]
            impl Identified for $name {
                type Id = $id;
            }
        };
    }

    action!(Eat, U0);
    action!(Sleep, U1);
    action!(Forage, U2);
    action!(Drink, U3);
    action!(Hunt, U4);
    action!(Flee, U5);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct BasicNeeds(Eat, Sleep, Forage, Drink);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct Predation(Hunt);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct Predator(BasicNeeds, Predation);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct Prey(BasicNeeds, Flee);

    macro_rules! animal {
        ($name:ident, $id:ty, $instinct:ty) => {
            struct $name;
            impl AnimalMember for $name {}

            impl Animal for $name {
                type Id = Id<$id>;
                type State = ();
                type Instinct = $instinct;
            }

            #[primitive(property = JungleAnimals)]
            impl Animals for $name {
                type List = Node<$id, $name>;
            }

            #[primitive(property = Ident)]
            impl Identified for $name {
                type Id = $id;
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

    animal!(Gorilla, U0, ApeInstinct);
    animal!(Chimpanzee, U1, ApeInstinct);
    animal!(Tiger, U2, CatInstinct);
    animal!(Jaguar, U3, CatInstinct);
    animal!(Anaconda, U4, AnacondaInstinct);
    animal!(Hippo, U5, GrazerInstinct);
    animal!(Elephant, U6, GrazerInstinct);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleAnimals])]
    struct Apes(Gorilla, Chimpanzee);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleAnimals])]
    struct Cats(Tiger, Jaguar);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleAnimals])]
    struct Predators(Cats, Anaconda);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleAnimals])]
    struct AllAnimals(Cats, Apes, Anaconda, Hippo, Elephant);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct AllActions(Predator, Prey);

    struct Zoo;
    impl Ecosystem for Zoo {
        type Actions = AllActions;
        type Animals = AllAnimals;
    }

    #[test]
    fn composite_actions() {
        type BasicList = list![Eat, Sleep, Forage, Drink];
        assert_type_eq!(ActionSet<BasicNeeds>, BasicList);

        type PredatorList = list![Eat, Sleep, Forage, Drink, Hunt];
        assert_type_eq!(ActionSet<Predator>, PredatorList);
    }

    #[test]
    fn composite_animals() {
        type ApeList = list![Gorilla, Chimpanzee];
        assert_type_eq!(AnimalSet<Apes>, ApeList);

        type PredatorList = list![Tiger, Jaguar, Anaconda];
        assert_type_eq!(AnimalSet<Predators>, PredatorList);
    }
}
