use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::assert_type_eq;

struct TraverseAEffect;

#[jungle::effect(id = 20)]
impl<J> Effect<J> for TraverseAEffect {
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

#[jungle::effect(id = 21)]
impl<J> Effect<J> for TraverseBEffect {
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

#[jungle::effect(id = 22)]
impl<J> Effect<J> for TraverseCEffect {
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

#[jungle::effect(id = 23)]
impl<J> Effect<J> for TraverseDEffect {
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
impl BoundAction<TraverseAnimal> for StepA {
    type Effect = TraverseAEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();
    type Carry = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn emit_with_carry(
        view: &<<Self as BoundAction<TraverseAnimal>>::Aspect as StateCarrier<
            <TraverseAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<TraverseAnimal>>::emit(view, input), ())
    }

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step A should succeed");
    }
}

struct StepB;
impl BoundAction<TraverseAnimal> for StepB {
    type Effect = TraverseBEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();
    type Carry = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn emit_with_carry(
        view: &<<Self as BoundAction<TraverseAnimal>>::Aspect as StateCarrier<
            <TraverseAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<TraverseAnimal>>::emit(view, input), ())
    }

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step B should succeed");
    }
}

struct StepC;
impl BoundAction<TraverseAnimal> for StepC {
    type Effect = TraverseCEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();
    type Carry = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn emit_with_carry(
        view: &<<Self as BoundAction<TraverseAnimal>>::Aspect as StateCarrier<
            <TraverseAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<TraverseAnimal>>::emit(view, input), ())
    }

    fn absorb(_state: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("step C should succeed");
    }
}

struct StepD;
impl BoundAction<TraverseAnimal> for StepD {
    type Effect = TraverseDEffect;
    type Aspect = Identity;
    type Input = ();
    type Output = ();
    type Carry = ();

    fn emit(_state: &i32, _input: Self::Input) -> Self::Input {}

    fn emit_with_carry(
        view: &<<Self as BoundAction<TraverseAnimal>>::Aspect as StateCarrier<
            <TraverseAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<TraverseAnimal>>::emit(view, input), ())
    }

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
    BoundFlowStep<TraverseAnimal, StepA>,
    While<KeepLooping, BoundFlowStep<TraverseAnimal, StepB>>,
>;

struct Seen<T>(core::marker::PhantomData<T>);
struct TraverseMapper;
impl<Step> TraverseStep<Step> for TraverseMapper {
    type Output = Seen<Step>;
}

struct ReplaceMapper;
impl ReplaceStep<BoundFlowStep<TraverseAnimal, StepA>> for ReplaceMapper {
    type Output = BoundFlowStep<TraverseAnimal, StepC>;
}
impl ReplaceStep<BoundFlowStep<TraverseAnimal, StepB>> for ReplaceMapper {
    type Output = BoundFlowStep<TraverseAnimal, StepD>;
}

#[test]
fn traverse_and_replace_are_type_level_transformations() {
    type TraversedFlow = jungle_sdk::types::Traversed<SourceFlow, TraverseMapper>;
    type ExpectedTraversed = Conditional<
        KeepLooping,
        Seen<BoundFlowStep<TraverseAnimal, StepA>>,
        While<KeepLooping, Seen<BoundFlowStep<TraverseAnimal, StepB>>>,
    >;
    assert_type_eq!(TraversedFlow, ExpectedTraversed);

    type ReplacedFlow = jungle_sdk::types::Replace<SourceFlow, ReplaceMapper>;
    type ExpectedReplaced = Conditional<
        KeepLooping,
        BoundFlowStep<TraverseAnimal, StepC>,
        While<KeepLooping, BoundFlowStep<TraverseAnimal, StepD>>,
    >;
    assert_type_eq!(ReplacedFlow, ExpectedReplaced);
}

#[jungle::animal(id = 24, generation = 0)]
impl Animal for TraverseAnimal {
    type State = i32;
    type Seed = i32;
    type Journey = SourceFlow;
}
