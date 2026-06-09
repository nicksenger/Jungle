use std::marker::PhantomData;
use std::time::Duration;

use jungle_sdk::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::condition::FlattenEither;
use crate::join::Pass;
use crate::loops::{Pred, WhileEnumerated};
use crate::predicate::Always;

use std::fmt::Display;
use std::io::Write;
pub struct Println<T>(PhantomData<T>);
#[jungle::effect(id = 1)]
impl<T, J> Effect<J> for Println<T>
where
    T: Serialize + DeserializeOwned + Send + Display + 'static,
{
    type In = T;
    type Out = T;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> + Send {
        async move {
            print!("{}\n", &input);
            let _ = std::io::stdout().flush();
            Ok(input)
        }
    }
}
pub struct Print<St, T>(PhantomData<St>, PhantomData<T>);
#[jungle::action]
impl<St, T> Action for Print<St, T>
where
    T: Serialize + DeserializeOwned + Send + Display + 'static,
{
    type Effect = Println<T>;
    type Input = T;
    type Output = T;

    fn emit(_state: &St, input: Self::Input) -> T {
        input
    }

    fn absorb(
        _state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(output?)
    }
}
#[derive(Flow)]
pub struct PrintFlow<St>(Step<Print<St, u32>>);

pub struct Fake<
    St,
    In,
    Out,
    Flo,
    const INITIAL_DELAY: u64,
    const MAX_DELAY: u64,
    const MULTIPLIER: u8,
>(
    PhantomData<St>,
    PhantomData<In>,
    PhantomData<Out>,
    PhantomData<Flo>,
);
#[jungle::action(carry = (In, Result<Out, Failure>))]
impl<St, In, Out, Flo, const INITIAL_DELAY: u64, const MAX_DELAY: u64, const MULTIPLIER: u8> Action
    for Fake<St, In, Out, Flo, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>
where
    Out: Default,
{
    type Effect = Sleep;
    type Input = (In, Result<Out, Failure>);
    type Output = (In, Result<Out, Failure>);

    fn emit(_state: &St, input: Self::Input) -> (Duration, (In, Result<Out, Failure>)) {
        (Duration::from_secs(1), input)
    }

    fn absorb(
        state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

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

#[derive(Flow)]
pub struct BackoffBody2<
    St,
    In: Clone + Serialize + DeserializeOwned + Send + 'static,
    Out: Serialize + DeserializeOwned + Send + 'static,
    Flo,
>(
    Step<CloneOver<St, In, Result<Out, Failure>>>,
    Join<Pass<St, In>, Attempt<Flo>>,
);

pub struct IfRightErr<St, In, Out>(St, In, Out);
impl<St, In, Out> Predicate<(&St, &(In, Result<Out, Failure>))> for IfRightErr<St, In, Out> {
    fn eval((_state, input): &(&St, &(In, Result<Out, Failure>))) -> bool {
        input.1.is_err()
    }
}

pub struct IfNonZero<St>(PhantomData<St>);
impl<St> Predicate<(&St, &u32)> for IfNonZero<St> {
    fn eval((_state, input): &(&St, &u32)) -> bool {
        !(**input == 0)
    }
}

#[derive(Flow)]
pub struct BackoffSleep<St, const INITIAL_DELAY: u64, const MAX_DELAY: u64, const MULTIPLIER: u8>(
    Step<SleepMult<St, INITIAL_DELAY, MAX_DELAY, MULTIPLIER>>,
);

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
#[jungle::action(carry = (In, Result<Out, Failure>))]
impl<St, In, Out> Action for WithErr<St, In, Out> {
    type Effect = NoEffect;
    type Input = In;
    type Output = (In, Result<Out, Failure>);

    fn emit(_state: &St, input: Self::Input) -> ((), (In, Result<Out, Failure>)) {
        ((), (input, Err(Failure::Message("test".to_string()))))
    }

    fn absorb(
        state: &mut St,
        _output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}
