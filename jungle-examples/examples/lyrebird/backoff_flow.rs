use crate::backoff::{ExponentialBackoffInput, ExponentialBackoffPolicy, FlattenEither};
use jungle_sdk::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;

#[derive(Optic, Clone, Debug, Serialize, Deserialize)]
pub struct ExponentialBackoffFlowState<St, In, Out> {
    pub attempts: u32,
    pub current_delay_ms: u64,
    pub policy: ExponentialBackoffPolicy,
    pub flow_input: Option<In>,
    pub last_result: Option<Result<Out, Failure>>,
    #[jungle(focus)]
    pub st: St,
}

impl<St, In, Out> ExponentialBackoffFlowState<St, In, Out> {
    pub fn new(st: St) -> Self {
        Self {
            attempts: 0,
            current_delay_ms: 0,
            policy: ExponentialBackoffPolicy::default(),
            flow_input: None,
            last_result: None,
            st,
        }
    }
}

impl<St, In, Out> Default for ExponentialBackoffFlowState<St, In, Out>
where
    St: Default,
{
    fn default() -> Self {
        Self::new(St::default())
    }
}

impl<St, In, Out> ViewProject<ExponentialBackoffFlowState<St, In, Out>>
    for ExponentialBackoffFlowState<St, In, Out>
{
    fn project_view(state: &mut Self) -> &mut ExponentialBackoffFlowState<St, In, Out> {
        state
    }
}

pub struct InitializeBackoffFlow<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for InitializeBackoffFlow<St, In, Out> {
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<In>;
    type Output = ();
    type Carry = ExponentialBackoffInput<In>;

    fn emit(
        _state: &ExponentialBackoffFlowState<St, In, Out>,
        input: Self::Input,
    ) -> ((), ExponentialBackoffInput<In>) {
        ((), input)
    }

    fn absorb(
        state: &mut ExponentialBackoffFlowState<St, In, Out>,
        _output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<In>,
    ) -> Result<Self::Output, Failure> {
        state.attempts = 0;
        state.current_delay_ms = carry.policy.initial_delay_ms;
        state.policy = carry.policy;
        state.flow_input = Some(carry.action_input);
        state.last_result = None;
        Ok(())
    }
}

pub struct CloneBackoffFlowInput<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for CloneBackoffFlowInput<St, In, Out>
where
    In: Clone,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = In;
    type Carry = In;

    fn emit(state: &ExponentialBackoffFlowState<St, In, Out>, _input: Self::Input) -> ((), In) {
        (
            (),
            state
                .flow_input
                .as_ref()
                .expect("backoff flow input should be configured before retry loop starts")
                .clone(),
        )
    }

    fn absorb(
        _state: &mut ExponentialBackoffFlowState<St, In, Out>,
        _output: EffectCompletion<Self::Effect>,
        carry: In,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct RecordBackoffFlowResult<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for RecordBackoffFlowResult<St, In, Out> {
    type Effect = NoEffect;
    type Input = Result<Out, Failure>;
    type Output = ();
    type Carry = Result<Out, Failure>;

    fn emit(
        _state: &ExponentialBackoffFlowState<St, In, Out>,
        input: Self::Input,
    ) -> ((), Result<Out, Failure>) {
        ((), input)
    }

    fn absorb(
        state: &mut ExponentialBackoffFlowState<St, In, Out>,
        _output: EffectCompletion<Self::Effect>,
        carry: Result<Out, Failure>,
    ) -> Result<Self::Output, Failure> {
        state.attempts = state.attempts.saturating_add(1);
        state.last_result = Some(carry);
        Ok(())
    }
}

pub struct SleepForBackoffFlow<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for SleepForBackoffFlow<St, In, Out> {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &ExponentialBackoffFlowState<St, In, Out>, _input: Self::Input) -> Duration {
        Duration::from_millis(state.current_delay_ms)
    }

    fn absorb(
        state: &mut ExponentialBackoffFlowState<St, In, Out>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        state.current_delay_ms = state.policy.next_delay_ms(state.current_delay_ms);
        Ok(())
    }
}

pub struct SkipBackoffFlowSleep<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for SkipBackoffFlowSleep<St, In, Out> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ExponentialBackoffFlowState<St, In, Out>, _input: Self::Input) {}

    fn absorb(
        _state: &mut ExponentialBackoffFlowState<St, In, Out>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

pub struct TakeBackoffFlowSuccess<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
#[jungle::action]
impl<St, In, Out> Action for TakeBackoffFlowSuccess<St, In, Out> {
    type Effect = NoEffect;
    type Input = ();
    type Output = Out;

    fn emit(_state: &ExponentialBackoffFlowState<St, In, Out>, _input: Self::Input) {}

    fn absorb(
        state: &mut ExponentialBackoffFlowState<St, In, Out>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        match state.last_result.take() {
            Some(Ok(value)) => Ok(value),
            Some(Err(err)) => Err(Failure::from(format!(
                "exponential backoff flow ended with a failure instead of success: {err}"
            ))),
            None => Err(Failure::from(
                "exponential backoff flow is missing the terminal subflow result",
            )),
        }
    }
}

pub struct BackoffFlowPending<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
impl<St, In, Out> Predicate<(&ExponentialBackoffFlowState<St, In, Out>, &())>
    for BackoffFlowPending<St, In, Out>
{
    fn eval((state, _): &(&ExponentialBackoffFlowState<St, In, Out>, &())) -> bool {
        match state.last_result.as_ref() {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(_)) => true,
        }
    }
}

pub struct BackoffFlowShouldSleep<St, In, Out>(PhantomData<fn() -> (St, In, Out)>);
impl<St, In, Out> Predicate<(ExponentialBackoffFlowState<St, In, Out>, ())>
    for BackoffFlowShouldSleep<St, In, Out>
{
    fn eval((state, _): &(ExponentialBackoffFlowState<St, In, Out>, ())) -> bool {
        matches!(state.last_result.as_ref(), Some(Err(_)))
    }
}

#[derive(Flow)]
pub struct ExponentialBackoffFlowBody<
    St,
    In: Clone + Serialize + DeserializeOwned,
    Out: Serialize + DeserializeOwned,
    F: TraverseFlow,
>(
    Step<CloneBackoffFlowInput<St, In, Out>>,
    Attempt<F>,
    Step<RecordBackoffFlowResult<St, In, Out>>,
    Conditional<
        FocusedCondition<
            BackoffFlowShouldSleep<St, In, Out>,
            ExponentialBackoffFlowState<St, In, Out>,
        >,
        Step<SleepForBackoffFlow<St, In, Out>>,
        Step<SkipBackoffFlowSleep<St, In, Out>>,
    >,
    Step<FlattenEither<(), ExponentialBackoffFlowState<St, In, Out>>>,
);

#[derive(Flow)]
#[jungle(focus = ExponentialBackoffFlowState<St, In, Out>)]
pub struct ExponentialBackoffFlow<
    St,
    In: Clone + Serialize + DeserializeOwned,
    Out: Serialize + DeserializeOwned,
    F: TraverseFlow,
>(
    Step<InitializeBackoffFlow<St, In, Out>>,
    While<
        FocusedLoopCondition<
            BackoffFlowPending<St, In, Out>,
            ExponentialBackoffFlowState<St, In, Out>,
        >,
        ExponentialBackoffFlowBody<St, In, Out, F>,
    >,
    Step<TakeBackoffFlowSuccess<St, In, Out>>,
);
