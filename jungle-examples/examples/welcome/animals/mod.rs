#![allow(dead_code)]

use jungle_sdk::prelude::*;
use num::{U0, U1, U2, U3, U4, U10, U11, U12, U13, U14};

macro_rules! define_stub_animal {
    (
        animal: $animal:ident,
        state: $state:ident,
        seed: $seed:ident,
        effect: $effect:ident,
        step: $step:ident,
        flow: $flow:ident,
        animal_id: $animal_id:ty,
        effect_id: $effect_id:ty
    ) => {
        pub type $state = ();
        pub type $seed = ();

        pub struct $effect;

        impl EffectSchema for $effect {
            type Id = Id<$effect_id>;
            type In = ();
            type Out = ();
            type Err = ();
        }

        impl<J> Effect<J> for $effect {
            fn effect(
                _jungle: &J,
                _input: Self::In,
            ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                std::future::ready(Ok(()))
            }
        }

        pub struct $step;

        #[jungle::act]
        impl Act for $step {
            type Effect = $effect;
            type Input = ();
            type Output = ();

            fn emit(_state: &$state, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}

            fn absorb(
                _state: &mut $state,
                _output: EffectCompletion<Self::Effect>,
            ) -> Self::Output {
            }
        }

        #[derive(Flow)]
        pub struct $flow(Step<$step>);

        pub struct $animal;

        impl Animal for $animal {
            type Id = Id<$animal_id>;
            type Generation = U0;
            type State = $state;
            type Seed = $seed;
            type Journey = $flow;
        }
    };
}

define_stub_animal!(
    animal: LeadVocalist,
    state: LeadVocalistState,
    seed: LeadVocalistSeed,
    effect: LeadVocalistStubEffect,
    step: LeadVocalistStubStep,
    flow: LeadVocalistFlow,
    animal_id: U0,
    effect_id: U10
);

define_stub_animal!(
    animal: LeadGuitarist,
    state: LeadGuitaristState,
    seed: LeadGuitaristSeed,
    effect: LeadGuitaristStubEffect,
    step: LeadGuitaristStubStep,
    flow: LeadGuitaristFlow,
    animal_id: U1,
    effect_id: U11
);

define_stub_animal!(
    animal: RhythmGuitarist,
    state: RhythmGuitaristState,
    seed: RhythmGuitaristSeed,
    effect: RhythmGuitaristStubEffect,
    step: RhythmGuitaristStubStep,
    flow: RhythmGuitaristFlow,
    animal_id: U2,
    effect_id: U12
);

define_stub_animal!(
    animal: Bass,
    state: BassState,
    seed: BassSeed,
    effect: BassStubEffect,
    step: BassStubStep,
    flow: BassFlow,
    animal_id: U3,
    effect_id: U13
);

define_stub_animal!(
    animal: Drums,
    state: DrumsState,
    seed: DrumsSeed,
    effect: DrumsStubEffect,
    step: DrumsStubStep,
    flow: DrumsFlow,
    animal_id: U4,
    effect_id: U14
);
