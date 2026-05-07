#[cfg(test)]
extern crate jungle_sdk as inception;
#[cfg(test)]
extern crate jungle_sdk as jungle_types;

#[cfg(test)]
mod tests {

    use jungle_sdk::core::Jungle as _;
    use jungle_sdk::types::{
        Action, ActionCompletion, ActionSet, Impulse, Creature, CreatureActionSet, CreatureSet,
        CreatureStates, Ecosystem, Identity, Task,
    };
    use jungle_sdk::typosaurus::assert_type_eq;
    use jungle_sdk::typosaurus::list;
    use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3, U4, U5, U6};
    use jungle_sdk::{Actions, Creatures, Flow};
    use std::marker::PhantomData;

    macro_rules! action {
        (
            $name:ident,
            $id:ty,
            dependency = $dependency_ty:ty
        ) => {
            struct $name;
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type Dependency = $dependency_ty;
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

    action!(Eat, U0, dependency = SharedState);
    action!(Sleep, U1, dependency = SharedState);
    action!(Forage, U2, dependency = SharedState);
    action!(Drink, U3, dependency = SharedState);
    action!(Hunt, U4, dependency = SharedState);
    action!(Flee, U5, dependency = SharedState);

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

    struct UnitOkStep<A>(PhantomData<fn() -> A>);
    impl<T, A> Task<T> for UnitOkStep<A>
    where
        T: Creature,
        A: Action<In = ()>,
        A: Action<Out = (), Err = ()>,
    {
        type Action = A;
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
                Impulse<$animal, UnitOkStep<Eat>>,
                Impulse<$animal, UnitOkStep<Sleep>>,
                Impulse<$animal, UnitOkStep<Forage>>,
                Impulse<$animal, UnitOkStep<Drink>>,
                Impulse<$animal, UnitOkStep<Flee>>,
            );
        };
    }

    macro_rules! predator_instinct {
        ($name:ident, $animal:ty) => {
            #[derive(Flow)]
            struct $name(
                Impulse<$animal, UnitOkStep<Eat>>,
                Impulse<$animal, UnitOkStep<Sleep>>,
                Impulse<$animal, UnitOkStep<Forage>>,
                Impulse<$animal, UnitOkStep<Drink>>,
                Impulse<$animal, UnitOkStep<Hunt>>,
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
            Impulse<StatefulGorilla, UnitOkStep<Eat>>,
            Impulse<StatefulGorilla, UnitOkStep<Sleep>>,
            Impulse<StatefulGorilla, UnitOkStep<Forage>>,
            Impulse<StatefulGorilla, UnitOkStep<Drink>>,
            Impulse<StatefulGorilla, UnitOkStep<Flee>>,
        );

        #[derive(Flow)]
        struct StatefulTigerInstinct(
            Impulse<StatefulTiger, UnitOkStep<Eat>>,
            Impulse<StatefulTiger, UnitOkStep<Sleep>>,
            Impulse<StatefulTiger, UnitOkStep<Forage>>,
            Impulse<StatefulTiger, UnitOkStep<Drink>>,
            Impulse<StatefulTiger, UnitOkStep<Hunt>>,
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
        let zoo = Zoo;
        let _jungle_fut = zoo.manifest();
    }

    #[test]
    fn jungle_executor_runs_actions_with_ecosystem_dependency() {
        use jungle_sdk::core::JungleExecutor;
        use jungle_sdk::types::{Impulse, Task};
        use jungle_sdk::Instinct;
        use jungle_sdk::typosaurus::num::consts::U7;
        use std::future::Future;
        use std::pin::pin;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn run_now<F: Future>(future: F) -> F::Output {
            fn raw_waker() -> RawWaker {
                fn clone(_: *const ()) -> RawWaker {
                    raw_waker()
                }
                fn wake(_: *const ()) {}
                fn wake_by_ref(_: *const ()) {}
                fn drop(_: *const ()) {}
                RawWaker::new(
                    std::ptr::null(),
                    &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
                )
            }

            let waker = unsafe { Waker::from_raw(raw_waker()) };
            let mut cx = Context::from_waker(&waker);
            let mut future = pin!(future);
            match Future::poll(future.as_mut(), &mut cx) {
                Poll::Ready(output) => output,
                Poll::Pending => panic!("test action future must resolve immediately"),
            }
        }

        struct TestAction;
        impl jungle_sdk::types::ActionMember for TestAction {}

        impl jungle_sdk::types::Action for TestAction {
            type Id = jungle_sdk::types::Id<U7>;
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

        struct TestStep;
        impl Task<TestCreature> for TestStep {
            type Action = TestAction;
            type Aspect = Identity;
            type In = ();
            type Out = ();

            fn prepare(_state: &SharedState, _input: Self::In) -> <Self::Action as Action>::In {}

            fn process(
                _state: &mut SharedState,
                output: ActionCompletion<Self::Action>,
            ) -> Self::Out {
                output.expect("test action should succeed");
            }
        }

        #[derive(Instinct)]
        struct TestInstinct(Impulse<TestCreature, TestStep>);

        animal!(TestCreature, U7, SharedState, TestInstinct);

        let zoo = Zoo;
        let mut executor = JungleExecutor::<Zoo, TestCreature>::new(&zoo, SharedState);
        let request = executor
            .next_executable_request(())
            .expect("zoo request should build");
        let request_input: () = request
            .deserialize_request()
            .expect("request should deserialize");
        assert_eq!(request_input, ());

        let completion = run_now(request.run()).expect("zoo action should execute");
        let _emitted = executor
            .complete_serialized(completion)
            .expect("completion should process");
        assert!(executor.is_complete());
    }

    mod action_step;
    mod aspect;
    mod conditional;
    mod progression;
    mod while_loop;
}
