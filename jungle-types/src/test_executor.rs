use crate::{
    Action, ActionCompletion, ActionInputMapper, ActionOutputMapper, ActionRequest, ActionStep,
    Animal, Awaiting, Yielding,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

pub trait ErasedStep<State> {
    fn progress(
        &self,
        state: State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(State, Value), TestExecutorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TestExecutorError {
    #[error("executor is already complete")]
    Complete,
    #[error("input deserialization failed: {0}")]
    InputDeserialize(String),
    #[error("output deserialization failed: {0}")]
    OutputDeserialize(String),
    #[error("error deserialization failed: {0}")]
    ErrorDeserialize(String),
    #[error("emit serialization failed: {0}")]
    EmitSerialize(String),
    #[error("not enough completions to advance to end")]
    NotEnoughCompletions,
}

pub trait TestFlow {
    type State;

    fn build_steps() -> Vec<Box<dyn ErasedStep<Self::State>>>;
}

pub struct TypedErasedStep<Step>(core::marker::PhantomData<fn() -> Step>);

impl<Step> TypedErasedStep<Step> {
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T, A, Prepare, Apply> ErasedStep<T::State>
    for TypedErasedStep<ActionStep<T, A, Prepare, Apply>>
where
    T: Animal,
    A: Action,
    A::Out: DeserializeOwned,
    A::Err: DeserializeOwned,
    Prepare: ActionInputMapper<T, A>,
    Prepare::In: DeserializeOwned,
    Apply: ActionOutputMapper<T, A>,
    Apply::Out: Serialize,
{
    fn progress(
        &self,
        state: T::State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(T::State, Value), TestExecutorError> {
        let typed_input = serde_json::from_value::<Prepare::In>(input)
            .map_err(|err| TestExecutorError::InputDeserialize(err.to_string()))?;

        let typed_completion: ActionCompletion<A> = match completion {
            Ok(output) => Ok(serde_json::from_value::<A::Out>(output)
                .map_err(|err| TestExecutorError::OutputDeserialize(err.to_string()))?),
            Err(error) => Err(serde_json::from_value::<A::Err>(error)
                .map_err(|err| TestExecutorError::ErrorDeserialize(err.to_string()))?),
        };

        let (state, request) =
            <ActionStep<T, A, Prepare, Apply> as Yielding>::run((state, typed_input));
        let _prepared: ActionRequest<A> = request;
        let (state, emitted) =
            <ActionStep<T, A, Prepare, Apply> as Awaiting>::accept((state, typed_completion));
        let emitted = serde_json::to_value(emitted)
            .map_err(|err| TestExecutorError::EmitSerialize(err.to_string()))?;
        Ok((state, emitted))
    }
}

pub struct TestExecutor<A>
where
    A: Animal,
    A::Instinct: TestFlow<State = A::State>,
{
    state: Option<A::State>,
    steps: Vec<Box<dyn ErasedStep<A::State>>>,
    cursor: usize,
}

impl<A> TestExecutor<A>
where
    A: Animal,
    A::Instinct: TestFlow<State = A::State>,
{
    pub fn new(state: A::State) -> Self {
        Self {
            state: Some(state),
            steps: <A::Instinct as TestFlow>::build_steps(),
            cursor: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.cursor >= self.steps.len()
    }

    pub fn next(
        &mut self,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<Value, TestExecutorError> {
        if self.is_complete() {
            return Err(TestExecutorError::Complete);
        }

        let state = self.state.take().expect("executor state is always present");
        let step = &self.steps[self.cursor];
        let (state, emitted) = step.progress(state, input, completion)?;
        self.state = Some(state);
        self.cursor += 1;
        Ok(emitted)
    }

    pub fn advance_to_end(
        &mut self,
        inputs: impl IntoIterator<Item = (Value, Result<Value, Value>)>,
    ) -> Result<Vec<Value>, TestExecutorError> {
        let mut completions = inputs.into_iter();
        let mut emitted = Vec::new();
        while !self.is_complete() {
            let (input, completion) = completions
                .next()
                .ok_or(TestExecutorError::NotEnoughCompletions)?;
            emitted.push(self.next(input, completion)?);
        }
        Ok(emitted)
    }

    pub fn into_state(self) -> A::State {
        self.state.expect("executor state is always present")
    }
}
