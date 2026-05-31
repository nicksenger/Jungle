use std::marker::PhantomData;

use crate::effect::{Rest as RestEffect, RestInput};
use jungle_sdk::prelude::*;

pub struct MergeUnit<S>(PhantomData<S>);
#[jungle::action]
impl<S> Action for MergeUnit<S> {
    type Effect = Noop;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {}
    fn absorb(
        _state: &mut S,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = (|| {})();
        Ok(__absorb_out_1)
    }
}

pub struct MergeEither<T, S>(PhantomData<T>, PhantomData<S>);
#[jungle::action(carry = T)]
impl<T, S> Action for MergeEither<T, S> {
    type Effect = Noop;
    type Input = Either<T, T>;
    type Output = T;

    fn emit(_state: &S, input: Self::Input) -> ((), T) {
        match input {
            Either::Left(t) | Either::Right(t) => ((), t),
        }
    }
    fn absorb(
        _state: &mut S,
        output: EffectCompletion<Self::Effect>,
        carry: T,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = (|| carry)();
        Ok(__absorb_out_2)
    }
}

pub struct Rest<S, const REST_TICK: u32, const LANE_ID: u8>(PhantomData<S>);
#[jungle::action]
impl<S, const REST_TICK: u32, const LANE_ID: u8> Action for Rest<S, REST_TICK, LANE_ID> {
    type Effect = RestEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &S, _input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        RestInput {
            lane_id: LANE_ID,
            ticks: REST_TICK,
        }
    }
    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = (|| {})();
        Ok(__absorb_out_3)
    }
}
