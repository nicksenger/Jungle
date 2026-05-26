use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::assert_type_eq;
use num::U910;

pub struct GenericActAnimal;
#[jungle::animal(id = 910, generation = 0)]
impl Animal for GenericActAnimal {
    type State = ();
    type Seed = ();
    type Journey = GenericActFlow;
}

pub struct GenericActEffect<const NOTE: u8>;
#[jungle::effect(id = 911)]
impl<const NOTE: u8, J> Effect<J> for GenericActEffect<NOTE> {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

pub struct GenericActSpec<const NOTE: u8, const D_TICK: u8>;
#[jungle::act]
impl<const NOTE: u8, const D_TICK: u8> Act for GenericActSpec<NOTE, D_TICK> {
    type Effect = GenericActEffect<NOTE>;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> Self::Input {}

    fn absorb(_state: &mut (), output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("generic act effect should succeed");
    }
}

#[derive(Flow)]
pub struct GenericActFlow(Step<GenericActSpec<7, 3>>);

pub struct CarryActAnimal;
#[jungle::animal(id = 912, generation = 0)]
impl Animal for CarryActAnimal {
    type State = i32;
    type Seed = i32;
    type Journey = CarryActFlow;
}

pub struct CarryActEffect;
#[jungle::effect(id = 913)]
impl<J> Effect<J> for CarryActEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct CarryActSpec;
#[jungle::act]
impl Act for CarryActSpec {
    type Effect = CarryActEffect;
    type Input = i32;
    type Output = i32;
    type Carry = i32;

    fn emit(state: &i32, input: Self::Input) -> (i32, Self::Carry) {
        (input + *state, *state - input)
    }

    fn absorb(
        state: &mut i32,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Self::Output {
        let value = output.expect("carry act effect should succeed");
        *state = value + carry;
        *state
    }
}

#[derive(Flow)]
pub struct CarryActFlow(Step<CarryActSpec>);

fn assert_bound<T: BoundAct<GenericActAnimal>>() {}

#[test]
fn generic_act_attr_generates_bind_type() {
    type Bound = <GenericActSpec<7, 3> as Act>::Bind<GenericActAnimal>;
    assert_bound::<Bound>();
    assert_type_eq!(
        <Bound as BoundAct<GenericActAnimal>>::Effect,
        GenericActEffect<7>
    );
    assert_type_eq!(<Bound as BoundAct<GenericActAnimal>>::Input, ());
    assert_type_eq!(<Bound as BoundAct<GenericActAnimal>>::Output, ());
    assert_type_eq!(<Bound as BoundAct<GenericActAnimal>>::Carry, ());
    assert_type_eq!(<GenericActAnimal as Animal>::Id, Id<U910>);
}

#[test]
fn act_attr_supports_explicit_carry() {
    type Bound = <CarryActSpec as Act>::Bind<CarryActAnimal>;
    assert_type_eq!(<CarryActSpec as Act>::Carry, i32);
    assert_type_eq!(<Bound as BoundAct<CarryActAnimal>>::Carry, i32);

    let (state, (request, carry)) = <BoundFlowStep<CarryActAnimal, Bound> as Running>::run((2, 5));
    assert_eq!(request.into_input(), 7);
    assert_eq!(carry, -3);

    let (state, emitted) =
        <BoundFlowStep<CarryActAnimal, Bound> as Waiting>::accept((state, Ok(11), carry));
    assert_eq!(state, 8);
    assert_eq!(emitted, 8);

    let mut executor = ManualExecutor::<CarryActAnimal>::new(2);
    let emitted: i32 = executor
        .next_typed(5, Ok::<i32, ()>(11))
        .expect("carry step should complete");
    assert_eq!(emitted, 8);
    assert_eq!(executor.into_state(), 8);
}
