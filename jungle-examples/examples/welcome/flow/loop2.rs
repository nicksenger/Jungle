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
#[jungle::act]
impl<St> Act for Loop2SetCounter<St> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        state.counter = 2;
    }
}

pub struct Loop2DecCounter<St>(core::marker::PhantomData<fn() -> St>);
#[allow(private_interfaces)]
#[jungle::act]
impl<St> Act for Loop2DecCounter<St> {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &Loop2Container<St>, input: Self::Input) {}

    fn absorb(
        state: &mut Loop2Container<St>,
        output: EffectCompletion<Self::Effect>,
    ) -> Self::Output {
        let value = output.expect("loop2 decrement step should succeed");
        state.counter = state.counter.saturating_sub(1);
    }
}

pub struct FlattenEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::act]
impl<T, S> Act for FlattenEither<T, S> {
    type Effect = Noop;
    type Input = Either<T, T>;
    type Output = T;
    type Carry = Either<T, T>;

    fn emit(_state: &S, input: Self::Input) -> ((), Either<T, T>) {
        ((), input)
    }

    fn absorb(
        state: &mut S,
        output: EffectCompletion<Self::Effect>,
        carry: Either<T, T>,
    ) -> Self::Output {
        match carry {
            Either::Left(t) => t,
            Either::Right(t) => t,
        }
    }
}

pub struct Loop2CounterGt0;
impl<St> LoopCondition<Loop2Container<St>> for Loop2CounterGt0 {
    type Arg = ();

    fn should_continue(state: &Loop2Container<St>) -> bool {
        state.counter > 0
    }
}

pub struct Loop2CounterIsEven;
impl<St> Condition<(Loop2Container<St>, ())> for Loop2CounterIsEven {
    fn choose((state, _): &(Loop2Container<St>, ())) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(
        Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
    )]
    struct Loop2TraceState {
        left_hits: u8,
        right_hits: u8,
        order: u8,
    }

    pub struct Loop2LeftSpec;
    #[jungle::act]
    impl Act for Loop2LeftSpec {
        type Effect = Noop;
        type Input = ();
        type Output = ();

        fn emit(_state: &Loop2TraceState, _input: Self::Input) {}

        fn absorb(
            state: &mut Loop2TraceState,
            output: EffectCompletion<Self::Effect>,
        ) -> Self::Output {
            output.expect("left loop2 arm should succeed");
            state.left_hits = state.left_hits.saturating_add(1);
            state.order = state.order.saturating_mul(10).saturating_add(1);
        }
    }

    pub struct Loop2RightSpec;
    #[jungle::act]
    impl Act for Loop2RightSpec {
        type Effect = Noop;
        type Input = ();
        type Output = ();

        fn emit(_state: &Loop2TraceState, _input: Self::Input) {}

        fn absorb(
            state: &mut Loop2TraceState,
            output: EffectCompletion<Self::Effect>,
        ) -> Self::Output {
            output.expect("right loop2 arm should succeed");
            state.right_hits = state.right_hits.saturating_add(1);
            state.order = state.order.saturating_mul(10).saturating_add(2);
        }
    }

    #[derive(Flow)]
    #[jungle(focus = Loop2TraceState)]
    struct Loop2LeftFlow(Step<Loop2LeftSpec>);

    #[derive(Flow)]
    #[jungle(focus = Loop2TraceState)]
    struct Loop2RightFlow(Step<Loop2RightSpec>);

    #[derive(
        Optic, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
    )]
    struct Loop2HarnessState {
        #[jungle(focus)]
        loop2: Loop2Container<Loop2TraceState>,
    }

    impl From<Loop2TraceState> for Loop2HarnessState {
        fn from(seed: Loop2TraceState) -> Self {
            Self {
                loop2: Loop2Container::new(seed),
            }
        }
    }

    type Loop2HarnessJourney = Loop2<Loop2TraceState, Loop2LeftFlow, Loop2RightFlow>;

    struct Loop2HarnessAnimal;
    #[jungle::animal(id = 900, generation = 0)]
    impl Animal for Loop2HarnessAnimal {
        type State = Loop2HarnessState;
        type Seed = Loop2TraceState;
        type Journey = Loop2HarnessJourney;
    }

    #[test]
    fn loop2_runs_left_then_right_in_isolated_case() {
        let mut exec = ManualExecutor::<Loop2HarnessAnimal>::new(Loop2HarnessState::from(
            Loop2TraceState::default(),
        ));

        assert!(!exec.is_complete());
        let step: Result<(), jungle_sdk::ExecutorError> = exec.next_typed((), Ok::<(), ()>(()));
        assert!(matches!(step, Err(jungle_sdk::ExecutorError::Complete)));
        assert!(exec.is_complete());

        let state = exec.into_state();
        assert_eq!(state.loop2.counter, 0);
        assert_eq!(state.loop2.st.left_hits, 1);
        assert_eq!(state.loop2.st.right_hits, 1);
        assert_eq!(state.loop2.st.order, 12);
    }
}
