use std::marker::PhantomData;

use jungle_sdk::prelude::*;

pub struct Noop<St, In = ()>(PhantomData<(St, In)>);

#[jungle::action(carry = In)]
impl<St, In> Action for Noop<St, In> {
    type Effect = NoEffect;
    type Input = In;
    type Output = In;

    fn emit(_state: &St, input: Self::Input) -> ((), In) {
        ((), input)
    }

    fn absorb(
        _state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: In,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}
