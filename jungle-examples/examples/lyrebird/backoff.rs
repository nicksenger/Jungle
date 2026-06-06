use jungle_sdk::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;

pub trait BackoffAction: Action<Output = Result<Self::Success, Self::Error>> {
    type Success;
    type Error;
}

impl<A, Success, Error> BackoffAction for A
where
    A: Action<Output = Result<Success, Error>>,
{
    type Success = Success;
    type Error = Error;
}

pub trait CloneBackoffAction: BackoffAction
where
    Self::Input: Clone,
{
}

impl<A> CloneBackoffAction for A
where
    A: BackoffAction,
    A::Input: Clone,
{
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExponentialBackoffPolicy {
    pub initial_delay_ms: u64,
    pub multiplier: u32,
    pub max_delay_ms: u64,
}

impl ExponentialBackoffPolicy {
    fn next_delay_ms(self, current_delay_ms: u64) -> u64 {
        if current_delay_ms == 0 {
            return 0;
        }

        let scaled = current_delay_ms.saturating_mul(u64::from(self.multiplier.max(1)));
        if self.max_delay_ms == 0 {
            scaled
        } else {
            scaled.min(self.max_delay_ms)
        }
    }
}

impl Default for ExponentialBackoffPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 100,
            multiplier: 2,
            max_delay_ms: 10_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExponentialBackoffInput<Input> {
    pub action_input: Input,
    pub policy: ExponentialBackoffPolicy,
}

#[derive(Optic, Clone, Debug, PartialEq)]
pub struct ExponentialBackoffState<St, A>
where
    A: BackoffAction,
{
    pub attempts: u32,
    pub current_delay_ms: u64,
    pub policy: ExponentialBackoffPolicy,
    pub action_input: Option<A::Input>,
    pub last_result: Option<Result<A::Success, A::Error>>,
    #[jungle(focus)]
    pub st: St,
}

impl<St, A> ExponentialBackoffState<St, A>
where
    A: BackoffAction,
{
    pub fn new(st: St) -> Self {
        Self {
            attempts: 0,
            current_delay_ms: 0,
            policy: ExponentialBackoffPolicy::default(),
            action_input: None,
            last_result: None,
            st,
        }
    }
}

impl<St, A> Default for ExponentialBackoffState<St, A>
where
    St: Default,
    A: BackoffAction,
{
    fn default() -> Self {
        Self::new(St::default())
    }
}

impl<St, A> ViewProject<ExponentialBackoffState<St, A>> for ExponentialBackoffState<St, A>
where
    A: BackoffAction,
{
    fn project_view(state: &mut Self) -> &mut ExponentialBackoffState<St, A> {
        state
    }
}

pub struct InitializeBackoff<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for InitializeBackoff<St, A>
where
    A: BackoffAction,
{
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<A::Input>;
    type Output = ();
    type Carry = ExponentialBackoffInput<A::Input>;

    fn emit(
        _state: &ExponentialBackoffState<St, A>,
        input: Self::Input,
    ) -> ((), ExponentialBackoffInput<A::Input>) {
        ((), input)
    }

    fn absorb(
        state: &mut ExponentialBackoffState<St, A>,
        _output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<A::Input>,
    ) -> Result<Self::Output, Failure> {
        state.attempts = 0;
        state.current_delay_ms = carry.policy.initial_delay_ms;
        state.policy = carry.policy;
        state.action_input = Some(carry.action_input);
        state.last_result = None;
        Ok(())
    }
}

pub struct CloneBackoffInput<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for CloneBackoffInput<St, A>
where
    A: BackoffAction,
    A::Input: Clone,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = A::Input;
    type Carry = A::Input;

    fn emit(state: &ExponentialBackoffState<St, A>, _input: Self::Input) -> ((), A::Input) {
        (
            (),
            state
                .action_input
                .as_ref()
                .expect("backoff action input should be configured before retry loop starts")
                .clone(),
        )
    }

    fn absorb(
        _state: &mut ExponentialBackoffState<St, A>,
        _output: EffectCompletion<Self::Effect>,
        carry: A::Input,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct RecordBackoffResult<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for RecordBackoffResult<St, A>
where
    A: BackoffAction,
{
    type Effect = NoEffect;
    type Input = Result<A::Success, A::Error>;
    type Output = ();
    type Carry = Result<A::Success, A::Error>;

    fn emit(
        _state: &ExponentialBackoffState<St, A>,
        input: Self::Input,
    ) -> ((), Result<A::Success, A::Error>) {
        ((), input)
    }

    fn absorb(
        state: &mut ExponentialBackoffState<St, A>,
        _output: EffectCompletion<Self::Effect>,
        carry: Result<A::Success, A::Error>,
    ) -> Result<Self::Output, Failure> {
        state.attempts = state.attempts.saturating_add(1);
        state.last_result = Some(carry);
        Ok(())
    }
}

pub struct SleepForBackoff<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for SleepForBackoff<St, A>
where
    A: BackoffAction,
{
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &ExponentialBackoffState<St, A>, _input: Self::Input) -> Duration {
        Duration::from_millis(state.current_delay_ms)
    }

    fn absorb(
        state: &mut ExponentialBackoffState<St, A>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        state.current_delay_ms = state.policy.next_delay_ms(state.current_delay_ms);
        Ok(())
    }
}

pub struct SkipBackoffSleep<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for SkipBackoffSleep<St, A>
where
    A: BackoffAction,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ExponentialBackoffState<St, A>, _input: Self::Input) {}

    fn absorb(
        _state: &mut ExponentialBackoffState<St, A>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

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

pub struct TakeBackoffSuccess<St, A>(PhantomData<fn() -> (St, A)>);
#[jungle::action]
impl<St, A> Action for TakeBackoffSuccess<St, A>
where
    A: BackoffAction,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = A::Success;

    fn emit(_state: &ExponentialBackoffState<St, A>, _input: Self::Input) {}

    fn absorb(
        state: &mut ExponentialBackoffState<St, A>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        match state.last_result.take() {
            Some(Ok(value)) => Ok(value),
            Some(Err(_)) => Err(Failure::from(
                "exponential backoff ended with an error result instead of success",
            )),
            None => Err(Failure::from(
                "exponential backoff is missing the terminal action result",
            )),
        }
    }
}

pub struct BackoffPending<St, A>(PhantomData<fn() -> (St, A)>);
impl<St, A> Predicate<(&ExponentialBackoffState<St, A>, &())> for BackoffPending<St, A>
where
    A: BackoffAction,
{
    fn eval((state, _): &(&ExponentialBackoffState<St, A>, &())) -> bool {
        match state.last_result.as_ref() {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(_)) => true,
        }
    }
}

pub struct BackoffShouldSleep<St, A>(PhantomData<fn() -> (St, A)>);
impl<St, A> Predicate<(ExponentialBackoffState<St, A>, ())> for BackoffShouldSleep<St, A>
where
    A: BackoffAction,
{
    fn eval((state, _): &(ExponentialBackoffState<St, A>, ())) -> bool {
        matches!(state.last_result.as_ref(), Some(Err(_)))
    }
}

#[derive(Flow)]
pub struct ExponentialBackoffBody<
    St,
    In: Clone + Serialize + DeserializeOwned,
    Ok: Serialize + DeserializeOwned,
    Err: Serialize + DeserializeOwned,
    Act: Action<Input = In, Output = Result<Ok, Err>>,
>(
    Step<CloneBackoffInput<St, Act>>,
    Scoped<St, Step<Act>>,
    Step<RecordBackoffResult<St, Act>>,
    Conditional<
        BackoffShouldSleep<St, Act>,
        Step<SleepForBackoff<St, Act>>,
        Step<SkipBackoffSleep<St, Act>>,
    >,
    Step<FlattenEither<(), ExponentialBackoffState<St, Act>>>,
);

#[derive(Flow)]
pub struct ExponentialBackoffFlow<
    St,
    In: Clone + Serialize + DeserializeOwned,
    Ok: Serialize + DeserializeOwned,
    Err: Serialize + DeserializeOwned,
    Act: Action<Input = In, Output = Result<Ok, Err>>,
>(
    Step<InitializeBackoff<St, Act>>,
    While<BackoffPending<St, Act>, ExponentialBackoffBody<St, In, Ok, Err, Act>>,
    Step<TakeBackoffSuccess<St, Act>>,
);

pub type ExponentialBackoff<St, Act> = ExponentialBackoffFlow<
    St,
    <Act as Action>::Input,
    <Act as BackoffAction>::Success,
    <Act as BackoffAction>::Error,
    Act,
>;
