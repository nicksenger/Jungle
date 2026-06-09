use std::marker::PhantomData;
use std::time::Duration;

use jungle_sdk::prelude::*;

pub struct Millis<const N: u64>;
pub struct Seconds<const N: u64>;
pub struct Minutes<const N: u64>;
pub struct Hours<const N: u64>;
pub struct Days<const N: u64>;

pub trait DurationLike {
    fn duration() -> Duration;
}
impl<const N: u64> DurationLike for Millis<N> {
    fn duration() -> Duration {
        Duration::from_millis(N)
    }
}
impl<const N: u64> DurationLike for Seconds<N> {
    fn duration() -> Duration {
        Duration::from_secs(N)
    }
}
impl<const N: u64> DurationLike for Minutes<N> {
    fn duration() -> Duration {
        Duration::from_secs(N * 60)
    }
}
impl<const N: u64> DurationLike for Hours<N> {
    fn duration() -> Duration {
        Duration::from_secs(N * 60 * 60)
    }
}
impl<const N: u64> DurationLike for Days<N> {
    fn duration() -> Duration {
        Duration::from_secs(N * 60 * 60 * 24)
    }
}

pub struct SleepFor<S, D, T = ()>(PhantomData<S>, PhantomData<D>, PhantomData<T>);
#[jungle::action(carry = T)]
impl<S, D, T> Action for SleepFor<S, D, T>
where
    D: DurationLike,
{
    type Effect = Sleep;
    type Input = T;
    type Output = T;

    fn emit(_state: &S, input: Self::Input) -> (Duration, T) {
        (<D as DurationLike>::duration(), input)
    }

    fn absorb(
        _state: &mut S,
        _output: EffectCompletion<Self::Effect>,
        carry: T,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}
