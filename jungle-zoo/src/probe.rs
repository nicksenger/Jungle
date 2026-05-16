use jungle_sdk::typosaurus::num::consts::U255;

pub struct ProbeEffect;

impl jungle_sdk::types::EffectSchema for ProbeEffect {
    type Id = jungle_sdk::types::Id<U255>;
    type In = ();
    type Out = ();
    type Err = ();
}

impl<J> jungle_sdk::types::EffectExec<J> for ProbeEffect {
    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

//#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::JungleEffects)]
//impl jungle_sdk::types::Effects for ProbeEffect {
//    type List = jungle_sdk::typosaurus::collections::sp::Node<U255, ProbeEffect>;
//}

//#[jungle_sdk::sdk_primitive(property = jungle_sdk::types::Ident)]
//impl jungle_sdk::types::Identified for ProbeEffect {
//    type Id = U255;
//}

pub struct ProbeStep;
impl jungle_sdk::types::Act<ProbeAnimal> for ProbeStep {
    type Effect = ProbeEffect;
    type StateAspect = jungle_sdk::types::Identity;
    type Input = ();
    type Output = ();

    fn emit(
        _state: &<ProbeAnimal as jungle_sdk::types::Animal>::State,
        _input: Self::Input,
    ) -> <Self::Effect as jungle_sdk::types::EffectSchema>::In {
    }

    fn absorb(
        _state: &mut <ProbeAnimal as jungle_sdk::types::Animal>::State,
        _output: jungle_sdk::types::EffectCompletion<Self::Effect>,
    ) -> Self::Output {
    }
}

#[derive(jungle_sdk::Journey)]
pub struct ProbeJourney(jungle_sdk::types::Step<ProbeAnimal, ProbeStep>);

pub struct ProbeAnimal;

impl jungle_sdk::types::Animal for ProbeAnimal {
    type Id = jungle_sdk::types::Id<U255>;
    type Generation = jungle_sdk::typosaurus::num::consts::U0;
    type State = ();
    type Seed = ();
    type Journey = ProbeJourney;
}

impl jungle_sdk::types::Observable for ProbeAnimal {
    type Observation = jungle_sdk::types::NoopObservation;
}

impl jungle_sdk::types::Perturbable for ProbeAnimal {
    type Perturbation = jungle_sdk::types::NoopPerturbation;
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
