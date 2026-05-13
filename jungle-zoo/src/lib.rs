//! Zoo crate with structural state, actions, and action adapters.

pub mod actions;
pub mod adapt;
pub mod animals;
pub mod state;
pub mod testing;

#[derive(jungle_sdk::Animals)]
pub struct ZooAnimals(animals::gorilla::Gorilla);

pub struct Zoo;
impl jungle_sdk::types::Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = ZooAnimals;
}

pub struct ProbeAction;
impl jungle_sdk::types::ActionMember for ProbeAction {}
impl jungle_sdk::types::Action for ProbeAction {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U255>;
    type Dependency = ();
    type In = u8;
    type Out = u8;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct ProbeStep;
impl jungle_sdk::types::Pulse<ProbeAnimal> for ProbeStep {
    type Action = ProbeAction;
    type Aspect = jungle_sdk::types::Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(state: &u8, _input: Self::CarryIn) -> <Self::Action as jungle_sdk::types::Action>::In {
        *state
    }

    fn absorb(
        state: &mut u8,
        output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
        *state = output.expect("probe action should succeed");
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(jungle_sdk::types::Step<ProbeAnimal, ProbeStep>);

pub struct ProbeAnimal;
impl jungle_sdk::types::AnimalMember for ProbeAnimal {}
impl jungle_sdk::types::Animal for ProbeAnimal {
    type Id = jungle_sdk::types::Id<jungle_sdk::typosaurus::num::consts::U255>;
    type Generation = jungle_sdk::typosaurus::num::consts::U0;
    type State = u8;
    type Seed = u8;
    type Journey = ProbeJourney;
}
impl jungle_sdk::types::AnimalObservation for ProbeAnimal {
    type Adapter = jungle_sdk::types::NoopObservation;
}
impl jungle_sdk::types::AnimalPerturbation for ProbeAnimal {
    type Adapter = jungle_sdk::types::NoopPerturbation;
}

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
