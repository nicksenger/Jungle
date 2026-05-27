use jungle_sdk::prelude::*;
use std::marker::PhantomData;

pub struct Loop2FlattenUnit<S>(PhantomData<fn() -> S>);

#[jungle::act]
impl<S> Act for Loop2FlattenUnit<S> {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();
    type Carry = ((), ());

    fn emit(_state: &S, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut S,
        output: EffectCompletion<Self::Effect>,
        _carry: Self::Carry,
    ) -> Self::Output {
        output.expect("loop2 flatten-unit step should complete");
    }
}

#[derive(Flow)]
pub struct Loop2WithState<L, R, S>(Join<L, R>, Step<Loop2FlattenUnit<S>>);

pub type Loop2<L, R> = Loop2WithState<L, R, ()>;
