use std::marker::PhantomData;

use jungle_sdk::prelude::*;

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct Loop2Container<St> {
    counter: usize,
    #[jungle(focus)]
    st: St,
}
impl<S> Loop2Container<S> {
    pub fn new(st: S) -> Self {
        Self { counter: 0, st }
    }
}

impl<St> ViewProject<Loop2Container<St>> for Loop2Container<St> {
    fn project_view(state: &mut Self) -> &mut Loop2Container<St> {
        state
    }
}

pub struct Loop2SetCounter<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for Loop2SetCounter<St> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, _input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.counter = 2;
    }
}

pub struct Loop2DecCounter<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::action]
impl<St> Action for Loop2DecCounter<St> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, _input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        output.map_err(|_err| Failure::from("loop2 decrement step should succeed"))?;
        state.counter = state.counter.saturating_sub(1);
    }
}

pub struct FlattenEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::action]
impl<T, S> Action for FlattenEither<T, S> {
    type Effect = Noop;
    type Input = Either<T, T>;
    type Output = T;
    type Carry = Either<T, T>;

    fn emit(_state: &S, input: Self::Input) -> ((), Either<T, T>) {
        ((), input)
    }

    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
        carry: Either<T, T>,
    ) -> Self::Output {
        match carry {
            Either::Left(t) => t,
            Either::Right(t) => t,
        }
    }
}

pub struct Loop2CounterGt0;
impl<St> Predicate<(&Loop2Container<St>, &())> for Loop2CounterGt0 {
    fn eval((state, _): &(&Loop2Container<St>, &())) -> bool {
        state.counter > 0
    }
}

pub struct Loop2CounterIsEven;
impl<St> Predicate<(Loop2Container<St>, ())> for Loop2CounterIsEven {
    fn eval((state, _): &(Loop2Container<St>, ())) -> bool {
        state.counter % 2 == 0
    }
}

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
pub struct Loop2Body<St, L: TraverseFlow, R: TraverseFlow>(
    Conditional<FocusedCondition<Loop2CounterIsEven, Loop2Container<St>>, L, R>,
    Step<FlattenEither<(), Loop2Container<St>>>,
    Step<Loop2DecCounter<St>>,
);

#[derive(Flow)]
#[jungle(focus = Loop2Container<St>)]
pub struct Loop2<St, L: TraverseFlow, R: TraverseFlow>(
    Step<Loop2SetCounter<St>>,
    While<FocusedLoopCondition<Loop2CounterGt0, Loop2Container<St>>, Loop2Body<St, L, R>>,
);
