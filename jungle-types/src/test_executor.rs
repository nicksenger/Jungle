use crate::{
    Action, ActionCompletion, ActionRequest, ActionStep, Animal, AspectStep, Waiting, Running,
};
use inception::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

pub type DynFlow<State> = Vec<Box<dyn ErasedStep<State>>>;

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
    fn build_steps() -> DynFlow<Self::State>;
}

pub struct TypedErasedStep<Step>(core::marker::PhantomData<fn() -> Step>);

impl<Step> TypedErasedStep<Step> {
    pub fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T, A, Step> ErasedStep<T::State> for TypedErasedStep<ActionStep<T, A, Step>>
where
    T: Animal,
    A: Action,
    A::Out: DeserializeOwned,
    A::Err: DeserializeOwned,
    Step: AspectStep<T, A>,
    Step::In: DeserializeOwned,
    Step::Out: Serialize,
{
    fn progress(
        &self,
        state: T::State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(T::State, Value), TestExecutorError> {
        let typed_input = serde_json::from_value::<Step::In>(input)
            .map_err(|err| TestExecutorError::InputDeserialize(err.to_string()))?;

        let typed_completion: ActionCompletion<A> = match completion {
            Ok(output) => Ok(serde_json::from_value::<A::Out>(output)
                .map_err(|err| TestExecutorError::OutputDeserialize(err.to_string()))?),
            Err(error) => Err(serde_json::from_value::<A::Err>(error)
                .map_err(|err| TestExecutorError::ErrorDeserialize(err.to_string()))?),
        };

        let (state, request) = <ActionStep<T, A, Step> as Running>::run((state, typed_input));
        let _prepared: ActionRequest<A> = request;
        let (state, emitted) =
            <ActionStep<T, A, Step> as Waiting>::accept((state, typed_completion));
        let emitted = serde_json::to_value(emitted)
            .map_err(|err| TestExecutorError::EmitSerialize(err.to_string()))?;
        Ok((state, emitted))
    }
}

#[inception(property = JungleDynFlow, signature(input = Input, output = Output))]
pub trait BuildTestFlow<Input> {
    type Output;

    fn push_steps(steps: Input) -> Self::Output;

    fn nothing(steps: Input) -> Input {
        steps
    }

    fn merge<H, R>(_l: H, _r: R, steps: Input) -> <R as BuildTestFlow<<H as BuildTestFlow<Input>>::Output>>::Output
    where
        H: BuildTestFlow<Input>,
        R: BuildTestFlow<<H as BuildTestFlow<Input>>::Output>,
    {
        let steps = <H as BuildTestFlow<_>>::push_steps(steps);
        <R as BuildTestFlow<_>>::push_steps(steps)
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, steps: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        steps
    }

    fn join<F>(_fields: F, steps: Input) -> <F as BuildTestFlow<Input>>::Output
    where
        F: BuildTestFlow<Input>,
    {
        <F as BuildTestFlow<_>>::push_steps(steps)
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<T, A, Step> BuildTestFlow<DynFlow<T::State>> for ActionStep<T, A, Step>
where
    T: Animal + 'static,
    A: Action + 'static,
    A::Out: DeserializeOwned,
    A::Err: DeserializeOwned,
    Step: AspectStep<T, A> + 'static,
    Step::In: DeserializeOwned,
    Step::Out: Serialize,
{
    type Output = DynFlow<T::State>;

    fn push_steps(mut steps: DynFlow<T::State>) -> Self::Output {
        steps.push(Box::new(TypedErasedStep::<ActionStep<T, A, Step>>::new()));
        steps
    }
}

pub struct TestExecutor<A>
where
    A: Animal,
    A::Instinct: BuildTestFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    state: Option<A::State>,
    steps: Vec<Box<dyn ErasedStep<A::State>>>,
    cursor: usize,
}

impl<A> TestExecutor<A>
where
    A: Animal,
    A::Instinct: BuildTestFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    pub fn new(state: A::State) -> Self {
        Self {
            state: Some(state),
            steps: <A::Instinct as BuildTestFlow<DynFlow<A::State>>>::push_steps(Vec::new()),
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
