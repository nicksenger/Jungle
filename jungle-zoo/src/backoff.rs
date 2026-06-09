use std::marker::PhantomData;
use std::time::Duration;

use jungle_sdk::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::condition::FlattenEither;
use crate::join::Pass;
use crate::loops::{Pred, WhileEnumerated};

#[derive(Flow)]
pub struct Backoff<
    St,
    In: Clone + Serialize + DeserializeOwned + Send + 'static,
    Out: Serialize + DeserializeOwned + Send + 'static,
    Flo,
    const INITIAL_DELAY: u64,
    const MAX_DELAY: u64,
    const MULTIPLIER: u8,
>(
    Step<WithErr<St, In, Out>>,
    WhileEnumerated<
        St,
        (In, Result<Out, Failure>),
        IfRightErr<St, In, Out>,
        BackoffBody<St, In, Out, Flo, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>,
    >,
);

#[derive(Flow)]
pub struct BackoffBody<
    St,
    In: Clone + Serialize + DeserializeOwned + Send + 'static,
    Out: Serialize + DeserializeOwned + Send + 'static,
    Flo,
    const INITIAL_DELAY: u64,
    const MAX_DELAY: u64,
    const MULTIPLIER: u8,
>(Join<BackoffSleep<St, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>, BackoffBody2<St, In, Out, Flo>>);
pub struct BackoffBody2<
    St,
    In: Clone + Serialize + DeserializeOwned + Send + 'static,
    Out: Serialize + DeserializeOwned + Send + 'static,
    Flo,
>(
    Step<CloneOver<St, In, Result<Out, Failure>>>,
    Join<Pass<St, In>, BackoffBody3<Flo>>,
);
pub struct BackoffBody3<Flo>(Attempt<Flo>);

pub struct IfRightErr<St, In, Out>(St, In, Out);
impl<St, In, Out> Predicate<(&St, &(In, Result<Out, Failure>))> for IfRightErr<St, In, Out> {
    fn eval((_state, input): &(&St, &(In, Result<Out, Failure>))) -> bool {
        input.1.is_err()
    }
}

pub struct IfNonZero<St>(St);
impl<St> Predicate<(&St, &u32)> for IfNonZero<St> {
    fn eval((_state, input): &(&St, &u32)) -> bool {
        !(**input == 0)
    }
}

#[derive(Flow)]
pub struct BackoffSleep<St, const INITIAL_DELAY: u64, const MAX_DELAY: u64, const MULTIPLIER: u8>(
    Conditional<
        IfNonZero<St>,
        BackoffSleepBody<St, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>,
        Pass<St, u32>,
    >,
);

#[derive(Flow)]
pub struct BackoffSleepBody<
    St,
    const INITIAL_DELAY: u64,
    const MAX_DELAY: u64,
    const MULTIPLIER: u8,
>(Step<SleepMult<St, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>>);

pub struct SleepMult<St, const INITIAL_DELAY: u64, const MAX_DELAY: u64, const MULTIPLIER: u8>(
    PhantomData<St>,
);
#[jungle::action(carry = u32)]
impl<St, const INITIAL_DELAY: u64, const MAX_DELAY: u64, const MULTIPLIER: u8> Action
    for SleepMult<St, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>
{
    type Effect = Sleep;
    type Input = u32;
    type Output = u32;

    fn emit(_state: &St, input: Self::Input) -> (Duration, u32) {
        (
            Duration::from_millis(
                (input as u64 * MULTIPLIER as u64 * INITIAL_DELAY).min(MAX_DELAY),
            ),
            input,
        )
    }

    fn absorb(
        state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct CloneOver<St, In, U>(PhantomData<St>, PhantomData<In>, PhantomData<U>);
#[jungle::action(carry = (In, U))]
impl<St, In, U> Action for CloneOver<St, In, U>
where
    In: Clone,
{
    type Effect = NoEffect;
    type Input = (In, U);
    type Output = (In, In);

    fn emit(_state: &St, input: Self::Input) -> ((), (In, U)) {
        ((), input)
    }

    fn absorb(
        state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok((carry.0.clone(), carry.0))
    }
}

pub struct WithErr<St, In, Out>(PhantomData<St>, PhantomData<In>, PhantomData<Out>);
#[jungle::action(carry = In)]
impl<St, In, Out> Action for WithErr<St, In, Out> {
    type Effect = NoEffect;
    type Input = In;
    type Output = (In, Result<Out, Failure>);

    fn emit(_state: &St, input: Self::Input) -> ((), In) {
        ((), input)
    }

    fn absorb(
        state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok((carry, Err(Failure::Message("test".to_string()))))
    }
}
