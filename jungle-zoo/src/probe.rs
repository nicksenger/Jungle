use jungle_sdk::typosaurus::num::consts::U255;

pub struct ProbeAction;
impl jungle_sdk::types::ActionMember for ProbeAction {}

impl jungle_sdk::types::Action for ProbeAction {
    type Id = jungle_sdk::types::Id<U255>;
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

//#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleActions)]
//impl jungle_sdk::types::Actions for ProbeAction {
//    type List = jungle_sdk::typosaurus::collections::sp::Node<U255, ProbeAction>;
//}

//#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
//impl jungle_sdk::types::Identified for ProbeAction {
//    type Id = U255;
//}

pub struct ProbeStep;
impl jungle_sdk::types::Pulse<ProbeAnimal> for ProbeStep {
    type Action = ProbeAction;
    type Aspect = jungle_sdk::types::Identity;
    type CarryIn = ();
    type CarryOut = ();

    fn emit(
        _state: &<ProbeAnimal as jungle_sdk::types::Animal>::State,
        _input: Self::CarryIn,
    ) -> <Self::Action as jungle_sdk::types::Action>::In {
    }

    fn absorb(
        _state: &mut <ProbeAnimal as jungle_sdk::types::Animal>::State,
        _output: jungle_sdk::types::ActionCompletion<Self::Action>,
    ) -> Self::CarryOut {
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(jungle_sdk::types::Step<ProbeAnimal, ProbeStep>);

pub struct ProbeAnimal;
impl jungle_sdk::types::AnimalMember for ProbeAnimal {}

impl jungle_sdk::types::Animal for ProbeAnimal {
    type Id = jungle_sdk::types::Id<U255>;
    type Generation = jungle_sdk::typosaurus::num::consts::U0;
    type State = ();
    type Seed = ();
    type Journey = ProbeJourney;
}

impl jungle_sdk::types::AnimalObservation for ProbeAnimal {
    type Adapter = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::AnimalPerturbation for ProbeAnimal {
    type Adapter = jungle_sdk::types::NoopPerturbation;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleAnimals)]
impl jungle_sdk::types::Animals for ProbeAnimal {
    type List = jungle_sdk::typosaurus::collections::sp::Node<U255, ProbeAnimal>;
}

#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
impl jungle_sdk::types::Identified for ProbeAnimal {
    type Id = U255;
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
