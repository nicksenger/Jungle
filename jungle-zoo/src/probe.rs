macro_rules! action {
    (
        $name:ident,
        $id:ty,
        dependency = $dependency_ty:ty
    ) => {
        pub struct $name;
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

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleActions)]
        impl jungle_sdk::types::Actions for $name {
            type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
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
        pub struct $name;
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
        pub struct $name;
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

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleActions)]
        impl jungle_sdk::types::Actions for $name {
            type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
        impl jungle_sdk::types::Identified for $name {
            type Id = $id;
        }
    };
}

macro_rules! animal {
    ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true, perturb = true) => {
        pub struct $name;

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::ObserveObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::TraitPerturbation;
        }
    };

    ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, perturb = true) => {
        pub struct $name;

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::NoopObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::TraitPerturbation;
        }
    };

    ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true) => {
        pub struct $name;

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::ObserveObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::NoopPerturbation;
        }
    };

    ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty) => {
        pub struct $name;

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::NoopObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::NoopPerturbation;
        }
    };

    ($name:ident, $id:ty, journey = $journey:ty) => {
        animal!($name, $id, state = (), journey = $journey);
    };

    ($name:ident, $id:ty, $journey:ty) => {
        animal!($name, $id, SharedState, $journey);
    };

    ($name:ident, $id:ty, $state:ty, $journey:ty) => {
        pub struct $name;
        impl jungle_sdk::types::AnimalMember for $name {}

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::NoopObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::NoopPerturbation;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
        impl jungle_sdk::types::Animals for $name {
            type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
        impl jungle_sdk::types::Identified for $name {
            type Id = $id;
        }
    };

    ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true, perturb = true) => {
        pub struct $name;
        impl jungle_sdk::types::AnimalMember for $name {}

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::ObserveObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::TraitPerturbation;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
        impl jungle_sdk::types::Animals for $name {
            type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
        impl jungle_sdk::types::Identified for $name {
            type Id = $id;
        }
    };

    ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true) => {
        pub struct $name;
        impl jungle_sdk::types::AnimalMember for $name {}

        impl jungle_sdk::types::Animal for $name {
            type Id = jungle_sdk::types::Id<$id>;
            type Generation = jungle_sdk::typosaurus::num::consts::U0;
            type State = $state;
            type Seed = $state;
            type Journey = $journey;
        }

        impl jungle_sdk::types::AnimalObservation for $name {
            type Adapter = jungle_sdk::types::ObserveObservation;
        }

        impl jungle_sdk::types::AnimalPerturbation for $name {
            type Adapter = jungle_sdk::types::NoopPerturbation;
        }

        #[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
        impl jungle_sdk::types::Animals for $name {
            type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
        }
    };
}

action!(ProbeAction, jungle_sdk::typosaurus::num::consts::U255);

pub struct ProbeStep;
impl jungle_sdk::types::Pulse<ProbeAnimal> for ProbeStep {
    type Action = ProbeAction;
    type Aspect = jungle_sdk::types::Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(
        _state: &<ProbeAnimal as jungle_sdk::types::Animal>::State,
        _input: Self::CarryIn,
    ) -> <Self::Action as jungle_sdk::types::Action>::In {}

    fn absorb(
        _state: &mut <ProbeAnimal as jungle_sdk::types::Animal>::State,
        _output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(jungle_sdk::types::Step<ProbeAnimal, ProbeStep>);

animal!(
    ProbeAnimal,
    jungle_sdk::typosaurus::num::consts::U255,
    state = (),
    journey = ProbeJourney
);

#[derive(jungle_sdk::Animals)]
pub struct ProbeZooAnimals(ProbeAnimal);

pub struct ProbeZoo;
impl jungle_sdk::types::Ecosystem for ProbeZoo {
    const NAME: &'static str = "probe-zoo";
    type Animals = ProbeZooAnimals;
}

impl From<&ProbeZoo> for () {
    fn from(_value: &ProbeZoo) -> Self {}
}
