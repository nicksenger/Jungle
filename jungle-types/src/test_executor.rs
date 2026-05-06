use crate::{
    Action, ActionCompletion, ActionStep, AspectStep, Condition, Conditional, Creature,
    LoopCondition, Running, While,
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

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, bool), TestExecutorError> {
        Ok((state, false))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TestExecutorError {
    #[error("executor is already complete")]
    Complete,
    #[error("input serialization failed: {0}")]
    InputSerialize(String),
    #[error("input deserialization failed: {0}")]
    InputDeserialize(String),
    #[error("output serialization failed: {0}")]
    OutputSerialize(String),
    #[error("output deserialization failed: {0}")]
    OutputDeserialize(String),
    #[error("error serialization failed: {0}")]
    ErrorSerialize(String),
    #[error("error deserialization failed: {0}")]
    ErrorDeserialize(String),
    #[error("emit serialization failed: {0}")]
    EmitSerialize(String),
    #[error("emit deserialization failed: {0}")]
    EmitDeserialize(String),
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

struct WhileErasedFlow<State> {
    should_continue: Box<dyn Fn(&State) -> bool>,
    build_body: Box<dyn Fn() -> DynFlow<State>>,
    active_body: DynFlow<State>,
    body_cursor: usize,
    complete: bool,
}

impl<State> WhileErasedFlow<State> {
    fn new(
        should_continue: Box<dyn Fn(&State) -> bool>,
        build_body: Box<dyn Fn() -> DynFlow<State>>,
    ) -> Self {
        Self {
            should_continue,
            build_body,
            active_body: Vec::new(),
            body_cursor: 0,
            complete: false,
        }
    }

    fn ensure_iteration_ready(&mut self) {
        if self.active_body.is_empty() {
            self.active_body = (self.build_body)();
            self.body_cursor = 0;
        }
    }
}

impl<State> ErasedFlow<State> for WhileErasedFlow<State> {
    fn progress(
        &mut self,
        state: State,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<(State, Value), TestExecutorError> {
        if self.complete {
            return Err(TestExecutorError::Complete);
        }

        if !(self.should_continue)(&state) {
            self.complete = true;
            return Err(TestExecutorError::Complete);
        }

        self.ensure_iteration_ready();

        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        let (state, emitted) = node.progress(state, input, completion)?;
        if node.is_complete() {
            self.body_cursor += 1;
            if self.body_cursor >= self.active_body.len() {
                self.active_body.clear();
                self.body_cursor = 0;
            }
        }
        Ok((state, emitted))
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, bool), TestExecutorError> {
        if self.complete {
            return Ok((state, true));
        }
        if !(self.should_continue)(&state) {
            self.complete = true;
            return Ok((state, true));
        }
        Ok((state, false))
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
        let choose_left =
            Box::new(|input: &(State, In)| <P as Condition<(State, In)>>::choose(input));
        steps.push(Box::new(ConditionalErasedFlow::<State, In>::new(
            left,
            right,
            choose_left,
        )));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, C, F> BuildTestFlow<DynFlow<State>> for While<C, F>
where
    State: 'static,
    C: LoopCondition<State> + 'static,
    F: BuildTestFlow<DynFlow<State>, Output = DynFlow<State>> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let should_continue =
            Box::new(|state: &State| <C as LoopCondition<State>>::should_continue(state));
        let build_body = Box::new(|| <F as BuildTestFlow<DynFlow<State>>>::push_steps(Vec::new()));
        steps.push(Box::new(WhileErasedFlow::new(should_continue, build_body)));
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
    fn settle_without_progress(&mut self) -> Result<(), TestExecutorError> {
        loop {
            if self.cursor >= self.steps.len() {
                break;
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            let (state, completed) = node.try_complete_without_progress(state)?;
            self.state = Some(state);
            if completed {
                self.cursor += 1;
                continue;
            }
            break;
        }
        Ok(())
    }

    pub fn new(state: A::State) -> Self {
        let mut executor = Self {
            state: Some(state),
            steps: <A::Instinct as BuildTestFlow<DynFlow<A::State>>>::push_steps(Vec::new()),
            cursor: 0,
        };
        executor
            .settle_without_progress()
            .expect("initial settle should not fail");
        executor
    }

    pub fn is_complete(&self) -> bool {
        self.cursor >= self.steps.len()
    }

    pub fn next<Emitted>(
        &mut self,
        input: Value,
        completion: Result<Value, Value>,
    ) -> Result<Emitted, TestExecutorError>
    where
        Emitted: DeserializeOwned,
    {
        self.settle_without_progress()?;
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
        self.settle_without_progress()?;
        serde_json::from_value(emitted)
            .map_err(|err| TestExecutorError::EmitDeserialize(err.to_string()))
    }

    pub fn next_typed<In, Out, Err, Emitted>(
        &mut self,
        input: In,
        completion: Result<Out, Err>,
    ) -> Result<Emitted, TestExecutorError>
    where
        In: Serialize,
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned,
    {
        let input = serde_json::to_value(input)
            .map_err(|err| TestExecutorError::InputSerialize(err.to_string()))?;
        let completion = match completion {
            Ok(output) => Ok(serde_json::to_value(output)
                .map_err(|err| TestExecutorError::OutputSerialize(err.to_string()))?),
            Err(error) => Err(serde_json::to_value(error)
                .map_err(|err| TestExecutorError::ErrorSerialize(err.to_string()))?),
        };

        self.next(input, completion)
    }

    pub fn advance_to_end<Emitted>(
        &mut self,
        inputs: impl IntoIterator<Item = (Value, Result<Value, Value>)>,
    ) -> Result<Vec<Emitted>, TestExecutorError>
    where
        Emitted: DeserializeOwned,
    {
        let mut completions = inputs.into_iter();
        let mut emitted = Vec::new();
        self.settle_without_progress()?;
        while !self.is_complete() {
            let (input, completion) = completions
                .next()
                .ok_or(TestExecutorError::NotEnoughCompletions)?;
            emitted.push(self.next(input, completion)?);
        }
        Ok(emitted)
    }

    pub fn advance_to_end_typed<In, Out, Err, Emitted>(
        &mut self,
        inputs: impl IntoIterator<Item = (In, Result<Out, Err>)>,
    ) -> Result<Vec<Emitted>, TestExecutorError>
    where
        In: Serialize,
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned,
    {
        let inputs = inputs
            .into_iter()
            .map(|(input, completion)| {
                let input = serde_json::to_value(input)
                    .map_err(|err| TestExecutorError::InputSerialize(err.to_string()))?;
                let completion = match completion {
                    Ok(output) => Ok(serde_json::to_value(output)
                        .map_err(|err| TestExecutorError::OutputSerialize(err.to_string()))?),
                    Err(error) => Err(serde_json::to_value(error)
                        .map_err(|err| TestExecutorError::ErrorSerialize(err.to_string()))?),
                };
                Ok((input, completion))
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.advance_to_end(inputs)
    }

    pub fn into_state(self) -> A::State {
        self.state.expect("executor state is always present")
    }
}
