use jungle_sdk::prelude::*;

use crate::effect::DecrementCounterEffect;

pub struct DecrementCounter<Focus>(core::marker::PhantomData<fn() -> Focus>);

#[jungle::act(aspect = Focus)]
impl<Focus> Act for DecrementCounter<Focus> {
    type Effect = DecrementCounterEffect;
    type Input = ();
    type Output = ();

    fn emit(_view: &u8, _input: Self::Input) -> Self::Input {}

    fn absorb(view: &mut u8, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("counter decrement should succeed");
        *view = view.saturating_sub(1);
    }
}
