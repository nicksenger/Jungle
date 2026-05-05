#[cfg(test)]
mod tests {
    use inception::Inception;
    use jungle_core::Jungle;
    use jungle_types::{
        Action, ActionCompletion, ActionInputMapper, ActionOutputMapper, ActionSet, ActionStep,
        Animal, AnimalActionSet, AnimalSet, AnimalStates, Ecosystem, Ident, JungleAnimals,
        JungleActions, JungleWorkflowActions,
    };
    use typosaurus::assert_type_eq;
    use typosaurus::list;
    use typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};

    macro_rules! action {
        (
            $name:ident,
            $id:ty,
            in = $in:ty,
            out = $out:ty,
            err = $err:ty,
            act = |$dependency:ident, $input:ident| $body:block
        ) => {
            struct $name;
            impl jungle_types::ActionMember for $name {}

            impl jungle_types::Action for $name {
                type Id = jungle_types::Id<$id>;
                type Dependency = ();
                type In = $in;
                type Out = $out;
                type Err = $err;

                fn act(
                    $dependency: &Self::Dependency,
                    $input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    $body
                }
            }
        };

        ($name:ident, $id:ty) => {
            struct $name;
            impl jungle_types::ActionMember for $name {}

            impl jungle_types::Action for $name {
                type Id = jungle_types::Id<$id>;
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

            #[inception::primitive(property = jungle_types::JungleActions)]
            impl jungle_types::Actions for $name {
                type List = typosaurus::collections::sp::Node<$id, $name>;
            }

            #[inception::primitive(property = jungle_types::Ident)]
            impl jungle_types::Identified for $name {
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
        ($name:ident, $id:ty, state = $state:ty, instinct = $instinct:ty) => {
            struct $name;

            impl jungle_types::Animal for $name {
                type Id = jungle_types::Id<$id>;
                type State = $state;
                type Instinct = $instinct;
            }
        };

        ($name:ident, $id:ty, instinct = $instinct:ty) => {
            animal!($name, $id, state = (), instinct = $instinct);
        };

        ($name:ident, $id:ty, $instinct:ty) => {
            animal!($name, $id, SharedState, $instinct);
        };

        ($name:ident, $id:ty, $state:ty, $instinct:ty) => {
            struct $name;
            impl jungle_types::AnimalMember for $name {}

            impl jungle_types::Animal for $name {
                type Id = jungle_types::Id<$id>;
                type State = $state;
                type Instinct = $instinct;
            }

            #[inception::primitive(property = jungle_types::JungleAnimals)]
            impl jungle_types::Animals for $name {
                type List = typosaurus::collections::sp::Node<$id, $name>;
            }

            #[inception::primitive(property = jungle_types::Ident)]
            impl jungle_types::Identified for $name {
                type Id = $id;
            }
        };
    }

    struct UnitInput;
    impl<T, A> ActionInputMapper<T, A> for UnitInput
    where
        T: Animal,
        A: Action<In = ()>,
    {
        type In = ();

        fn map_input(_state: &T::State, _input: Self::In) {}
    }

    struct ExpectOk;
    impl<T, A> ActionOutputMapper<T, A> for ExpectOk
    where
        T: Animal,
        A: Action<Out = (), Err = ()>,
    {
        type Out = ();

        fn map_output(_state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
            output.expect("workflow action should succeed");
        }
    }

    macro_rules! prey_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Inception)]
            #[inception(properties = [JungleWorkflowActions])]
            struct $name(
                ActionStep<$animal, Eat, UnitInput, ExpectOk>,
                ActionStep<$animal, Sleep, UnitInput, ExpectOk>,
                ActionStep<$animal, Forage, UnitInput, ExpectOk>,
                ActionStep<$animal, Drink, UnitInput, ExpectOk>,
                ActionStep<$animal, Flee, UnitInput, ExpectOk>,
            );
        };
    }

    macro_rules! predator_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Inception)]
            #[inception(properties = [JungleWorkflowActions])]
            struct $name(
                ActionStep<$animal, Eat, UnitInput, ExpectOk>,
                ActionStep<$animal, Sleep, UnitInput, ExpectOk>,
                ActionStep<$animal, Forage, UnitInput, ExpectOk>,
                ActionStep<$animal, Drink, UnitInput, ExpectOk>,
                ActionStep<$animal, Hunt, UnitInput, ExpectOk>,
            );
        };
    }

    animal!(Gorilla, U0, GorillaInstinct);
    animal!(Chimpanzee, U1, ChimpanzeeInstinct);
    animal!(Tiger, U2, TigerInstinct);
    animal!(Jaguar, U3, JaguarInstinct);
    animal!(Anaconda, U4, AnacondaInstinct);
    animal!(Hippo, U5, HippoInstinct);
    animal!(Elephant, U6, ElephantInstinct);

    prey_instinct!(GorillaInstinct, Gorilla);
    prey_instinct!(ChimpanzeeInstinct, Chimpanzee);
    predator_instinct!(TigerInstinct, Tiger);
    predator_instinct!(JaguarInstinct, Jaguar);
    predator_instinct!(AnacondaInstinct, Anaconda);
    prey_instinct!(HippoInstinct, Hippo);
    prey_instinct!(ElephantInstinct, Elephant);

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

        #[derive(Inception)]
        #[inception(properties = [JungleWorkflowActions])]
        struct StatefulGorillaInstinct(
            ActionStep<StatefulGorilla, Eat, UnitInput, ExpectOk>,
            ActionStep<StatefulGorilla, Sleep, UnitInput, ExpectOk>,
            ActionStep<StatefulGorilla, Forage, UnitInput, ExpectOk>,
            ActionStep<StatefulGorilla, Drink, UnitInput, ExpectOk>,
            ActionStep<StatefulGorilla, Flee, UnitInput, ExpectOk>,
        );

        #[derive(Inception)]
        #[inception(properties = [JungleWorkflowActions])]
        struct StatefulTigerInstinct(
            ActionStep<StatefulTiger, Eat, UnitInput, ExpectOk>,
            ActionStep<StatefulTiger, Sleep, UnitInput, ExpectOk>,
            ActionStep<StatefulTiger, Forage, UnitInput, ExpectOk>,
            ActionStep<StatefulTiger, Drink, UnitInput, ExpectOk>,
            ActionStep<StatefulTiger, Hunt, UnitInput, ExpectOk>,
        );

        animal!(StatefulGorilla, U0, ApeState, StatefulGorillaInstinct);
        animal!(StatefulTiger, U1, CatState, StatefulTigerInstinct);

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

    mod action_step;
    mod progression;
}
