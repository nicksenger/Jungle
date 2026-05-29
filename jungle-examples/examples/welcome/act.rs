use std::marker::PhantomData;

use crate::effect::{Rest as RestEffect, RestInput};
use jungle_sdk::prelude::*;

pub struct MergeUnit<S>(PhantomData<S>);
#[jungle::act]
impl<S> Act for MergeUnit<S> {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}
    fn absorb(_state: &mut S, output: EffectCompletion<Self::Effect>) -> Self::Output {}
}

pub struct MergeEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::act(carry = T)]
impl<T, S> Act for MergeEither<T, S> {
    type Effect = Noop;
    type Input = Either<T, T>;
    type Output = T;

    fn emit(_state: &S, input: Self::Input) -> ((), T) {
        match input {
            Either::Left(t) | Either::Right(t) => ((), t),
        }
    }
    fn absorb(_state: &mut S, output: EffectCompletion<Self::Effect>, carry: T) -> Self::Output {
        carry
    }
}

pub struct Rest<S, const REST_TICK: u32, const LANE_ID: u8>(PhantomData<S>);
#[jungle::act]
impl<S, const REST_TICK: u32, const LANE_ID: u8> Act for Rest<S, REST_TICK, LANE_ID> {
    type Effect = RestEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        RestInput {
            lane_id: LANE_ID,
            ticks: REST_TICK,
        }
    }
    fn absorb(_state: &mut S, _output: EffectCompletion<Self::Effect>) -> Self::Output {}
}
