use jungle_sdk::prelude::*;
use std::marker::PhantomData;

#[derive(Flow)]
pub struct Loop2FlattenLeft<T, S>(PhantomData<fn() -> T>, PhantomData<fn() -> S>);

#[jungle::act]
impl<L, R, S> Act for Loop2FlattenLeft<(L, R), S> {
    type Effect = Noop;
    type Input = (L, R);
    type Output = L;
    type Carry = (L, R);

    fn emit(_state: &S, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut S,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Self::Output {
        output.expect("loop2 flatten-left step should complete");
        carry.0
    }
}

#[derive(Flow)]
pub struct Loop2<L, R, S>(Join<L, R>, Step<Loop2FlattenLeft<(L::Out, R::Out), S>>)
where
    L: TraverseFlow + Running,
    R: TraverseFlow + Running<In = L::In>;
