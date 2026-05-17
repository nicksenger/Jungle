use jungle_sdk::animal;
use jungle_sdk::effect;
use jungle_sdk::types::{
    Act, Animal, Conditional, EffectCompletion, Id, Identity, LoopCondition, ReplaceStep, Step,
    TraverseStep, While,
};
use jungle_sdk::typosaurus::assert_type_eq;
use jungle_sdk::typosaurus::num::consts::{U0, U20, U21, U22, U23, U24};

struct TraverseAEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for TraverseAEffect {
    type Id = Id<U20>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct TraverseBEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for TraverseBEffect {
    type Id = Id<U21>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct TraverseCEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for TraverseCEffect {
    type Id = Id<U22>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}
struct TraverseDEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for TraverseDEffect {
    type Id = Id<U23>;
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _d: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

struct TraverseAnimal;

struct StepA;
impl Act<TraverseAnimal> for StepA {
    type Effect = TraverseAEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step A should succeed");
    }
}

struct StepB;
impl Act<TraverseAnimal> for StepB {
    type Effect = TraverseBEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step B should succeed");
    }
}

struct StepC;
impl Act<TraverseAnimal> for StepC {
    type Effect = TraverseCEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step C should succeed");
    }
}

struct StepD;
impl Act<TraverseAnimal> for StepD {
    type Effect = TraverseDEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step D should succeed");
    }
}

struct KeepLooping;
impl LoopCondition<i32> for KeepLooping {
    type Arg = ();

    fn should_continue(state: &i32) -> bool {
        *state < 1
    }
}

type SourceFlow = Conditional<
    KeepLooping,
    Step<TraverseAnimal, StepA>,
    While<KeepLooping, Step<TraverseAnimal, StepB>>,
>;

struct Seen<T>(core::marker::PhantomData<T>);
struct TraverseMapper;
impl<Step> TraverseStep<Step> for TraverseMapper {
    type Output = Seen<Step>;
}

struct ReplaceMapper;
impl ReplaceStep<Step<TraverseAnimal, StepA>> for ReplaceMapper {
    type Output = Step<TraverseAnimal, StepC>;
}
impl ReplaceStep<Step<TraverseAnimal, StepB>> for ReplaceMapper {
    type Output = Step<TraverseAnimal, StepD>;
}

#[test]
fn traverse_and_replace_are_type_level_transformations() {
    type TraversedFlow = jungle_sdk::types::Traversed<SourceFlow, TraverseMapper>;
    type ExpectedTraversed = Conditional<
        KeepLooping,
        Seen<Step<TraverseAnimal, StepA>>,
        While<KeepLooping, Seen<Step<TraverseAnimal, StepB>>>,
    >;
    assert_type_eq!(TraversedFlow, ExpectedTraversed);

    type ReplacedFlow = jungle_sdk::types::Replace<SourceFlow, ReplaceMapper>;
    type ExpectedReplaced = Conditional<
        KeepLooping,
        Step<TraverseAnimal, StepC>,
        While<KeepLooping, Step<TraverseAnimal, StepD>>,
    >;
    assert_type_eq!(ReplacedFlow, ExpectedReplaced);
}

#[animal]
impl Animal for TraverseAnimal {
    type Id = Id<U24>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = SourceFlow;
}
