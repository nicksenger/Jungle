use std::marker::PhantomData;

use jungle_sdk::prelude::*;

pub struct FlattenEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::action]
impl<T, S> Action for FlattenEither<T, S> {
    type Effect = NoEffect;
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
    ) -> Result<Self::Output, Failure> {
        Ok(match carry {
            Either::Left(value) | Either::Right(value) => value,
        })
    }
}
