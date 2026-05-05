#[cfg(test)]
mod tests {
    use inception::Inception;
    use jungle_core::Jungle;
    use jungle_types::{
        Action, ActionCompletion, ActionSet, ActionStep, Creature, CreatureActionSet, CreatureSet,
        CreatureStates, AspectStep, Ecosystem, Ident, JungleActions, JungleCreatures,
        JungleFlow, Whole,
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

            impl jungle_types::Creature for $name {
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
            impl jungle_types::CreatureMember for $name {}

            impl jungle_types::Creature for $name {
                type Id = jungle_types::Id<$id>;
                type State = $state;
                type Instinct = $instinct;
            }

            #[inception::primitive(property = jungle_types::JungleCreatures)]
            impl jungle_types::Creatures for $name {
                type List = typosaurus::collections::sp::Node<$id, $name>;
            }

            #[inception::primitive(property = jungle_types::Ident)]
            impl jungle_types::Identified for $name {
                type Id = $id;
            }
        };
    }

    struct UnitOkStep;
    impl<T, A> AspectStep<T, A> for UnitOkStep
    where
        T: Creature,
        A: Action<In = ()>,
        A: Action<Out = (), Err = ()>,
    {
        type Aspect = Whole;
        type In = ();
        type Out = ();

        fn prepare(_state: &T::State, _input: Self::In) -> A::In {}

        fn apply(_state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
            output.expect("workflow action should succeed");
        }
    }

    macro_rules! prey_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Inception)]
            #[inception(properties = [JungleFlow])]
            struct $name(
                ActionStep<$animal, Eat, UnitOkStep>,
                ActionStep<$animal, Sleep, UnitOkStep>,
                ActionStep<$animal, Forage, UnitOkStep>,
                ActionStep<$animal, Drink, UnitOkStep>,
                ActionStep<$animal, Flee, UnitOkStep>,
            );
        };
    }

    macro_rules! predator_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Inception)]
            #[inception(properties = [JungleFlow])]
            struct $name(
                ActionStep<$animal, Eat, UnitOkStep>,
                ActionStep<$animal, Sleep, UnitOkStep>,
                ActionStep<$animal, Forage, UnitOkStep>,
                ActionStep<$animal, Drink, UnitOkStep>,
                ActionStep<$animal, Hunt, UnitOkStep>,
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
    #[inception(properties = [Ident, JungleCreatures])]
    struct Apes(Gorilla, Chimpanzee);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleCreatures])]
    struct Cats(Tiger, Jaguar);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleCreatures])]
    struct Predators(Cats, Anaconda);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleCreatures])]
    struct AllCreatures(Cats, Apes, Anaconda, Hippo, Elephant);

    #[derive(Inception)]
    #[inception(properties = [Ident, JungleActions])]
    struct AllActions(Predator, Prey);

    struct Zoo;
    impl Ecosystem for Zoo {
        type Creatures = AllCreatures;
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
        assert_type_eq!(CreatureSet<Apes>, ApeList);

        type PredatorList = list![Tiger, Jaguar, Anaconda];
        assert_type_eq!(CreatureSet<Predators>, PredatorList);
    }

    #[test]
    fn animal_action_set() {
        type ApeCreatureActions = list![Eat, Sleep, Forage, Drink, Flee];
        assert_type_eq!(CreatureActionSet<Apes>, ApeCreatureActions);

        type AllCreatureActions = list![Eat, Sleep, Forage, Drink, Hunt, Flee];
        assert_type_eq!(CreatureActionSet<AllCreatures>, AllCreatureActions);
    }

    #[test]
    fn animal_state_set() {
        struct ApeState;
        struct CatState;

        #[derive(Inception)]
        #[inception(properties = [JungleFlow])]
        struct StatefulGorillaInstinct(
            ActionStep<StatefulGorilla, Eat, UnitOkStep>,
            ActionStep<StatefulGorilla, Sleep, UnitOkStep>,
            ActionStep<StatefulGorilla, Forage, UnitOkStep>,
            ActionStep<StatefulGorilla, Drink, UnitOkStep>,
            ActionStep<StatefulGorilla, Flee, UnitOkStep>,
        );

        #[derive(Inception)]
        #[inception(properties = [JungleFlow])]
        struct StatefulTigerInstinct(
            ActionStep<StatefulTiger, Eat, UnitOkStep>,
            ActionStep<StatefulTiger, Sleep, UnitOkStep>,
            ActionStep<StatefulTiger, Forage, UnitOkStep>,
            ActionStep<StatefulTiger, Drink, UnitOkStep>,
            ActionStep<StatefulTiger, Hunt, UnitOkStep>,
        );

        animal!(StatefulGorilla, U0, ApeState, StatefulGorillaInstinct);
        animal!(StatefulTiger, U1, CatState, StatefulTigerInstinct);

        #[derive(Inception)]
        #[inception(properties = [Ident, JungleCreatures])]
        struct StatefulCreatures(StatefulGorilla, StatefulTiger);

        type StatefulCreatureStates = list![ApeState, CatState];
        assert_type_eq!(CreatureStates<StatefulCreatures>, StatefulCreatureStates);
    }

    #[test]
    fn jungle_impl() {
        let zoo = Zoo;
        let jungle_fut = zoo.manifest();
    }

    mod action_step;
    mod aspect;
    mod progression;
}
