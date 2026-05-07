#[cfg(test)]
extern crate jungle_sdk as inception;
#[cfg(test)]
extern crate jungle_sdk as jungle_types;

#[cfg(test)]
mod tests {

    use jungle_sdk::core::Jungle as _;
    use jungle_sdk::types::{
        Action, ActionCompletion, ActionSet, ActionTask, Creature, CreatureActionSet, CreatureSet,
        CreatureStates, Ecosystem, Identity, Task,
    };
    use jungle_sdk::typosaurus::assert_type_eq;
    use jungle_sdk::typosaurus::list;
    use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};
    use jungle_sdk::{Actions, Creatures, Flow};

    macro_rules! action {
        (
            $name:ident,
            $id:ty,
            in = $in:ty,
            out = $out:ty,
            err = $err:ty,
            act = |$dependency:ident, $input:ident| $body:expr
        ) => {
            struct $name;
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
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
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
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

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleActions)]
            impl jungle_sdk::types::Actions for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
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

    #[derive(Actions)]
    struct BasicNeeds(Eat, Sleep, Forage, Drink);

    #[derive(Actions)]
    struct Predation(Hunt);

    #[derive(Actions)]
    struct Predator(BasicNeeds, Predation);

    #[derive(Actions)]
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

            impl jungle_sdk::types::Creature for $name {
                type Id = jungle_sdk::types::Id<$id>;
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
            impl jungle_sdk::types::CreatureMember for $name {}

            impl jungle_sdk::types::Creature for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Instinct = $instinct;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleCreatures)]
            impl jungle_sdk::types::Creatures for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    struct UnitOkStep;
    impl<T, A> Task<T, A> for UnitOkStep
    where
        T: Creature,
        A: Action<In = ()>,
        A: Action<Out = (), Err = ()>,
    {
        type Aspect = Identity;
        type In = ();
        type Out = ();

        fn prepare(_state: &T::State, _input: Self::In) -> A::In {}

        fn process(_state: &mut T::State, output: ActionCompletion<A>) -> Self::Out {
            output.expect("workflow action should succeed");
        }
    }

    macro_rules! prey_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Flow)]
            struct $name(
                ActionTask<$animal, Eat, UnitOkStep>,
                ActionTask<$animal, Sleep, UnitOkStep>,
                ActionTask<$animal, Forage, UnitOkStep>,
                ActionTask<$animal, Drink, UnitOkStep>,
                ActionTask<$animal, Flee, UnitOkStep>,
            );
        };
    }

    macro_rules! predator_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Flow)]
            struct $name(
                ActionTask<$animal, Eat, UnitOkStep>,
                ActionTask<$animal, Sleep, UnitOkStep>,
                ActionTask<$animal, Forage, UnitOkStep>,
                ActionTask<$animal, Drink, UnitOkStep>,
                ActionTask<$animal, Hunt, UnitOkStep>,
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

    #[derive(Creatures)]
    struct Apes(Gorilla, Chimpanzee);

    #[derive(Creatures)]
    struct Cats(Tiger, Jaguar);

    #[derive(Creatures)]
    struct Predators(Cats, Anaconda);

    #[derive(Creatures)]
    struct AllCreatures(Cats, Apes, Anaconda, Hippo, Elephant);

    #[derive(Actions)]
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

        #[derive(Flow)]
        struct StatefulGorillaInstinct(
            ActionTask<StatefulGorilla, Eat, UnitOkStep>,
            ActionTask<StatefulGorilla, Sleep, UnitOkStep>,
            ActionTask<StatefulGorilla, Forage, UnitOkStep>,
            ActionTask<StatefulGorilla, Drink, UnitOkStep>,
            ActionTask<StatefulGorilla, Flee, UnitOkStep>,
        );

        #[derive(Flow)]
        struct StatefulTigerInstinct(
            ActionTask<StatefulTiger, Eat, UnitOkStep>,
            ActionTask<StatefulTiger, Sleep, UnitOkStep>,
            ActionTask<StatefulTiger, Forage, UnitOkStep>,
            ActionTask<StatefulTiger, Drink, UnitOkStep>,
            ActionTask<StatefulTiger, Hunt, UnitOkStep>,
        );

        animal!(StatefulGorilla, U0, ApeState, StatefulGorillaInstinct);
        animal!(StatefulTiger, U1, CatState, StatefulTigerInstinct);

        #[derive(Creatures)]
        struct StatefulCreatures(StatefulGorilla, StatefulTiger);

        type StatefulCreatureStates = list![ApeState, CatState];
        assert_type_eq!(CreatureStates<StatefulCreatures>, StatefulCreatureStates);
    }

    #[test]
    fn jungle_impl() {
        struct ManifestAction;
        impl jungle_sdk::types::ActionMember for ManifestAction {}

        impl jungle_sdk::types::Action for ManifestAction {
            type Id = jungle_sdk::types::Id<U0>;
            type Dependency = SharedState;
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

        #[derive(Flow)]
        struct ManifestInstinct(ActionTask<ManifestCreature, ManifestAction, UnitOkStep>);

        animal!(ManifestCreature, U0, ManifestInstinct);

        #[derive(Creatures)]
        struct ManifestCreatures(ManifestCreature);

        struct ManifestZoo;
        impl Ecosystem for ManifestZoo {
            type Creatures = ManifestCreatures;
        }

        let zoo = ManifestZoo;
        let _jungle_fut = zoo.manifest();
    }

    mod action_step;
    mod aspect;
    mod conditional;
    mod progression;
    mod while_loop;
}
