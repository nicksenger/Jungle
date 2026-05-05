#[cfg(test)]
mod tests {
    use inception::{primitive, Inception};
    use jungle_core::Jungle;
    use jungle_types::{
        Action, ActionCompletion, ActionInputMapper, ActionMember, ActionOutputMapper, ActionSet,
        ActionStep, Actions, Animal, AnimalActionSet, AnimalMember, AnimalSet, AnimalStates,
        Animals, Awaiting, Ecosystem, Id, Ident, Identified, Instinct, JungleActions,
        JungleAnimals, Yielding,
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
                type Dependency = ();
                type In = ();
                type Out = ();
                type Err = ();

                fn act(
                    _dependency: &Self::Dependency,
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

    struct SharedState;
    impl<T> From<&T> for SharedState {
        fn from(_value: &T) -> Self {
            Self
        }
    }

    macro_rules! animal {
        ($name:ident, $id:ty, $instinct:ty) => {
            animal!($name, $id, SharedState, $instinct);
        };

        ($name:ident, $id:ty, $state:ty, $instinct:ty) => {
            struct $name;
            impl AnimalMember for $name {}

            impl Animal for $name {
                type Id = Id<$id>;
                type State = $state;
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

    #[test]
    fn animal_action_set() {
        type ApeAnimalActions = list![Eat, Sleep, Forage, Drink, Flee];
        assert_type_eq!(AnimalActionSet<Apes>, ApeAnimalActions);

        type AllAnimalActions = list![Eat, Sleep, Forage, Drink, Hunt, Flee];
        assert_type_eq!(AnimalActionSet<AllAnimals>, AllAnimalActions);
    }

    #[test]
    fn animal_state_set() {
        struct ApeState;
        struct CatState;

        animal!(StatefulGorilla, U0, ApeState, ApeInstinct);
        animal!(StatefulTiger, U1, CatState, CatInstinct);

        #[derive(Inception)]
        #[inception(properties = [Ident, JungleAnimals])]
        struct StatefulAnimals(StatefulGorilla, StatefulTiger);

        type StatefulAnimalStates = list![ApeState, CatState];
        assert_type_eq!(AnimalStates<StatefulAnimals>, StatefulAnimalStates);
    }

    #[test]
    fn jungle_impl() {
        let zoo = Zoo;
        let jungle_fut = zoo.manifest();
    }

    struct GatherAction;
    impl Action for GatherAction {
        type Id = Id<U0>;
        type Dependency = ();
        type In = i32;
        type Out = i32;
        type Err = ();

        fn act(
            _dependency: &Self::Dependency,
            input: Self::In,
        ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
            std::future::ready(Ok(input + 1))
        }
    }

    struct GatherAnimal;
    impl Animal for GatherAnimal {
        type Id = Id<U0>;
        type State = ();
        type Instinct = ApeInstinct;
    }

    struct PrepareGather;
    impl ActionInputMapper<GatherAnimal, GatherAction> for PrepareGather {
        type In = i32;

        fn map_input(&self, _state: &(), input: Self::In) -> i32 {
            input + 4
        }
    }

    struct ApplyGather;
    impl ActionOutputMapper<GatherAnimal, GatherAction> for ApplyGather {
        type Out = i32;

        fn map_output(
            &self,
            _state: &mut (),
            output: ActionCompletion<GatherAction>,
        ) -> Self::Out {
            output.expect("gather action should succeed")
        }
    }

    #[test]
    fn action_step_adapts_action_to_temporal_protocol() {
        let step = ActionStep::<GatherAnimal, GatherAction, PrepareGather, ApplyGather>::new(
            PrepareGather,
            ApplyGather,
        );
        let (dependency, request) = step.run(((), 3));
        assert_eq!(request.into_input(), 7);

        let apply_step =
            ActionStep::<GatherAnimal, GatherAction, PrepareGather, ApplyGather>::new(
            PrepareGather,
            ApplyGather,
        );
        let (next_dependency, emitted) = apply_step.accept((dependency, Ok(9)));
        assert_eq!(emitted, 9);
        assert_eq!(next_dependency, ());
    }
}
