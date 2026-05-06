use crate::{
    Action, ActionCompletion, ActionStep, Condition, Conditional, Creature, AspectStep, Running,
};
use inception::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

pub type DynFlow<State> = Vec<Box<dyn ErasedFlow<State>>>;
pub type ErasedStep<State> = dyn ErasedFlow<State>;

pub trait ErasedFlow<State> {
    fn progress(
        &mut self,
        state: State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(State, Value), TestExecutorError>;

    fn is_complete(&self) -> bool;
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

pub struct TypedErasedStep<Step> {
    complete: bool,
    marker: core::marker::PhantomData<fn() -> Step>,
}

impl<Step> TypedErasedStep<Step> {
    pub fn new() -> Self {
        Self {
            complete: false,
            marker: core::marker::PhantomData,
        }
    }
}

impl<T, A, Step> ErasedFlow<T::State> for TypedErasedStep<ActionStep<T, A, Step>>
where
    T: Creature,
    A: Action,
    A::Out: DeserializeOwned,
    A::Err: DeserializeOwned,
    Step: AspectStep<T, A>,
    Step::In: DeserializeOwned,
    Step::Out: Serialize,
{
    fn progress(
        &mut self,
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

        let (state, _request) = <ActionStep<T, A, Step> as Running>::run((state, typed_input));
        let (state, emitted) =
            <ActionStep<T, A, Step> as crate::Waiting>::accept((state, typed_completion));
        let emitted = serde_json::to_value(emitted)
            .map_err(|err| TestExecutorError::EmitSerialize(err.to_string()))?;
        self.complete = true;
        Ok((state, emitted))
    }

    fn is_complete(&self) -> bool {
        self.complete
    }
}

enum ActiveBranch {
    Left,
    Right,
}

struct ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned,
{
    left: DynFlow<State>,
    right: DynFlow<State>,
    choose_left: Box<dyn Fn(&(State, In)) -> bool>,
    active_branch: Option<ActiveBranch>,
    cursor: usize,
}

impl<State, In> ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned,
{
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        choose_left: Box<dyn Fn(&(State, In)) -> bool>,
    ) -> Self {
        Self {
            left,
            right,
            choose_left,
            active_branch: None,
            cursor: 0,
        }
    }

    fn branch_len(&self) -> usize {
        match self.active_branch {
            Some(ActiveBranch::Left) => self.left.len(),
            Some(ActiveBranch::Right) => self.right.len(),
            None => 0,
        }
    }
}

impl<State, In> ErasedFlow<State> for ConditionalErasedFlow<State, In>
where
    State: Clone,
    In: DeserializeOwned,
{
    fn progress(
        &mut self,
        state: State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(State, Value), TestExecutorError> {
        if self.active_branch.is_none() {
            let typed_input = serde_json::from_value::<In>(input.clone())
                .map_err(|err| TestExecutorError::InputDeserialize(err.to_string()))?;
            let choose_left = (self.choose_left)(&(state.clone(), typed_input));
            self.active_branch = Some(if choose_left {
                ActiveBranch::Left
            } else {
                ActiveBranch::Right
            });
        }

        if self.cursor >= self.branch_len() {
            return Err(TestExecutorError::Complete);
        }

        let (state, emitted) = match self.active_branch {
            Some(ActiveBranch::Left) => {
                let node = self
                    .left
                    .get_mut(self.cursor)
                    .expect("cursor was checked against left branch length");
                node.progress(state, input, completion)?
            }
            Some(ActiveBranch::Right) => {
                let node = self
                    .right
                    .get_mut(self.cursor)
                    .expect("cursor was checked against right branch length");
                node.progress(state, input, completion)?
            }
            None => unreachable!("branch was initialized above"),
        };

        self.cursor += 1;
        Ok((state, emitted))
    }

    fn is_complete(&self) -> bool {
        self.active_branch.is_some() && self.cursor >= self.branch_len()
    }
}

#[inception(property = JungleDynFlow, signature(input = Input, output = Output))]
pub trait BuildTestFlow<Input> {
    type Output;

    fn push_steps(steps: Input) -> Self::Output;

    fn nothing(steps: Input) -> Input {
        steps
    }

    fn merge<H, R>(
        _l: H,
        _r: R,
        steps: Input,
    ) -> <R as BuildTestFlow<<H as BuildTestFlow<Input>>::Output>>::Output
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
    T: Creature + 'static,
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

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, In, P, L, R> BuildTestFlow<DynFlow<State>> for Conditional<P, L, R>
where
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    P: Condition<(State, In)> + 'static,
    L: BuildTestFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
    R: BuildTestFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildTestFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildTestFlow<DynFlow<State>>>::push_steps(Vec::new());
        let choose_left = Box::new(|input: &(State, In)| <P as Condition<(State, In)>>::choose(input));
        steps.push(Box::new(ConditionalErasedFlow::<State, In>::new(
            left,
            right,
            choose_left,
        )));
        steps
    }
}

pub struct TestExecutor<A>
where
    A: Creature,
    A::Instinct: BuildTestFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    state: Option<A::State>,
    steps: DynFlow<A::State>,
    cursor: usize,
}

impl<A> TestExecutor<A>
where
    A: Creature,
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
        let node = self
            .steps
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let (state, emitted) = node.progress(state, input, completion)?;
        if node.is_complete() {
            self.cursor += 1;
        }
        self.state = Some(state);
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
