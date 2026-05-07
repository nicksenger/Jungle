use crate::{
    Action, ActionCompletion, Impulse, Condition, Conditional, Creature, Task,
    LoopCondition, Running, While,
};
use inception::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

type Serialized = Vec<u8>;
type SerializedCompletion = Result<Serialized, Serialized>;
type ActionFuture = Pin<Box<dyn Future<Output = Result<SerializedCompletion, ExecutorError>>>>;
type ActionRunner = Box<dyn FnOnce() -> ActionFuture>;

pub type DynFlow<State> = Vec<Box<dyn ErasedFlow<State>>>;
pub type ErasedStep<State> = dyn ErasedFlow<State>;

pub struct ExecutableActionRequest {
    request: Serialized,
    runner: ActionRunner,
}

impl ExecutableActionRequest {
    fn new(request: Serialized, runner: ActionRunner) -> Self {
        Self { request, runner }
    }

    pub fn deserialize_request<Request>(&self) -> Result<Request, ExecutorError>
    where
        Request: DeserializeOwned,
    {
        deserialize_request(self.request.clone())
    }

    pub fn run(self) -> impl Future<Output = Result<SerializedCompletion, ExecutorError>> {
        (self.runner)()
    }
}

pub trait ErasedFlow<State> {
    fn request(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, Serialized), ExecutorError>;

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError>;

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, ExecutableActionRequest), ExecutorError>;

    fn is_waiting_completion(&self) -> bool;

    fn is_complete(&self) -> bool;

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, bool), ExecutorError> {
        Ok((state, false))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("executor is already complete")]
    Complete,
    #[error("step is awaiting completion")]
    AwaitingCompletion,
    #[error("step has no pending request")]
    NoPendingRequest,
    #[error("input serialization failed: {0}")]
    InputSerialize(String),
    #[error("input deserialization failed: {0}")]
    InputDeserialize(String),
    #[error("request serialization failed: {0}")]
    RequestSerialize(String),
    #[error("request deserialization failed: {0}")]
    RequestDeserialize(String),
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

pub trait ExecutorFlow {
    type State;
    fn build_steps() -> DynFlow<Self::State>;
}

pub struct TypedErasedStep<Step> {
    complete: bool,
    waiting_completion: bool,
    marker: core::marker::PhantomData<fn() -> Step>,
}

impl<Step> TypedErasedStep<Step> {
    pub fn new() -> Self {
        Self {
            complete: false,
            waiting_completion: false,
            marker: core::marker::PhantomData,
        }
    }
}

impl<T, Step> ErasedFlow<T::State> for TypedErasedStep<Impulse<T, Step>>
where
    T: Creature,
    Step: Task<T>,
    <Step as Task<T>>::Action: Action<Dependency = ()>,
    <<Step as Task<T>>::Action as Action>::Dependency: 'static,
    <<Step as Task<T>>::Action as Action>::In: 'static,
    <<Step as Task<T>>::Action as Action>::Out: 'static,
    <<Step as Task<T>>::Action as Action>::Err: Serialize + 'static,
    <<Step as Task<T>>::Action as Action>::Out: DeserializeOwned,
    <<Step as Task<T>>::Action as Action>::Err: DeserializeOwned,
    Step::In: DeserializeOwned,
    Step::Out: Serialize,
{
    fn request(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> Result<(T::State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if self.waiting_completion {
            return Err(ExecutorError::AwaitingCompletion);
        }

        let typed_input = postcard::from_bytes::<Step::In>(&input)
            .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
        let (state, request) = <Impulse<T, Step> as Running>::run((state, typed_input));
        let request = postcard::to_allocvec(&request.into_input())
            .map_err(|err| ExecutorError::RequestSerialize(err.to_string()))?;
        self.waiting_completion = true;
        Ok((state, request))
    }

    fn request_executable(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> Result<(T::State, ExecutableActionRequest), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if self.waiting_completion {
            return Err(ExecutorError::AwaitingCompletion);
        }

        let typed_input = postcard::from_bytes::<Step::In>(&input)
            .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
        let (state, request) = <Impulse<T, Step> as Running>::run((state, typed_input));
        let action_input = request.into_input();
        let request = postcard::to_allocvec(&action_input)
            .map_err(|err| ExecutorError::RequestSerialize(err.to_string()))?;
        let runner: ActionRunner = Box::new(move || {
            Box::pin(async move {
                let completion = <<Step as Task<T>>::Action as Action>::act(&(), action_input).await;
                serialize_completion(completion)
            })
        });

        self.waiting_completion = true;
        Ok((state, ExecutableActionRequest::new(request, runner)))
    }

    fn complete(
        &mut self,
        state: T::State,
        completion: SerializedCompletion,
    ) -> Result<(T::State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }

        let typed_completion: ActionCompletion<<Step as Task<T>>::Action> = match completion {
            Ok(output) => Ok(postcard::from_bytes::<<<Step as Task<T>>::Action as Action>::Out>(&output)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?),
            Err(error) => Err(postcard::from_bytes::<<<Step as Task<T>>::Action as Action>::Err>(&error)
                .map_err(|err| ExecutorError::ErrorDeserialize(err.to_string()))?),
        };

        let (state, emitted) = <Impulse<T, Step> as crate::Waiting>::accept((state, typed_completion));
        let emitted = postcard::to_allocvec(&emitted)
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))?;
        self.waiting_completion = false;
        self.complete = true;
        Ok((state, emitted))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
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

    fn active_branch_mut(&mut self) -> Option<&mut DynFlow<State>> {
        match self.active_branch {
            Some(ActiveBranch::Left) => Some(&mut self.left),
            Some(ActiveBranch::Right) => Some(&mut self.right),
            None => None,
        }
    }

    fn active_node_mut(&mut self) -> Option<&mut Box<dyn ErasedFlow<State>>> {
        let cursor = self.cursor;
        self.active_branch_mut()?.get_mut(cursor)
    }
}

impl<State, In> ErasedFlow<State> for ConditionalErasedFlow<State, In>
where
    State: Clone,
    In: DeserializeOwned,
{
    fn request(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.active_branch.is_none() {
            let typed_input = postcard::from_bytes::<In>(&input)
                .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
            let choose_left = (self.choose_left)(&(state.clone(), typed_input));
            self.active_branch = Some(if choose_left {
                ActiveBranch::Left
            } else {
                ActiveBranch::Right
            });
        }

        if self.cursor >= self.branch_len() {
            return Err(ExecutorError::Complete);
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request(state, input)
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.cursor >= self.branch_len() {
            return Err(ExecutorError::Complete);
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        let (state, emitted) = node.complete(state, completion)?;
        if node.is_complete() {
            self.cursor += 1;
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, ExecutableActionRequest), ExecutorError> {
        if self.active_branch.is_none() {
            let typed_input = postcard::from_bytes::<In>(&input)
                .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
            let choose_left = (self.choose_left)(&(state.clone(), typed_input));
            self.active_branch = Some(if choose_left {
                ActiveBranch::Left
            } else {
                ActiveBranch::Right
            });
        }

        if self.cursor >= self.branch_len() {
            return Err(ExecutorError::Complete);
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request_executable(state, input)
    }

    fn is_waiting_completion(&self) -> bool {
        if self.cursor >= self.branch_len() {
            return false;
        }

        match self.active_branch {
            Some(ActiveBranch::Left) => self
                .left
                .get(self.cursor)
                .expect("cursor was checked against left branch length")
                .is_waiting_completion(),
            Some(ActiveBranch::Right) => self
                .right
                .get(self.cursor)
                .expect("cursor was checked against right branch length")
                .is_waiting_completion(),
            None => false,
        }
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
    fn request(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }

        if !(self.should_continue)(&state) {
            self.complete = true;
            return Err(ExecutorError::Complete);
        }

        self.ensure_iteration_ready();

        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        node.request(state, input)
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }

        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        let (state, emitted) = node.complete(state, completion)?;
        if node.is_complete() {
            self.body_cursor += 1;
            if self.body_cursor >= self.active_body.len() {
                self.active_body.clear();
                self.body_cursor = 0;
            }
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> Result<(State, ExecutableActionRequest), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }

        if !(self.should_continue)(&state) {
            self.complete = true;
            return Err(ExecutorError::Complete);
        }

        self.ensure_iteration_ready();

        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        node.request_executable(state, input)
    }

    fn is_waiting_completion(&self) -> bool {
        self.active_body
            .get(self.body_cursor)
            .map(|node| node.is_waiting_completion())
            .unwrap_or(false)
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, bool), ExecutorError> {
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
pub trait BuildFlow<Input> {
    type Output;

    fn push_steps(steps: Input) -> Self::Output;

    fn nothing(steps: Input) -> Input {
        steps
    }

    fn merge<H, R>(
        _l: H,
        _r: R,
        steps: Input,
    ) -> <R as BuildFlow<<H as BuildFlow<Input>>::Output>>::Output
    where
        H: BuildFlow<Input>,
        R: BuildFlow<<H as BuildFlow<Input>>::Output>,
    {
        let steps = <H as BuildFlow<_>>::push_steps(steps);
        <R as BuildFlow<_>>::push_steps(steps)
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, steps: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        steps
    }

    fn join<F>(_fields: F, steps: Input) -> <F as BuildFlow<Input>>::Output
    where
        F: BuildFlow<Input>,
    {
        <F as BuildFlow<_>>::push_steps(steps)
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<T, Step> BuildFlow<DynFlow<T::State>> for Impulse<T, Step>
where
    T: Creature + 'static,
    Step: Task<T> + 'static,
    <Step as Task<T>>::Action: Action<Dependency = ()> + 'static,
    <<Step as Task<T>>::Action as Action>::Err: Serialize,
    <<Step as Task<T>>::Action as Action>::Out: DeserializeOwned,
    <<Step as Task<T>>::Action as Action>::Err: DeserializeOwned,
    Step::In: DeserializeOwned,
    Step::Out: Serialize,
{
    type Output = DynFlow<T::State>;

    fn push_steps(mut steps: DynFlow<T::State>) -> Self::Output {
        steps.push(Box::new(TypedErasedStep::<Impulse<T, Step>>::new()));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, In, P, L, R> BuildFlow<DynFlow<State>> for Conditional<P, L, R>
where
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    P: Condition<(State, In)> + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
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
impl<State, C, F> BuildFlow<DynFlow<State>> for While<C, F>
where
    State: 'static,
    C: LoopCondition<State> + 'static,
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let should_continue =
            Box::new(|state: &State| <C as LoopCondition<State>>::should_continue(state));
        let build_body = Box::new(|| <F as BuildFlow<DynFlow<State>>>::push_steps(Vec::new()));
        steps.push(Box::new(WhileErasedFlow::new(should_continue, build_body)));
        steps
    }
}

fn serialize_input<In>(input: In) -> Result<Serialized, ExecutorError>
where
    In: Serialize,
{
    postcard::to_allocvec(&input).map_err(|err| ExecutorError::InputSerialize(err.to_string()))
}

fn serialize_completion<Out, Err>(
    completion: Result<Out, Err>,
) -> Result<SerializedCompletion, ExecutorError>
where
    Out: Serialize,
    Err: Serialize,
{
    match completion {
        Ok(output) => Ok(Ok(postcard::to_allocvec(&output)
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?)),
        Err(error) => Ok(Err(postcard::to_allocvec(&error)
            .map_err(|err| ExecutorError::ErrorSerialize(err.to_string()))?)),
    }
}

fn deserialize_request<Request>(request: Serialized) -> Result<Request, ExecutorError>
where
    Request: DeserializeOwned,
{
    postcard::from_bytes(&request).map_err(|err| ExecutorError::RequestDeserialize(err.to_string()))
}

fn deserialize_emitted<Emitted>(emitted: Serialized) -> Result<Emitted, ExecutorError>
where
    Emitted: DeserializeOwned,
{
    postcard::from_bytes(&emitted).map_err(|err| ExecutorError::EmitDeserialize(err.to_string()))
}

pub struct ManualExecutor<A>
where
    A: Creature,
    A::Instinct: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    state: Option<A::State>,
    steps: DynFlow<A::State>,
    cursor: usize,
}

impl<A> ManualExecutor<A>
where
    A: Creature,
    A::Instinct: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    fn settle_without_progress(&mut self) -> Result<(), ExecutorError> {
        loop {
            if self.cursor >= self.steps.len() {
                break;
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            if node.is_waiting_completion() {
                self.state = Some(state);
                break;
            }
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
            steps: <A::Instinct as BuildFlow<DynFlow<A::State>>>::push_steps(Vec::new()),
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

    pub fn next_request(&mut self, input: Serialized) -> Result<Serialized, ExecutorError> {
        self.settle_without_progress()?;
        if self.is_complete() {
            return Err(ExecutorError::Complete);
        }

        let state = self.state.take().expect("executor state is always present");
        let node = self
            .steps
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let (state, request) = node.request(state, input)?;
        self.state = Some(state);
        Ok(request)
    }

    pub fn next_executable_request(
        &mut self,
        input: Serialized,
    ) -> Result<ExecutableActionRequest, ExecutorError> {
        self.settle_without_progress()?;
        if self.is_complete() {
            return Err(ExecutorError::Complete);
        }

        let state = self.state.take().expect("executor state is always present");
        let node = self
            .steps
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let (state, request) = node.request_executable(state, input)?;
        self.state = Some(state);
        Ok(request)
    }

    pub fn next_request_typed<In, Request>(&mut self, input: In) -> Result<Request, ExecutorError>
    where
        In: Serialize,
        Request: DeserializeOwned,
    {
        let input = serialize_input(input)?;
        let request = self.next_request(input)?;
        deserialize_request(request)
    }

    pub fn complete<Emitted>(
        &mut self,
        completion: SerializedCompletion,
    ) -> Result<Emitted, ExecutorError>
    where
        Emitted: DeserializeOwned,
    {
        self.settle_without_progress()?;
        if self.is_complete() {
            return Err(ExecutorError::Complete);
        }

        let state = self.state.take().expect("executor state is always present");
        let node = self
            .steps
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let (state, emitted) = node.complete(state, completion)?;
        if node.is_complete() {
            self.cursor += 1;
        }
        self.state = Some(state);
        self.settle_without_progress()?;
        deserialize_emitted(emitted)
    }

    pub fn complete_serialized(
        &mut self,
        completion: SerializedCompletion,
    ) -> Result<Serialized, ExecutorError> {
        self.settle_without_progress()?;
        if self.is_complete() {
            return Err(ExecutorError::Complete);
        }

        let state = self.state.take().expect("executor state is always present");
        let node = self
            .steps
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let (state, emitted) = node.complete(state, completion)?;
        if node.is_complete() {
            self.cursor += 1;
        }
        self.state = Some(state);
        self.settle_without_progress()?;
        Ok(emitted)
    }

    pub fn complete_typed<Out, Err, Emitted>(
        &mut self,
        completion: Result<Out, Err>,
    ) -> Result<Emitted, ExecutorError>
    where
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned,
    {
        let completion = serialize_completion(completion)?;
        let emitted = self.complete_serialized(completion)?;
        deserialize_emitted(emitted)
    }

    pub fn next<Emitted>(
        &mut self,
        input: Serialized,
        completion: SerializedCompletion,
    ) -> Result<Emitted, ExecutorError>
    where
        Emitted: DeserializeOwned,
    {
        self.next_request(input)?;
        self.complete(completion)
    }

    pub fn next_typed<In, Out, Err, Emitted>(
        &mut self,
        input: In,
        completion: Result<Out, Err>,
    ) -> Result<Emitted, ExecutorError>
    where
        In: Serialize,
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned,
    {
        let input = serialize_input(input)?;
        let completion = serialize_completion(completion)?;
        self.next(input, completion)
    }

    pub fn advance_to_end<Emitted>(
        &mut self,
        inputs: impl IntoIterator<Item = (Serialized, SerializedCompletion)>,
    ) -> Result<Vec<Emitted>, ExecutorError>
    where
        Emitted: DeserializeOwned,
    {
        let mut completions = inputs.into_iter();
        let mut emitted = Vec::new();
        self.settle_without_progress()?;
        while !self.is_complete() {
            let (input, completion) = completions
                .next()
                .ok_or(ExecutorError::NotEnoughCompletions)?;
            emitted.push(self.next(input, completion)?);
        }
        Ok(emitted)
    }

    pub fn advance_to_end_typed<In, Out, Err, Emitted>(
        &mut self,
        inputs: impl IntoIterator<Item = (In, Result<Out, Err>)>,
    ) -> Result<Vec<Emitted>, ExecutorError>
    where
        In: Serialize,
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned,
    {
        let inputs = inputs
            .into_iter()
            .map(|(input, completion)| {
                let input = serialize_input(input)?;
                let completion = serialize_completion(completion)?;
                Ok((input, completion))
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.advance_to_end(inputs)
    }

    pub fn into_state(self) -> A::State {
        self.state.expect("executor state is always present")
    }
}

pub struct Executor<A>
where
    A: Creature,
    A::Instinct: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    manual: ManualExecutor<A>,
    last_emitted: Option<Serialized>,
}

impl<A> Executor<A>
where
    A: Creature,
    A::Instinct: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    pub fn new(state: A::State) -> Self {
        Self {
            manual: ManualExecutor::new(state),
            last_emitted: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.manual.is_complete()
    }

    pub fn next_request<Request>(&mut self) -> Result<Request, ExecutorError>
    where
        Request: DeserializeOwned + Default + Serialize,
    {
        let input = self.last_emitted.take().unwrap_or_else(|| {
            serialize_input(Request::default()).expect("default input serializes")
        });
        let request = self.manual.next_request(input)?;
        deserialize_request(request)
    }

    pub fn next_executable_request<Initial>(
        &mut self,
        initial_input: Initial,
    ) -> Result<ExecutableActionRequest, ExecutorError>
    where
        Initial: Serialize,
    {
        let input = match self.last_emitted.take() {
            Some(input) => input,
            None => serialize_input(initial_input)?,
        };
        self.manual.next_executable_request(input)
    }

    pub fn complete<Out, Err, Emitted>(
        &mut self,
        completion: Result<Out, Err>,
    ) -> Result<Emitted, ExecutorError>
    where
        Out: Serialize,
        Err: Serialize,
        Emitted: DeserializeOwned + Serialize,
    {
        let completion = serialize_completion(completion)?;
        let emitted = self.manual.complete_serialized(completion)?;
        self.last_emitted = Some(emitted.clone());
        let emitted: Emitted = deserialize_emitted(emitted)?;
        Ok(emitted)
    }

    pub fn complete_serialized(
        &mut self,
        completion: SerializedCompletion,
    ) -> Result<Serialized, ExecutorError> {
        let emitted = self.manual.complete_serialized(completion)?;
        self.last_emitted = Some(emitted.clone());
        Ok(emitted)
    }

    pub async fn next_and_complete_with(
        &mut self,
        initial_input: impl Serialize,
    ) -> Result<Serialized, ExecutorError> {
        let request = self.next_executable_request(initial_input)?;
        let completion = request.run().await?;
        self.complete_serialized(completion)
    }

    pub async fn advance_to_end_with<Initial>(
        &mut self,
        initial_input: Initial,
    ) -> Result<Vec<Serialized>, ExecutorError>
    where
        Initial: Serialize + Clone,
    {
        let mut emitted = Vec::new();
        while !self.is_complete() {
            emitted.push(self.next_and_complete_with(initial_input.clone()).await?);
        }
        Ok(emitted)
    }

    pub fn advance_to_end<Out, Err, Request, Emitted>(
        &mut self,
        completions: impl IntoIterator<Item = Result<Out, Err>>,
    ) -> Result<Vec<Emitted>, ExecutorError>
    where
        Out: Serialize,
        Err: Serialize,
        Request: DeserializeOwned + Default + Serialize,
        Emitted: DeserializeOwned + Serialize,
    {
        let mut completions = completions.into_iter();
        let mut emitted = Vec::new();

        while !self.is_complete() {
            let _request: Request = self.next_request()?;
            let completion = completions
                .next()
                .ok_or(ExecutorError::NotEnoughCompletions)?;
            emitted.push(self.complete(completion)?);
        }

        Ok(emitted)
    }

    pub fn into_state(self) -> A::State {
        self.manual.into_state()
    }
}
