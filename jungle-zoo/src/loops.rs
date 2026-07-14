use std::marker::PhantomData;

use jungle_sdk::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

pub struct InitIter<St, In>(PhantomData<St>, PhantomData<In>);
#[jungle::action(carry = In, name = "InitLoopIter")]
impl<St, In> Action for InitIter<St, In>
where
    In: Serialize + DeserializeOwned + Send + 'static,
{
    type Effect = NoEffect;
    type Input = In;
    type Output = (u32, In);

    fn emit(_state: &St, input: Self::Input) -> ((), In) {
        ((), input)
    }

    fn absorb(
        _state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: In,
    ) -> Result<Self::Output, Failure> {
        Ok((0, carry))
    }
}

pub struct Pred<St, In, P2>(PhantomData<St>, PhantomData<In>, PhantomData<P2>);
impl<St, In, P2> Predicate<(&St, &(u32, In))> for Pred<St, In, P2>
where
    P2: for<'a> Predicate<(&'a St, &'a In)>,
{
    fn eval((state, input): &(&St, &(u32, In))) -> bool {
        <P2 as Predicate<(&St, &In)>>::eval(&(state, &input.1))
    }
}

#[derive(Flow)]
pub struct WhileEnumerated<St, In: Serialize + DeserializeOwned + Send + 'static, P2, Flo>(
    Step<InitIter<St, In>>,
    While<Pred<St, In, P2>, WhileEnumeratedBody<St, In, Flo>>,
);

#[derive(Flow)]
pub struct WhileEnumeratedBody<St, In: Serialize + DeserializeOwned + Send + 'static, Flo>(
    Step<IncIter<St, In>>,
    Flo,
);

pub struct IncIter<St, In>(PhantomData<St>, PhantomData<In>);
#[jungle::action(carry = (u32, In))]
impl<St, In> Action for IncIter<St, In>
where
    In: Serialize + DeserializeOwned + Send + 'static,
{
    type Effect = NoEffect;
    type Input = (u32, In);
    type Output = (u32, In);

    fn emit(_state: &St, input: Self::Input) -> ((), (u32, In)) {
        ((), input)
    }

    fn absorb(
        _state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: (u32, In),
    ) -> Result<Self::Output, Failure> {
        Ok((carry.0 + 1, carry.1))
    }
}
