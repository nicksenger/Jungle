use crate::{
    Effect, EffectCompletion, Animal, BackendError, Conditional, Join, LoopCondition, Act,
    Running, Select, Step, Transparent, While,
};
use inception::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde::{Deserialize, Serialize as SerdeSerialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type Serialized = Vec<u8>;
type SerializedCompletion = Result<Serialized, Serialized>;
type EffectFuture =
    Pin<Box<dyn Future<Output = Result<SerializedCompletion, ExecutorError>> + Send>>;
type EffectRunner = Box<dyn FnOnce() -> EffectFuture + Send>;
type RequestError<State> = (State, ExecutorError);
type RequestResult<State, Request> = Result<(State, Request), RequestError<State>>;

trait SplitStateCarry<State> {
    type Carry;
}

impl<State, Carry> SplitStateCarry<State> for (State, Carry) {
    type Carry = Carry;
}

trait ArgputForState<State> {
    type Carry;
}

impl<State, T, A> ArgputForState<State> for Step<T, A>
where
    T: Animal<State = State>,
    A: Act<T>,
{
    type Carry = A::Arg;
}

impl<State, P, L, R, M> ArgputForState<State> for Conditional<P, L, R, M>
where
    L: ArgputForState<State>,
    R: ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
{
    type Carry = <L as ArgputForState<State>>::Carry;
}

impl<State, M, F> ArgputForState<State> for Transparent<M, F>
where
    F: ArgputForState<State>,
{
    type Carry = <F as ArgputForState<State>>::Carry;
}

impl<State, L, R, M> ArgputForState<State> for Select<L, R, M>
where
    L: ArgputForState<State>,
    R: ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
{
    type Carry = <L as ArgputForState<State>>::Carry;
}

impl<State, L, R, M> ArgputForState<State> for Join<L, R, M>
where
    L: ArgputForState<State>,
    R: ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
{
    type Carry = <L as ArgputForState<State>>::Carry;
}

impl<State, C, F, M> ArgputForState<State> for While<C, F, M>
where
    C: LoopCondition<State>,
{
    type Carry = <C as LoopCondition<State>>::Arg;
}

impl<State, F> ArgputForState<State> for F
where
    (): crate::__inception_running::FieldsInput<F>,
    <() as crate::__inception_running::FieldsInput<F>>::In: SplitStateCarry<State>,
{
    type Carry =
        <<() as crate::__inception_running::FieldsInput<F>>::In as SplitStateCarry<State>>::Carry;
}

fn decode_controlled_input<In, F>(input: &[u8], fallback: F) -> Result<(bool, In), ExecutorError>
where
    In: DeserializeOwned + Serialize,
    F: FnOnce(&In) -> bool,
{
    if let Ok((should, carry)) = postcard::from_bytes::<(bool, In)>(input) {
        let reencoded = postcard::to_allocvec(&(should, &carry))
            .map_err(|err| ExecutorError::InputSerialize(err.to_string()))?;
        if reencoded.as_slice() == input {
            return Ok((should, carry));
        }
    }

    let carry = postcard::from_bytes::<In>(input)
        .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
    Ok((fallback(&carry), carry))
}

pub type DynFlow<State> = Vec<Box<dyn ErasedFlow<State> + Send>>;
pub type ErasedStep<State> = dyn ErasedFlow<State> + Send;

pub struct ExecutableEffectRequest {
    node_id: u32,
    effect_type: &'static str,
    request: Serialized,
    runner: EffectRunner,
}

impl ExecutableEffectRequest {
    fn new(
        node_id: u32,
        effect_type: &'static str,
        request: Serialized,
        runner: EffectRunner,
    ) -> Self {
        Self {
            node_id,
            effect_type,
            request,
            runner,
        }
    }

    pub fn node_id(&self) -> u32 {
        self.node_id
    }

    pub fn effect_type(&self) -> &'static str {
        self.effect_type
    }

    pub fn request_bytes(&self) -> &[u8] {
        &self.request
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

pub trait ErasedFlow<State>: Send {
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized>;

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError>;

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest>;

    fn is_waiting_completion(&self) -> bool;

    fn is_complete(&self) -> bool;

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, bool), ExecutorError> {
        Ok((state, false))
    }

    fn assign_node_ids(&mut self, _next_id: &mut u32) {}
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
    #[error("client transport channel closed")]
    ClientTransportClosed,
    #[error("client transport acknowledgement dropped")]
    ClientTransportAckDropped,
    #[error("client transport failed: {0}")]
    ClientTransport(String),
    #[error("backend error: {0}")]
    Backend(#[source] BackendError),
    #[error("not enough completions to advance to end")]
    NotEnoughCompletions,
}

pub trait ExecutorFlow {
    type State;
    fn build_steps() -> DynFlow<Self::State>;
}

pub struct TypedErasedStep<Step> {
    node_id: u32,
    complete: bool,
    waiting_completion: bool,
    marker: core::marker::PhantomData<fn() -> Step>,
}

impl<Step> TypedErasedStep<Step> {
    pub fn new() -> Self {
        Self {
            node_id: 0,
            complete: false,
            waiting_completion: false,
            marker: core::marker::PhantomData,
        }
    }
}

impl<T, A> ErasedFlow<T::State> for TypedErasedStep<Step<T, A>>
where
    T: Animal,
    A: Act<T>,
    <A as Act<T>>::Effect: Effect<Dependency = ()>,
    <<A as Act<T>>::Effect as Effect>::Dependency: 'static,
    <<A as Act<T>>::Effect as Effect>::In: 'static,
    <<A as Act<T>>::Effect as Effect>::Out: 'static,
    <<A as Act<T>>::Effect as Effect>::Err: Serialize + 'static,
    <<A as Act<T>>::Effect as Effect>::Out: DeserializeOwned,
    <<A as Act<T>>::Effect as Effect>::Err: DeserializeOwned,
    A::Arg: DeserializeOwned,
    A::Ret: Serialize,
{
    fn request(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> RequestResult<T::State, Serialized> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let typed_input = match postcard::from_bytes::<A::Arg>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, ExecutorError::InputDeserialize(err.to_string()))),
        };
        let (state, request) = <Step<T, A> as Running>::run((state, typed_input));
        let request = match postcard::to_allocvec(&request.into_input()) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        self.waiting_completion = true;
        Ok((state, request))
    }

    fn request_executable(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> RequestResult<T::State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let typed_input = match postcard::from_bytes::<A::Arg>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, ExecutorError::InputDeserialize(err.to_string()))),
        };
        let (state, request) = <Step<T, A> as Running>::run((state, typed_input));
        let effect_input = request.into_input();
        let request = match postcard::to_allocvec(&effect_input) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let completion = <<A as Act<T>>::Effect as Effect>::act(&(), effect_input).await;
                serialize_completion(completion)
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(
                self.node_id,
                core::any::type_name::<<A as Act<T>>::Effect>(),
                request,
                runner,
            ),
        ))
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

        let typed_completion: EffectCompletion<<A as Act<T>>::Effect> = match completion {
            Ok(output) => Ok(
                postcard::from_bytes::<<<A as Act<T>>::Effect as Effect>::Out>(&output)
                    .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            ),
            Err(error) => Err(
                postcard::from_bytes::<<<A as Act<T>>::Effect as Effect>::Err>(&error)
                    .map_err(|err| ExecutorError::ErrorDeserialize(err.to_string()))?,
            ),
        };

        let (state, emitted) = <Step<T, A> as crate::Waiting>::accept((state, typed_completion));
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

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
    }
}

pub struct ContextualTypedErasedStep<Context, R> {
    context: Arc<Context>,
    node_id: u32,
    complete: bool,
    waiting_completion: bool,
    marker: core::marker::PhantomData<fn() -> R>,
}

impl<Context, R> ContextualTypedErasedStep<Context, R> {
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            context,
            node_id: 0,
            complete: false,
            waiting_completion: false,
            marker: core::marker::PhantomData,
        }
    }
}

impl<Context, T, A> ErasedFlow<T::State> for ContextualTypedErasedStep<Context, Step<T, A>>
where
    Context: Send + Sync + 'static,
    T: Animal,
    A: Act<T>,
    <A as Act<T>>::Effect: Effect,
    for<'ctx> &'ctx Context: Into<<<A as Act<T>>::Effect as Effect>::Dependency>,
    <<A as Act<T>>::Effect as Effect>::Dependency: 'static,
    <<A as Act<T>>::Effect as Effect>::In: 'static,
    <<A as Act<T>>::Effect as Effect>::Out: 'static,
    <<A as Act<T>>::Effect as Effect>::Err: Serialize + 'static,
    <<A as Act<T>>::Effect as Effect>::Out: DeserializeOwned,
    <<A as Act<T>>::Effect as Effect>::Err: DeserializeOwned,
    A::Arg: DeserializeOwned,
    A::Ret: Serialize,
{
    fn request(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> RequestResult<T::State, Serialized> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let typed_input = match postcard::from_bytes::<A::Arg>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, ExecutorError::InputDeserialize(err.to_string()))),
        };
        let (state, request) = <Step<T, A> as Running>::run((state, typed_input));
        let request = match postcard::to_allocvec(&request.into_input()) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        self.waiting_completion = true;
        Ok((state, request))
    }

    fn request_executable(
        &mut self,
        state: T::State,
        input: Serialized,
    ) -> RequestResult<T::State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let typed_input = match postcard::from_bytes::<A::Arg>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, ExecutorError::InputDeserialize(err.to_string()))),
        };
        let (state, request) = <Step<T, A> as Running>::run((state, typed_input));
        let effect_input = request.into_input();
        let request = match postcard::to_allocvec(&effect_input) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let dependency: <<A as Act<T>>::Effect as Effect>::Dependency =
            self.context.as_ref().into();
        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let completion =
                    <<A as Act<T>>::Effect as Effect>::act(&dependency, effect_input).await;
                serialize_completion(completion)
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(
                self.node_id,
                core::any::type_name::<<A as Act<T>>::Effect>(),
                request,
                runner,
            ),
        ))
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

        let typed_completion: EffectCompletion<<A as Act<T>>::Effect> = match completion {
            Ok(output) => Ok(
                postcard::from_bytes::<<<A as Act<T>>::Effect as Effect>::Out>(&output)
                    .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            ),
            Err(error) => Err(
                postcard::from_bytes::<<<A as Act<T>>::Effect as Effect>::Err>(&error)
                    .map_err(|err| ExecutorError::ErrorDeserialize(err.to_string()))?,
            ),
        };

        let (state, emitted) = <Step<T, A> as crate::Waiting>::accept((state, typed_completion));
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

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
    }
}

enum ActiveBranch {
    Left,
    Right,
}

struct ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    left: DynFlow<State>,
    right: DynFlow<State>,
    choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
    active_branch: Option<ActiveBranch>,
    cursor: usize,
}

impl<State, In> ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
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

    fn active_node_mut(&mut self) -> Option<&mut Box<dyn ErasedFlow<State> + Send>> {
        let cursor = self.cursor;
        self.active_branch_mut()?.get_mut(cursor)
    }
}

impl<State, In> ErasedFlow<State> for ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let (choose_left, branch_input) = match decode_controlled_input::<In, _>(&input, |carry| {
            (self.choose_left)(&state, carry)
        }) {
            Ok(pair) => pair,
            Err(err) => return Err((state, err)),
        };
        if self.active_branch.is_none() {
            self.active_branch = Some(if choose_left {
                ActiveBranch::Left
            } else {
                ActiveBranch::Right
            });
        }
        let branch_input = match postcard::to_allocvec(&branch_input) {
            Ok(branch_input) => branch_input,
            Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
        };

        if self.cursor >= self.branch_len() {
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request(state, branch_input)
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
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let (choose_left, branch_input) = match decode_controlled_input::<In, _>(&input, |carry| {
            (self.choose_left)(&state, carry)
        }) {
            Ok(pair) => pair,
            Err(err) => return Err((state, err)),
        };
        if self.active_branch.is_none() {
            self.active_branch = Some(if choose_left {
                ActiveBranch::Left
            } else {
                ActiveBranch::Right
            });
        }
        let branch_input = match postcard::to_allocvec(&branch_input) {
            Ok(branch_input) => branch_input,
            Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
        };

        if self.cursor >= self.branch_len() {
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request_executable(state, branch_input)
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

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

struct WhileErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
    build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    active_body: DynFlow<State>,
    body_node_id_start: u32,
    body_cursor: usize,
    complete: bool,
    deferred_state: Option<State>,
    marker: core::marker::PhantomData<fn() -> In>,
}

#[derive(Debug, Clone, Deserialize, SerdeSerialize)]
enum SelectCompletionEnvelope {
    Left(SerializedCompletion),
    Right(SerializedCompletion),
}

#[derive(Debug, Clone, Deserialize, SerdeSerialize)]
struct SelectRequestEnvelope {
    left: Serialized,
    right: Serialized,
}

struct SelectErasedFlow<State> {
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> SelectErasedFlow<State> {
    fn new(left: DynFlow<State>, right: DynFlow<State>) -> Self {
        Self {
            node_id: 0,
            left,
            right,
            waiting_completion: false,
            complete: false,
        }
    }
}

struct SelectContextErasedFlow<State> {
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> SelectContextErasedFlow<State> {
    fn new(left: DynFlow<State>, right: DynFlow<State>) -> Self {
        Self {
            node_id: 0,
            left,
            right,
            waiting_completion: false,
            complete: false,
        }
    }
}

impl<State> ErasedFlow<State> for SelectErasedFlow<State>
where
    State: Clone + 'static,
{
    fn request(&mut self, state: State, _input: Serialized) -> RequestResult<State, Serialized> {
        Err((
            state,
            ExecutorError::ClientTransport("Select requires executable request mode".to_string()),
        ))
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }

        let envelope: SelectCompletionEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "select completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let emitted = match envelope {
            SelectCompletionEnvelope::Left(left_completion) => {
                let (left_state, left_emitted) = self
                    .left
                    .get_mut(0)
                    .ok_or(ExecutorError::Complete)?
                    .complete(state, left_completion)?;
                let mut serialized = Vec::with_capacity(1 + left_emitted.len());
                serialized.push(0);
                serialized.extend_from_slice(&left_emitted);
                (left_state, serialized)
            }
            SelectCompletionEnvelope::Right(right_completion) => {
                let (right_state, right_emitted) = self
                    .right
                    .get_mut(0)
                    .ok_or(ExecutorError::Complete)?
                    .complete(state, right_completion)?;
                let mut serialized = Vec::with_capacity(1 + right_emitted.len());
                serialized.push(1);
                serialized.extend_from_slice(&right_emitted);
                (right_state, serialized)
            }
        };

        self.waiting_completion = false;
        self.complete = true;
        Ok(emitted)
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let left_node = match self.left.get_mut(0) {
            Some(left_node) => left_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_left_state, left_req) = left_node.request_executable(state.clone(), input.clone())?;
        let right_node = match self.right.get_mut(0) {
            Some(right_node) => right_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_right_state, right_req) = right_node.request_executable(state.clone(), input)?;

        let payload = SelectRequestEnvelope {
            left: left_req.request_bytes().to_vec(),
            right: right_req.request_bytes().to_vec(),
        };
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let left = left_req.run();
                let right = right_req.run();
                let selected = futures::future::select(
                    Box::pin(left) as Pin<Box<_>>,
                    Box::pin(right) as Pin<Box<_>>,
                )
                .await;
                let envelope = match selected {
                    futures::future::Either::Left((left_completion, _)) => {
                        SelectCompletionEnvelope::Left(left_completion?)
                    }
                    futures::future::Either::Right((right_completion, _)) => {
                        SelectCompletionEnvelope::Right(right_completion?)
                    }
                };
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Select", request, runner),
        ))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

impl<State> ErasedFlow<State> for SelectContextErasedFlow<State>
where
    State: Clone + 'static,
{
    fn request(&mut self, state: State, _input: Serialized) -> RequestResult<State, Serialized> {
        Err((
            state,
            ExecutorError::ClientTransport("Select requires executable request mode".to_string()),
        ))
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }

        let envelope: SelectCompletionEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "select completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let emitted = match envelope {
            SelectCompletionEnvelope::Left(left_completion) => {
                let (left_state, left_emitted) = self
                    .left
                    .get_mut(0)
                    .ok_or(ExecutorError::Complete)?
                    .complete(state, left_completion)?;
                let mut serialized = Vec::with_capacity(1 + left_emitted.len());
                serialized.push(0);
                serialized.extend_from_slice(&left_emitted);
                (left_state, serialized)
            }
            SelectCompletionEnvelope::Right(right_completion) => {
                let (right_state, right_emitted) = self
                    .right
                    .get_mut(0)
                    .ok_or(ExecutorError::Complete)?
                    .complete(state, right_completion)?;
                let mut serialized = Vec::with_capacity(1 + right_emitted.len());
                serialized.push(1);
                serialized.extend_from_slice(&right_emitted);
                (right_state, serialized)
            }
        };

        self.waiting_completion = false;
        self.complete = true;
        Ok(emitted)
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let left_node = match self.left.get_mut(0) {
            Some(left_node) => left_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_left_state, left_req) = left_node.request_executable(state.clone(), input.clone())?;
        let right_node = match self.right.get_mut(0) {
            Some(right_node) => right_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_right_state, right_req) = right_node.request_executable(state.clone(), input)?;

        let payload = SelectRequestEnvelope {
            left: left_req.request_bytes().to_vec(),
            right: right_req.request_bytes().to_vec(),
        };
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let left = left_req.run();
                let right = right_req.run();
                let selected = futures::future::select(
                    Box::pin(left) as Pin<Box<_>>,
                    Box::pin(right) as Pin<Box<_>>,
                )
                .await;
                let envelope = match selected {
                    futures::future::Either::Left((left_completion, _)) => {
                        SelectCompletionEnvelope::Left(left_completion?)
                    }
                    futures::future::Either::Right((right_completion, _)) => {
                        SelectCompletionEnvelope::Right(right_completion?)
                    }
                };
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Select", request, runner),
        ))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

#[derive(Debug, Clone, Deserialize, SerdeSerialize)]
struct JoinCompletionEnvelope {
    left: SerializedCompletion,
    right: SerializedCompletion,
}

#[derive(Debug, Clone, Deserialize, SerdeSerialize)]
struct JoinRequestEnvelope {
    left: Serialized,
    right: Serialized,
}

struct JoinErasedFlow<State> {
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> JoinErasedFlow<State> {
    fn new(left: DynFlow<State>, right: DynFlow<State>) -> Self {
        Self {
            node_id: 0,
            left,
            right,
            waiting_completion: false,
            complete: false,
        }
    }
}

struct JoinContextErasedFlow<State> {
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> JoinContextErasedFlow<State> {
    fn new(left: DynFlow<State>, right: DynFlow<State>) -> Self {
        Self {
            node_id: 0,
            left,
            right,
            waiting_completion: false,
            complete: false,
        }
    }
}

impl<State> ErasedFlow<State> for JoinErasedFlow<State>
where
    State: Clone + 'static,
{
    fn request(&mut self, state: State, _input: Serialized) -> RequestResult<State, Serialized> {
        Err((
            state,
            ExecutorError::ClientTransport("Join requires executable request mode".to_string()),
        ))
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }

        let envelope: JoinCompletionEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "join completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let left_node = self.left.get_mut(0).ok_or(ExecutorError::Complete)?;
        let (left_state, left_emitted) = left_node.complete(state, envelope.left)?;
        let right_node = self.right.get_mut(0).ok_or(ExecutorError::Complete)?;
        let (right_state, right_emitted) = right_node.complete(left_state, envelope.right)?;
        let mut emitted = Vec::with_capacity(left_emitted.len() + right_emitted.len());
        emitted.extend_from_slice(&left_emitted);
        emitted.extend_from_slice(&right_emitted);

        self.waiting_completion = false;
        self.complete = true;
        Ok((right_state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let left_node = match self.left.get_mut(0) {
            Some(left_node) => left_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_left_state, left_req) = left_node.request_executable(state.clone(), input.clone())?;
        let right_node = match self.right.get_mut(0) {
            Some(right_node) => right_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_right_state, right_req) = right_node.request_executable(state.clone(), input)?;

        let payload = JoinRequestEnvelope {
            left: left_req.request_bytes().to_vec(),
            right: right_req.request_bytes().to_vec(),
        };
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let (left_completion, right_completion) =
                    futures::join!(left_req.run(), right_req.run());
                let envelope = JoinCompletionEnvelope {
                    left: left_completion?,
                    right: right_completion?,
                };
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Join", request, runner),
        ))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

impl<State> ErasedFlow<State> for JoinContextErasedFlow<State>
where
    State: Clone + 'static,
{
    fn request(&mut self, state: State, _input: Serialized) -> RequestResult<State, Serialized> {
        Err((
            state,
            ExecutorError::ClientTransport("Join requires executable request mode".to_string()),
        ))
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }

        let envelope: JoinCompletionEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "join completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let left_node = self.left.get_mut(0).ok_or(ExecutorError::Complete)?;
        let (left_state, left_emitted) = left_node.complete(state, envelope.left)?;
        let right_node = self.right.get_mut(0).ok_or(ExecutorError::Complete)?;
        let (right_state, right_emitted) = right_node.complete(left_state, envelope.right)?;
        let mut emitted = Vec::with_capacity(left_emitted.len() + right_emitted.len());
        emitted.extend_from_slice(&left_emitted);
        emitted.extend_from_slice(&right_emitted);

        self.waiting_completion = false;
        self.complete = true;
        Ok((right_state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        let left_node = match self.left.get_mut(0) {
            Some(left_node) => left_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_left_state, left_req) = left_node.request_executable(state.clone(), input.clone())?;
        let right_node = match self.right.get_mut(0) {
            Some(right_node) => right_node,
            None => return Err((state, ExecutorError::Complete)),
        };
        let (_right_state, right_req) = right_node.request_executable(state.clone(), input)?;

        let payload = JoinRequestEnvelope {
            left: left_req.request_bytes().to_vec(),
            right: right_req.request_bytes().to_vec(),
        };
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let (left_completion, right_completion) =
                    futures::join!(left_req.run(), right_req.run());
                let envelope = JoinCompletionEnvelope {
                    left: left_completion?,
                    right: right_completion?,
                };
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Join", request, runner),
        ))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        *next_id = next_id.saturating_add(1);
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

impl<State, In> WhileErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn new(
        should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
        build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    ) -> Self {
        Self {
            should_continue,
            build_body,
            active_body: Vec::new(),
            body_node_id_start: 0,
            body_cursor: 0,
            complete: false,
            deferred_state: None,
            marker: core::marker::PhantomData,
        }
    }

    fn ensure_iteration_ready(&mut self) {
        if self.active_body.is_empty() {
            self.active_body = (self.build_body)();
            let mut next_id = self.body_node_id_start;
            for node in &mut self.active_body {
                node.assign_node_ids(&mut next_id);
            }
            self.body_cursor = 0;
        }
    }
}

impl<State, In> ErasedFlow<State> for WhileErasedFlow<State, In>
where
    State: Send,
    In: DeserializeOwned + Serialize,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let input = input;
        let mut state = state;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            let (should_continue, branch_input) =
                match decode_controlled_input::<In, _>(&input, |carry| {
                    (self.should_continue)(&state, carry)
                }) {
                    Ok(pair) => pair,
                    Err(err) => return Err((state, err)),
                };
            if !should_continue {
                self.complete = true;
                self.deferred_state = Some(state);
                let state = self
                    .deferred_state
                    .take()
                    .expect("deferred state was just set");
                return Err((state, ExecutorError::Complete));
            }
            let branch_input = match postcard::to_allocvec(&branch_input) {
                Ok(branch_input) => branch_input,
                Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
            };

            self.ensure_iteration_ready();

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            match node.request(state, branch_input) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    if node.is_complete() {
                        self.body_cursor += 1;
                        if self.body_cursor >= self.active_body.len() {
                            self.active_body.clear();
                            self.body_cursor = 0;
                        }
                        state = next_state;
                        continue;
                    }
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, err)) => return Err((next_state, err)),
            }
        }
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
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let input = input;
        let mut state = state;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            let (should_continue, branch_input) =
                match decode_controlled_input::<In, _>(&input, |carry| {
                    (self.should_continue)(&state, carry)
                }) {
                    Ok(pair) => pair,
                    Err(err) => return Err((state, err)),
                };
            if !should_continue {
                self.complete = true;
                self.deferred_state = Some(state);
                let state = self
                    .deferred_state
                    .take()
                    .expect("deferred state was just set");
                return Err((state, ExecutorError::Complete));
            }
            let branch_input = match postcard::to_allocvec(&branch_input) {
                Ok(branch_input) => branch_input,
                Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
            };

            self.ensure_iteration_ready();

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            match node.request_executable(state, branch_input) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    if node.is_complete() {
                        self.body_cursor += 1;
                        if self.body_cursor >= self.active_body.len() {
                            self.active_body.clear();
                            self.body_cursor = 0;
                        }
                        state = next_state;
                        continue;
                    }
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, err)) => return Err((next_state, err)),
            }
        }
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
        if let Some(saved) = self.deferred_state.take() {
            return Ok((saved, true));
        }
        if self.complete {
            return Ok((state, true));
        }
        Ok((state, false))
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.body_node_id_start = *next_id;
        let mut template = (self.build_body)();
        for node in &mut template {
            node.assign_node_ids(next_id);
        }
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
impl<T, A> BuildFlow<DynFlow<T::State>> for Step<T, A>
where
    T: Animal + 'static,
    A: Act<T> + 'static,
    <A as Act<T>>::Effect: Effect<Dependency = ()> + 'static,
    <<A as Act<T>>::Effect as Effect>::Err: Serialize,
    <<A as Act<T>>::Effect as Effect>::Out: DeserializeOwned,
    <<A as Act<T>>::Effect as Effect>::Err: DeserializeOwned,
    A::Arg: DeserializeOwned,
    A::Ret: Serialize,
{
    type Output = DynFlow<T::State>;

    fn push_steps(mut steps: DynFlow<T::State>) -> Self::Output {
        steps.push(Box::new(TypedErasedStep::<Step<T, A>>::new()));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, P, L, R, M> BuildFlow<DynFlow<State>> for Conditional<P, L, R, M>
where
    State: Clone + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + ArgputForState<State>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>>
        + ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
    <L as ArgputForState<State>>::Carry: Clone + DeserializeOwned + Serialize + 'static,
    P: crate::Condition<(State, <L as ArgputForState<State>>::Carry)> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let choose_left = Box::new(
            |state: &State, input: &<L as ArgputForState<State>>::Carry| {
                <P as crate::Condition<(State, <L as ArgputForState<State>>::Carry)>>::choose(&(
                    state.clone(),
                    input.clone(),
                ))
            },
        );
        steps.push(Box::new(ConditionalErasedFlow::<
            State,
            <L as ArgputForState<State>>::Carry,
        >::new(left, right, choose_left)));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, In, C, F, M> BuildFlow<DynFlow<State>> for While<C, F, M>
where
    State: Send + 'static,
    C: LoopCondition<State, Arg = In> + 'static,
    In: DeserializeOwned + Serialize + 'static,
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let _marker = core::marker::PhantomData::<(C, In)>;
        let should_continue = Box::new(|state: &State, _input: &In| {
            <C as LoopCondition<State>>::should_continue(state)
        });
        let build_body = Box::new(|| <F as BuildFlow<DynFlow<State>>>::push_steps(Vec::new()));
        steps.push(Box::new(WhileErasedFlow::<State, In>::new(
            should_continue,
            build_body,
        )));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, M, F> BuildFlow<DynFlow<State>> for Transparent<M, F>
where
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
{
    type Output = DynFlow<State>;

    fn push_steps(steps: DynFlow<State>) -> Self::Output {
        <F as BuildFlow<DynFlow<State>>>::push_steps(steps)
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, In, L, R, M> BuildFlow<DynFlow<State>> for Select<L, R, M>
where
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        steps.push(Box::new(SelectErasedFlow::<State>::new(left, right)));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, In, L, R, M> BuildFlow<DynFlow<State>> for Join<L, R, M>
where
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + Running<In = (State, In)>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        steps.push(Box::new(JoinErasedFlow::<State>::new(left, right)));
        steps
    }
}

enum ActiveContextBranch {
    Left,
    Right,
}

#[inception(property = JungleDynFlowContext, signature(input = Input, output = Output))]
pub trait BuildFlowWithContext<Input> {
    type Output;

    fn push_steps(input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H, R>(
        _l: H,
        _r: R,
        input: Input,
    ) -> <R as BuildFlowWithContext<<H as BuildFlowWithContext<Input>>::Output>>::Output
    where
        H: BuildFlowWithContext<Input>,
        R: BuildFlowWithContext<<H as BuildFlowWithContext<Input>>::Output>,
    {
        let input = <H as BuildFlowWithContext<_>>::push_steps(input);
        <R as BuildFlowWithContext<_>>::push_steps(input)
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, input: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(_fields: F, input: Input) -> <F as BuildFlowWithContext<Input>>::Output
    where
        F: BuildFlowWithContext<Input>,
    {
        <F as BuildFlowWithContext<_>>::push_steps(input)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, T, A> BuildFlowWithContext<(Arc<Context>, DynFlow<T::State>)> for Step<T, A>
where
    Context: Send + Sync + 'static,
    T: Animal + 'static,
    A: Act<T> + 'static,
    <A as Act<T>>::Effect: Effect + 'static,
    for<'ctx> &'ctx Context: Into<<<A as Act<T>>::Effect as Effect>::Dependency>,
    <<A as Act<T>>::Effect as Effect>::Err: Serialize,
    <<A as Act<T>>::Effect as Effect>::Out: DeserializeOwned,
    <<A as Act<T>>::Effect as Effect>::Err: DeserializeOwned,
    A::Arg: DeserializeOwned,
    A::Ret: Serialize,
{
    type Output = (Arc<Context>, DynFlow<T::State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<T::State>)) -> Self::Output {
        steps.push(Box::new(
            ContextualTypedErasedStep::<Context, Step<T, A>>::new(Arc::clone(&context)),
        ));
        (context, steps)
    }
}

struct ConditionalContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    left: DynFlow<State>,
    right: DynFlow<State>,
    choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
    active_branch: Option<ActiveContextBranch>,
    cursor: usize,
}

impl<State, In> ConditionalContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
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
            Some(ActiveContextBranch::Left) => self.left.len(),
            Some(ActiveContextBranch::Right) => self.right.len(),
            None => 0,
        }
    }

    fn active_branch_mut(&mut self) -> Option<&mut DynFlow<State>> {
        match self.active_branch {
            Some(ActiveContextBranch::Left) => Some(&mut self.left),
            Some(ActiveContextBranch::Right) => Some(&mut self.right),
            None => None,
        }
    }

    fn active_node_mut(&mut self) -> Option<&mut Box<dyn ErasedFlow<State> + Send>> {
        let cursor = self.cursor;
        self.active_branch_mut()?.get_mut(cursor)
    }
}

impl<State, In> ErasedFlow<State> for ConditionalContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let (choose_left, branch_input) = match decode_controlled_input::<In, _>(&input, |carry| {
            (self.choose_left)(&state, carry)
        }) {
            Ok(pair) => pair,
            Err(err) => return Err((state, err)),
        };
        if self.active_branch.is_none() {
            self.active_branch = Some(if choose_left {
                ActiveContextBranch::Left
            } else {
                ActiveContextBranch::Right
            });
        }
        let branch_input = match postcard::to_allocvec(&branch_input) {
            Ok(branch_input) => branch_input,
            Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
        };

        if self.cursor >= self.branch_len() {
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request(state, branch_input)
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
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let (choose_left, branch_input) = match decode_controlled_input::<In, _>(&input, |carry| {
            (self.choose_left)(&state, carry)
        }) {
            Ok(pair) => pair,
            Err(err) => return Err((state, err)),
        };
        if self.active_branch.is_none() {
            self.active_branch = Some(if choose_left {
                ActiveContextBranch::Left
            } else {
                ActiveContextBranch::Right
            });
        }
        let branch_input = match postcard::to_allocvec(&branch_input) {
            Ok(branch_input) => branch_input,
            Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
        };

        if self.cursor >= self.branch_len() {
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.request_executable(state, branch_input)
    }

    fn is_waiting_completion(&self) -> bool {
        if self.cursor >= self.branch_len() {
            return false;
        }

        match self.active_branch {
            Some(ActiveContextBranch::Left) => self
                .left
                .get(self.cursor)
                .expect("cursor was checked against left branch length")
                .is_waiting_completion(),
            Some(ActiveContextBranch::Right) => self
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

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, P, L, R, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for Conditional<P, L, R, M>
where
    Context: 'static,
    State: Clone + 'static,
    L: ArgputForState<State>,
    <L as ArgputForState<State>>::Carry: Clone + DeserializeOwned + Serialize + 'static,
    P: crate::Condition<(State, <L as ArgputForState<State>>::Carry)> + 'static,
    L: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    R: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let (_, left) = <L as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        let (_, right) = <R as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        let choose_left = Box::new(
            |state: &State, input: &<L as ArgputForState<State>>::Carry| {
                <P as crate::Condition<(State, <L as ArgputForState<State>>::Carry)>>::choose(&(
                    state.clone(),
                    input.clone(),
                ))
            },
        );
        steps.push(Box::new(ConditionalContextErasedFlow::<
            State,
            <L as ArgputForState<State>>::Carry,
        >::new(left, right, choose_left)));
        (context, steps)
    }
}

struct WhileContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
    build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    active_body: DynFlow<State>,
    body_node_id_start: u32,
    body_cursor: usize,
    complete: bool,
    deferred_state: Option<State>,
    marker: core::marker::PhantomData<fn() -> In>,
}

impl<State, In> WhileContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn new(
        should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
        build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    ) -> Self {
        Self {
            should_continue,
            build_body,
            active_body: Vec::new(),
            body_node_id_start: 0,
            body_cursor: 0,
            complete: false,
            deferred_state: None,
            marker: core::marker::PhantomData,
        }
    }

    fn ensure_iteration_ready(&mut self) {
        if self.active_body.is_empty() {
            self.active_body = (self.build_body)();
            let mut next_id = self.body_node_id_start;
            for node in &mut self.active_body {
                node.assign_node_ids(&mut next_id);
            }
            self.body_cursor = 0;
        }
    }
}

impl<State, In> ErasedFlow<State> for WhileContextErasedFlow<State, In>
where
    State: Send,
    In: DeserializeOwned + Serialize,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let input = input;
        let mut state = state;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            let (should_continue, branch_input) =
                match decode_controlled_input::<In, _>(&input, |carry| {
                    (self.should_continue)(&state, carry)
                }) {
                    Ok(pair) => pair,
                    Err(err) => return Err((state, err)),
                };
            if !should_continue {
                self.complete = true;
                self.deferred_state = Some(state);
                let state = self
                    .deferred_state
                    .take()
                    .expect("deferred state was just set");
                return Err((state, ExecutorError::Complete));
            }
            let branch_input = match postcard::to_allocvec(&branch_input) {
                Ok(branch_input) => branch_input,
                Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
            };

            self.ensure_iteration_ready();

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            match node.request(state, branch_input) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    if node.is_complete() {
                        self.body_cursor += 1;
                        if self.body_cursor >= self.active_body.len() {
                            self.active_body.clear();
                            self.body_cursor = 0;
                        }
                        state = next_state;
                        continue;
                    }
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, err)) => return Err((next_state, err)),
            }
        }
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
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let input = input;
        let mut state = state;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            let (should_continue, branch_input) =
                match decode_controlled_input::<In, _>(&input, |carry| {
                    (self.should_continue)(&state, carry)
                }) {
                    Ok(pair) => pair,
                    Err(err) => return Err((state, err)),
                };
            if !should_continue {
                self.complete = true;
                self.deferred_state = Some(state);
                let state = self
                    .deferred_state
                    .take()
                    .expect("deferred state was just set");
                return Err((state, ExecutorError::Complete));
            }
            let branch_input = match postcard::to_allocvec(&branch_input) {
                Ok(branch_input) => branch_input,
                Err(err) => return Err((state, ExecutorError::InputSerialize(err.to_string()))),
            };

            self.ensure_iteration_ready();

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            match node.request_executable(state, branch_input) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    if node.is_complete() {
                        self.body_cursor += 1;
                        if self.body_cursor >= self.active_body.len() {
                            self.active_body.clear();
                            self.body_cursor = 0;
                        }
                        state = next_state;
                        continue;
                    }
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, err)) => return Err((next_state, err)),
            }
        }
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
        if let Some(saved) = self.deferred_state.take() {
            return Ok((saved, true));
        }
        if self.complete {
            return Ok((state, true));
        }
        Ok((state, false))
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.body_node_id_start = *next_id;
        let mut template = (self.build_body)();
        for node in &mut template {
            node.assign_node_ids(next_id);
        }
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, In, C, F, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for While<C, F, M>
where
    Context: Send + Sync + 'static,
    State: Send + 'static,
    C: LoopCondition<State, Arg = In> + 'static,
    In: DeserializeOwned + Serialize + 'static,
    F: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + 'static,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let _marker = core::marker::PhantomData::<(C, In)>;
        let should_continue = Box::new(|state: &State, _input: &In| {
            <C as LoopCondition<State>>::should_continue(state)
        });
        let context_for_body = Arc::clone(&context);
        let build_body = Box::new(move || {
            let (_, flow) = <F as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(
                (Arc::clone(&context_for_body), Vec::new()),
            );
            flow
        });
        steps.push(Box::new(WhileContextErasedFlow::<State, In>::new(
            should_continue,
            build_body,
        )));
        (context, steps)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, M, F> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for Transparent<M, F>
where
    F: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps(input: (Arc<Context>, DynFlow<State>)) -> Self::Output {
        <F as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, In, L, R, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for Select<L, R, M>
where
    Context: 'static,
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    L: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + Running<In = (State, In)>,
    R: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + Running<In = (State, In)>,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let (_, left) = <L as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        let (_, right) = <R as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        steps.push(Box::new(SelectContextErasedFlow::<State>::new(left, right)));
        (context, steps)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, In, L, R, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for Join<L, R, M>
where
    Context: 'static,
    State: Clone + 'static,
    In: DeserializeOwned + 'static,
    L: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + Running<In = (State, In)>,
    R: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + Running<In = (State, In)>,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let (_, left) = <L as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        let (_, right) = <R as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        steps.push(Box::new(JoinContextErasedFlow::<State>::new(left, right)));
        (context, steps)
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

fn assign_flow_node_ids<State>(steps: &mut DynFlow<State>) {
    let mut next_id = 0_u32;
    for node in steps {
        node.assign_node_ids(&mut next_id);
    }
}

pub struct ContextExecutor<Context, A>
where
    A: Animal,
    A::Journey: BuildFlowWithContext<
        (Arc<Context>, DynFlow<A::State>),
        Output = (Arc<Context>, DynFlow<A::State>),
    >,
{
    _context: core::marker::PhantomData<fn() -> Context>,
    state: Option<A::State>,
    steps: DynFlow<A::State>,
    cursor: usize,
    last_emitted: Option<Serialized>,
}

impl<Context, A> ContextExecutor<Context, A>
where
    Context: 'static,
    A: Animal,
    A::Journey: BuildFlowWithContext<
        (Arc<Context>, DynFlow<A::State>),
        Output = (Arc<Context>, DynFlow<A::State>),
    >,
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

    pub fn new(context: Arc<Context>, state: A::State) -> Self {
        let (_, mut steps) = <A::Journey as BuildFlowWithContext<(
            Arc<Context>,
            DynFlow<A::State>,
        )>>::push_steps((context, Vec::new()));
        assign_flow_node_ids(&mut steps);
        let mut executor = Self {
            _context: core::marker::PhantomData,
            state: Some(state),
            steps,
            cursor: 0,
            last_emitted: None,
        };
        executor
            .settle_without_progress()
            .expect("initial settle should not fail");
        executor
    }

    pub fn is_complete(&self) -> bool {
        self.cursor >= self.steps.len()
    }

    pub fn state(&self) -> &A::State {
        self.state
            .as_ref()
            .expect("executor state is always present")
    }

    pub fn state_mut(&mut self) -> &mut A::State {
        self.state
            .as_mut()
            .expect("executor state is always present")
    }

    pub fn next_request<Request>(&mut self) -> Result<Request, ExecutorError>
    where
        Request: DeserializeOwned + Default + Serialize,
    {
        let input = self.last_emitted.take().unwrap_or_else(|| {
            serialize_input(Request::default()).expect("default input serializes")
        });
        let request = self.next_request_serialized(input)?;
        deserialize_request(request)
    }

    pub fn next_executable_request<Initial>(
        &mut self,
        initial_input: Initial,
    ) -> Result<ExecutableEffectRequest, ExecutorError>
    where
        Initial: Serialize,
    {
        let input = match self.last_emitted.take() {
            Some(input) => input,
            None => serialize_input(initial_input)?,
        };
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            match node.request_executable(state, input.clone()) {
                Ok((state, request)) => {
                    self.state = Some(state);
                    return Ok(request);
                }
                Err((state, ExecutorError::Complete)) => {
                    self.state = Some(state);
                    let state = self.state.take().expect("executor state is always present");
                    let (state, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    self.settle_without_progress()?;
                    return Err(ExecutorError::Complete);
                }
                Err((state, err)) => {
                    self.state = Some(state);
                    return Err(err);
                }
            }
        }
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
        let emitted = self.complete_serialized(completion)?;
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
        self.state.expect("executor state is always present")
    }

    fn next_request_serialized(&mut self, input: Serialized) -> Result<Serialized, ExecutorError> {
        let input = input;
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            match node.request(state, input.clone()) {
                Ok((state, request)) => {
                    self.state = Some(state);
                    return Ok(request);
                }
                Err((state, ExecutorError::Complete)) => {
                    self.state = Some(state);
                    let state = self.state.take().expect("executor state is always present");
                    let (state, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    self.settle_without_progress()?;
                    return Err(ExecutorError::Complete);
                }
                Err((state, err)) => {
                    self.state = Some(state);
                    return Err(err);
                }
            }
        }
    }
}

pub struct ManualExecutor<A>
where
    A: Animal,
    A::Journey: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    state: Option<A::State>,
    steps: DynFlow<A::State>,
    cursor: usize,
}

impl<A> ManualExecutor<A>
where
    A: Animal,
    A::Journey: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
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
        let mut steps = <A::Journey as BuildFlow<DynFlow<A::State>>>::push_steps(Vec::new());
        assign_flow_node_ids(&mut steps);
        let mut executor = Self {
            state: Some(state),
            steps,
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

    pub fn state(&self) -> &A::State {
        self.state
            .as_ref()
            .expect("executor state is always present")
    }

    pub fn state_mut(&mut self) -> &mut A::State {
        self.state
            .as_mut()
            .expect("executor state is always present")
    }

    pub fn next_request(&mut self, input: Serialized) -> Result<Serialized, ExecutorError> {
        let input = input;
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            match node.request(state, input.clone()) {
                Ok((state, request)) => {
                    self.state = Some(state);
                    return Ok(request);
                }
                Err((state, ExecutorError::Complete)) => {
                    self.state = Some(state);
                    let state = self.state.take().expect("executor state is always present");
                    let (state, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    self.settle_without_progress()?;
                    return Err(ExecutorError::Complete);
                }
                Err((state, err)) => {
                    self.state = Some(state);
                    return Err(err);
                }
            }
        }
    }

    pub fn next_executable_request(
        &mut self,
        input: Serialized,
    ) -> Result<ExecutableEffectRequest, ExecutorError> {
        let input = input;
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }

            let state = self.state.take().expect("executor state is always present");
            let node = self
                .steps
                .get_mut(self.cursor)
                .expect("cursor was checked against steps len");
            match node.request_executable(state, input.clone()) {
                Ok((state, request)) => {
                    self.state = Some(state);
                    return Ok(request);
                }
                Err((state, ExecutorError::Complete)) => {
                    self.state = Some(state);
                    let state = self.state.take().expect("executor state is always present");
                    let (state, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    self.settle_without_progress()?;
                    return Err(ExecutorError::Complete);
                }
                Err((state, err)) => {
                    self.state = Some(state);
                    return Err(err);
                }
            }
        }
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
    A: Animal,
    A::Journey: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    manual: ManualExecutor<A>,
    last_emitted: Option<Serialized>,
}

impl<A> Executor<A>
where
    A: Animal,
    A::Journey: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
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

    pub fn state(&self) -> &A::State {
        self.manual.state()
    }

    pub fn state_mut(&mut self) -> &mut A::State {
        self.manual.state_mut()
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
    ) -> Result<ExecutableEffectRequest, ExecutorError>
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
