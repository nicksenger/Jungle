use crate::{
    Animal, Attempt, BackendError, BoundAction, BoundAnimal, BoundAnimalJourney, BoundFlowStep,
    Conditional, Effect, EffectCompletion, EffectSchema, Either, Failure, Join, NoEffect,
    NodeLifecycle, NodeLifecyclePhase, RunnerOut, Running, Scoped, Select, Sleep, StateCarrier,
    Transparent, While,
};
use futures::channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use futures::StreamExt;
use inception::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use typosaurus::collections::list::{self, List as TList};

type Serialized = Vec<u8>;
type SerializedCompletion = Result<Serialized, Serialized>;
type EffectFuture =
    Pin<Box<dyn Future<Output = Result<SerializedCompletion, ExecutorError>> + Send>>;
type EffectRunner = Box<dyn FnOnce() -> EffectFuture + Send>;
type RequestError<State> = (State, ExecutorError);
type RequestResult<State, Request> = Result<(State, Request), RequestError<State>>;

#[derive(Default)]
struct LifecycleState {
    node_id: u32,
    journey_id: Option<uuid::Uuid>,
    next_activation_id: u64,
    parent_activation_path: Vec<u64>,
    current_activation_path: Option<Vec<u64>>,
    updates: Vec<NodeLifecycle>,
}

impl LifecycleState {
    fn set_node_id(&mut self, node_id: u32) {
        self.node_id = node_id;
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.journey_id = Some(journey_id);
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        if self.current_activation_path.is_none() {
            self.parent_activation_path.clear();
            self.parent_activation_path.extend_from_slice(path);
        }
    }

    fn enter(&mut self) {
        if self.current_activation_path.is_some() {
            return;
        }
        let activation_id = self.next_activation_id;
        self.next_activation_id = self.next_activation_id.saturating_add(1);
        let mut activation_path = self.parent_activation_path.clone();
        activation_path.push(activation_id);
        if let Some(journey_id) = self.journey_id {
            self.updates.push(NodeLifecycle {
                node_id: self.node_id,
                activation_path: activation_path.clone(),
                phase: NodeLifecyclePhase::Entered,
                uuid: journey_id,
            });
        }
        self.current_activation_path = Some(activation_path);
    }

    fn succeed(&mut self) {
        let Some(activation_path) = self.current_activation_path.take() else {
            return;
        };
        if let Some(journey_id) = self.journey_id {
            self.updates.push(NodeLifecycle {
                node_id: self.node_id,
                activation_path,
                phase: NodeLifecyclePhase::Succeeded,
                uuid: journey_id,
            });
        }
    }

    fn fail(&mut self) {
        let Some(activation_path) = self.current_activation_path.take() else {
            return;
        };
        if let Some(journey_id) = self.journey_id {
            self.updates.push(NodeLifecycle {
                node_id: self.node_id,
                activation_path,
                phase: NodeLifecyclePhase::Failed,
                uuid: journey_id,
            });
        }
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.current_activation_path.as_deref()
    }

    fn journey_id(&self) -> Option<uuid::Uuid> {
        self.journey_id
    }

    fn take_updates(&mut self) -> Vec<NodeLifecycle> {
        std::mem::take(&mut self.updates)
    }
}

fn input_deserialize_error(context: &'static str, err: postcard::Error) -> ExecutorError {
    ExecutorError::InputDeserialize(format!("{context}: {err}"))
}

pub trait SplitStateCarry<State> {
    type Carry;
}

impl<State, Carry> SplitStateCarry<State> for (State, Carry) {
    type Carry = Carry;
}

pub trait ArgputForState<State> {
    type Carry;
}

impl<State, T, A> ArgputForState<State> for BoundFlowStep<T, A>
where
    T: Animal<State = State>,
    A: BoundAction<T>,
{
    type Carry = A::Input;
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

impl<State, View, F> ArgputForState<State> for Scoped<View, F>
where
    F: ArgputForState<State>,
{
    type Carry = <F as ArgputForState<State>>::Carry;
}

impl<State, Carrier, F> ArgputForState<State> for crate::FocusedBoundFlow<Carrier, F>
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

impl<State, M, F> ArgputForState<State> for Attempt<F, M>
where
    F: ArgputForState<State>,
{
    type Carry = <F as ArgputForState<State>>::Carry;
}

impl<State, C, F, M> ArgputForState<State> for While<C, F, M>
where
    F: FlowCarry<State>,
{
    type Carry = <F as FlowCarry<State>>::In;
}

impl<State> ArgputForState<State> for list::Empty {
    type Carry = ();
}

impl<State, Head, Tail> ArgputForState<State> for TList<(Head, Tail)>
where
    Head: ArgputForState<State>,
{
    type Carry = <Head as ArgputForState<State>>::Carry;
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

    if let Ok(either) = postcard::from_bytes::<Either<Serialized, Serialized>>(input) {
        return match either {
            Either::Left(carry) => {
                let carry = postcard::from_bytes::<In>(&carry)
                    .map_err(|err| input_deserialize_error("conditional either left carry", err))?;
                Ok((true, carry))
            }
            Either::Right(carry) => {
                let carry = postcard::from_bytes::<In>(&carry).map_err(|err| {
                    input_deserialize_error("conditional either right carry", err)
                })?;
                Ok((false, carry))
            }
        };
    }

    let carry = postcard::from_bytes::<In>(input)
        .map_err(|err| input_deserialize_error("conditional direct carry", err))?;
    Ok((fallback(&carry), carry))
}

fn decode_loop_input<In, F>(input: &[u8], fallback: F) -> Result<(bool, In), ExecutorError>
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

    if let Ok(either) = postcard::from_bytes::<Either<Serialized, Serialized>>(input) {
        return match either {
            Either::Left(carry) | Either::Right(carry) => {
                let carry = postcard::from_bytes::<In>(&carry)
                    .map_err(|err| input_deserialize_error("while either carry", err))?;
                Ok((fallback(&carry), carry))
            }
        };
    }

    let carry = postcard::from_bytes::<In>(input)
        .map_err(|err| input_deserialize_error("while direct carry", err))?;
    Ok((fallback(&carry), carry))
}

fn deserialize_exact<T>(bytes: &[u8]) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let (value, remainder): (T, &[u8]) =
        postcard::take_from_bytes(bytes).map_err(|err| err.to_string())?;
    if !remainder.is_empty() {
        return Err("unexpected trailing bytes".to_string());
    }
    Ok(value)
}

fn try_deserialize_step_input_from_either<T>(bytes: &[u8], depth: usize) -> Option<T>
where
    T: DeserializeOwned,
{
    if depth > 16 {
        return None;
    }

    let (tag, payload) = match postcard::from_bytes::<Either<Serialized, Serialized>>(bytes) {
        Ok(Either::Left(payload)) => (0_u8, payload),
        Ok(Either::Right(payload)) => (1_u8, payload),
        Err(_) => return None,
    };

    if let Ok(value) = deserialize_exact::<T>(&payload) {
        return Some(value);
    }
    if let Some(value) = try_deserialize_step_input_from_either::<T>(&payload, depth + 1) {
        return Some(value);
    }

    let mut tagged = Vec::with_capacity(1 + payload.len());
    tagged.push(tag);
    tagged.extend_from_slice(&payload);
    if let Ok(value) = deserialize_exact::<T>(&tagged) {
        return Some(value);
    }
    try_deserialize_step_input_from_either::<T>(&tagged, depth + 1)
}

fn deserialize_step_input<T>(bytes: &[u8]) -> Result<T, ExecutorError>
where
    T: DeserializeOwned,
{
    let direct_err = match deserialize_exact::<T>(bytes) {
        Ok(value) => return Ok(value),
        Err(err) => err,
    };

    if let Some(value) = try_deserialize_step_input_from_either::<T>(bytes, 0) {
        return Ok(value);
    }

    if postcard::from_bytes::<Either<Serialized, Serialized>>(bytes).is_ok() {
        return Err(ExecutorError::InputDeserialize(format!(
            "step either envelope for {}: {direct_err}",
            core::any::type_name::<T>()
        )));
    }

    Err(ExecutorError::InputDeserialize(format!(
        "step direct input for {}: {direct_err}",
        core::any::type_name::<T>()
    )))
}

pub type DynFlow<State> = Vec<Box<dyn ErasedFlow<State> + Send>>;
pub type ErasedStep<State> = dyn ErasedFlow<State> + Send;

pub struct ExecutableEffectRequest {
    node_id: u32,
    effect_type: &'static str,
    request: Serialized,
    runner: EffectRunner,
    live_history_rx: Option<UnboundedReceiver<RunnerOut>>,
    suspended_completion: Option<SerializedCompletion>,
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
            live_history_rx: None,
            suspended_completion: None,
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

    pub fn take_live_history(&mut self) -> Option<UnboundedReceiver<RunnerOut>> {
        self.live_history_rx.take()
    }

    pub fn has_live_history(&self) -> bool {
        self.live_history_rx.is_some()
    }

    pub fn suspended_completion(&self) -> Option<&SerializedCompletion> {
        self.suspended_completion.as_ref()
    }

    fn with_live_history(mut self, live_history_rx: UnboundedReceiver<RunnerOut>) -> Self {
        self.live_history_rx = Some(live_history_rx);
        self
    }

    fn with_suspended_completion(mut self, completion: SerializedCompletion) -> Self {
        self.suspended_completion = Some(completion);
        self
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
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        Ok((state, None, false))
    }

    fn set_parent_activation_path(&mut self, _path: &[u64]) {}

    fn set_journey_id(&mut self, _journey_id: uuid::Uuid) {}

    fn current_activation_path(&self) -> Option<&[u64]> {
        None
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        Vec::new()
    }

    fn node_id(&self) -> u32 {
        0
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
    #[error("action failure: {0}")]
    ActionFailure(#[source] Failure),
    #[error("not enough completions to advance to end")]
    NotEnoughCompletions,
}

pub trait ExecutorFlow {
    type State;
    fn build_steps() -> DynFlow<Self::State>;
}

pub trait StepCarry {
    type Carry: Send;
}

impl<T, A> StepCarry for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
    <A as BoundAction<T>>::Carry: Send,
{
    type Carry = A::Carry;
}

pub struct TypedErasedStep<Step>
where
    Step: StepCarry,
{
    lifecycle: LifecycleState,
    node_id: u32,
    complete: bool,
    waiting_completion: bool,
    pending_inline_input: Option<Serialized>,
    pending_carry: Option<<Step as StepCarry>::Carry>,
    marker: core::marker::PhantomData<fn() -> Step>,
}

impl<Step> TypedErasedStep<Step>
where
    Step: StepCarry,
{
    pub fn new() -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            complete: false,
            waiting_completion: false,
            pending_inline_input: None,
            pending_carry: None,
            marker: core::marker::PhantomData,
        }
    }
}

impl<Step> Default for TypedErasedStep<Step>
where
    Step: StepCarry,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, A> ErasedFlow<T::State> for TypedErasedStep<BoundFlowStep<T, A>>
where
    T: Animal,
    A: BoundAction<T>,
    <A as BoundAction<T>>::Carry: Send + 'static,
    <A as BoundAction<T>>::Effect: Effect<()>,
    <<A as BoundAction<T>>::Effect as EffectSchema>::In: 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Out: 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Err: Serialize + 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Out: DeserializeOwned,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Err: DeserializeOwned,
    A::Input: DeserializeOwned,
    A::Output: Serialize,
{
    fn try_complete_without_progress(
        &mut self,
        state: T::State,
    ) -> Result<(T::State, Option<Serialized>, bool), ExecutorError> {
        let Some(input) = self.pending_inline_input.take() else {
            return Ok((state, None, false));
        };

        if self.complete {
            return Ok((state, None, true));
        }
        if self.waiting_completion {
            return Ok((state, None, false));
        }

        let typed_input = deserialize_step_input::<A::Input>(&input)?;
        self.lifecycle.enter();
        let (state, (_request, carry)) =
            <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
        let completion = Ok(postcard::to_allocvec(&())
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?);
        self.waiting_completion = true;
        let (state, emitted) = self.complete(state, completion)?;
        Ok((state, Some(emitted), true))
    }

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
        if core::any::type_name::<<A as BoundAction<T>>::Effect>()
            == core::any::type_name::<NoEffect>()
        {
            if let Err(err) = deserialize_step_input::<A::Input>(&input) {
                return Err((state, err));
            }
            self.lifecycle.enter();
            self.pending_inline_input = Some(input);
            return Err((state, ExecutorError::Complete));
        }

        let typed_input = match deserialize_step_input::<A::Input>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, err)),
        };
        self.lifecycle.enter();
        let (state, (request, carry)) = <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
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
        if core::any::type_name::<<A as BoundAction<T>>::Effect>()
            == core::any::type_name::<NoEffect>()
        {
            if let Err(err) = deserialize_step_input::<A::Input>(&input) {
                return Err((state, err));
            }
            self.lifecycle.enter();
            self.pending_inline_input = Some(input);
            return Err((state, ExecutorError::Complete));
        }

        let typed_input = match deserialize_step_input::<A::Input>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, err)),
        };
        self.lifecycle.enter();
        let (state, (request, carry)) = <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
        let effect_input = request.into_input();
        let request = match postcard::to_allocvec(&effect_input) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let completion =
                    <<A as BoundAction<T>>::Effect as Effect<()>>::effect(&(), effect_input).await;
                serialize_completion(completion)
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(
                self.node_id,
                core::any::type_name::<<A as BoundAction<T>>::Effect>(),
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

        let typed_completion: EffectCompletion<<A as BoundAction<T>>::Effect> = match completion {
            Ok(output) => Ok(postcard::from_bytes::<
                <<A as BoundAction<T>>::Effect as EffectSchema>::Out,
            >(&output)
            .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?),
            Err(error) => Err(postcard::from_bytes::<
                <<A as BoundAction<T>>::Effect as EffectSchema>::Err,
            >(&error)
            .map_err(|err| ExecutorError::ErrorDeserialize(err.to_string()))?),
        };

        let carry = self
            .pending_carry
            .take()
            .expect("carry must exist while waiting for completion");
        let mut state = state;
        let view = <<A as BoundAction<T>>::Aspect as StateCarrier<T::State>>::focus(&mut state);
        let emitted = match <A as BoundAction<T>>::absorb_with_carry(view, typed_completion, carry)
        {
            Ok(emitted) => emitted,
            Err(failure) => {
                self.waiting_completion = false;
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
        };
        let emitted = postcard::to_allocvec(&emitted)
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))?;
        self.waiting_completion = false;
        self.complete = true;
        self.lifecycle.succeed();
        Ok((state, emitted))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        self.lifecycle.take_updates()
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
    }
}

pub struct ContextualTypedErasedStep<Context, R>
where
    R: StepCarry,
{
    context: Arc<Context>,
    lifecycle: LifecycleState,
    node_id: u32,
    complete: bool,
    waiting_completion: bool,
    pending_inline_input: Option<Serialized>,
    pending_carry: Option<<R as StepCarry>::Carry>,
    marker: core::marker::PhantomData<fn() -> R>,
}

impl<Context, R> ContextualTypedErasedStep<Context, R>
where
    R: StepCarry,
{
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            context,
            lifecycle: LifecycleState::default(),
            node_id: 0,
            complete: false,
            waiting_completion: false,
            pending_inline_input: None,
            pending_carry: None,
            marker: core::marker::PhantomData,
        }
    }
}

impl<Context, T, A> ErasedFlow<T::State> for ContextualTypedErasedStep<Context, BoundFlowStep<T, A>>
where
    Context: Send + Sync + 'static,
    T: Animal,
    A: BoundAction<T>,
    <A as BoundAction<T>>::Carry: Send + 'static,
    <A as BoundAction<T>>::Effect: Effect<Context>,
    <A as BoundAction<T>>::Effect: EffectSchema<
        Context,
        In = <<A as BoundAction<T>>::Effect as EffectSchema>::In,
        Out = <<A as BoundAction<T>>::Effect as EffectSchema>::Out,
        Err = <<A as BoundAction<T>>::Effect as EffectSchema>::Err,
    >,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::In: 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Out:
        Serialize + DeserializeOwned + 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Err:
        Serialize + DeserializeOwned + 'static,
    A::Input: DeserializeOwned,
    A::Output: Serialize,
{
    fn try_complete_without_progress(
        &mut self,
        state: T::State,
    ) -> Result<(T::State, Option<Serialized>, bool), ExecutorError> {
        let Some(input) = self.pending_inline_input.take() else {
            return Ok((state, None, false));
        };

        if self.complete {
            return Ok((state, None, true));
        }
        if self.waiting_completion {
            return Ok((state, None, false));
        }

        let typed_input = deserialize_step_input::<A::Input>(&input)?;
        self.lifecycle.enter();
        let (state, (_request, carry)) =
            <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
        let completion = Ok(postcard::to_allocvec(&())
            .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?);
        self.waiting_completion = true;
        let (state, emitted) = self.complete(state, completion)?;
        Ok((state, Some(emitted), true))
    }

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
        if core::any::type_name::<<A as BoundAction<T>>::Effect>()
            == core::any::type_name::<NoEffect>()
        {
            if let Err(err) = deserialize_step_input::<A::Input>(&input) {
                return Err((state, err));
            }
            self.lifecycle.enter();
            self.pending_inline_input = Some(input);
            return Err((state, ExecutorError::Complete));
        }

        let typed_input = match deserialize_step_input::<A::Input>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, err)),
        };
        self.lifecycle.enter();
        let (state, (request, carry)) = <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
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
        if core::any::type_name::<<A as BoundAction<T>>::Effect>()
            == core::any::type_name::<NoEffect>()
        {
            if let Err(err) = deserialize_step_input::<A::Input>(&input) {
                return Err((state, err));
            }
            self.lifecycle.enter();
            self.pending_inline_input = Some(input);
            return Err((state, ExecutorError::Complete));
        }

        let typed_input = match deserialize_step_input::<A::Input>(&input) {
            Ok(typed_input) => typed_input,
            Err(err) => return Err((state, err)),
        };
        self.lifecycle.enter();
        let (state, (request, carry)) = <BoundFlowStep<T, A> as Running>::run((state, typed_input));
        self.pending_carry = Some(carry);
        let effect_input = request.into_input();
        let request = match postcard::to_allocvec(&effect_input) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let context = Arc::clone(&self.context);
        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let completion = <<A as BoundAction<T>>::Effect as Effect<Context>>::effect(
                    context.as_ref(),
                    effect_input,
                )
                .await;
                serialize_completion(completion)
            })
        });

        self.waiting_completion = true;
        Ok((
            state,
            ExecutableEffectRequest::new(
                self.node_id,
                core::any::type_name::<<A as BoundAction<T>>::Effect>(),
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

        let typed_completion: EffectCompletion<<A as BoundAction<T>>::Effect> = match completion {
            Ok(output) => Ok(postcard::from_bytes::<
                <<A as BoundAction<T>>::Effect as EffectSchema>::Out,
            >(&output)
            .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?),
            Err(error) => Err(postcard::from_bytes::<
                <<A as BoundAction<T>>::Effect as EffectSchema>::Err,
            >(&error)
            .map_err(|err| ExecutorError::ErrorDeserialize(err.to_string()))?),
        };

        let carry = self
            .pending_carry
            .take()
            .expect("carry must exist while waiting for completion");
        let mut state = state;
        let view = <<A as BoundAction<T>>::Aspect as StateCarrier<T::State>>::focus(&mut state);
        let emitted = match <A as BoundAction<T>>::absorb_with_carry(view, typed_completion, carry)
        {
            Ok(emitted) => emitted,
            Err(failure) => {
                self.waiting_completion = false;
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
        };
        let emitted = postcard::to_allocvec(&emitted)
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))?;
        self.waiting_completion = false;
        self.complete = true;
        self.lifecycle.succeed();
        Ok((state, emitted))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        self.lifecycle.take_updates()
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
    }
}

#[derive(Clone, Copy)]
enum ActiveBranch {
    Left,
    Right,
}

fn encode_conditional_emitted(active_branch: ActiveBranch, emitted: Serialized) -> Serialized {
    let tagged = match active_branch {
        ActiveBranch::Left => Either::Left(emitted),
        ActiveBranch::Right => Either::Right(emitted),
    };
    postcard::to_allocvec(&tagged).expect("conditional emitted envelope should serialize")
}

struct ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    lifecycle: LifecycleState,
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
    active_branch: Option<ActiveBranch>,
    cursor: usize,
    deferred_emitted: Option<Serialized>,
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
            lifecycle: LifecycleState::default(),
            node_id: 0,
            left,
            right,
            choose_left,
            active_branch: None,
            cursor: 0,
            deferred_emitted: None,
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

impl<State: Clone, In> ErasedFlow<State> for ConditionalErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.active_branch.is_some() && self.cursor >= self.branch_len() {
            self.lifecycle.succeed();
            return Ok((state, self.deferred_emitted.take(), true));
        }
        if self.active_branch.is_none() {
            return Ok((state, None, false));
        }

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.set_parent_activation_path(&parent_path);
        if node.is_waiting_completion() {
            return Ok((state, None, false));
        }

        let (state, emitted, completed) = match node.try_complete_without_progress(state) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if !completed {
            return Ok((state, emitted, false));
        }

        self.cursor += 1;
        if self.cursor >= self.branch_len() {
            let active_branch = self
                .active_branch
                .expect("active branch is present while conditional is executing");
            let emitted = emitted.map(|bytes| encode_conditional_emitted(active_branch, bytes));
            self.deferred_emitted = emitted.clone();
            self.lifecycle.succeed();
            return Ok((state, emitted, true));
        }

        Ok((state, emitted, false))
    }

    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let (choose_left, branch_input) = if self.active_branch.is_none() {
            match decode_controlled_input::<In, _>(&input, |carry| {
                (self.choose_left)(&state, carry)
            }) {
                Ok(pair) => pair,
                Err(err) => return Err((state, err)),
            }
        } else {
            let request_input = input.clone();
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            return node.request(state, request_input);
        };
        if self.active_branch.is_none() {
            self.lifecycle.enter();
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

        let mut state = state;
        let mut branch_input = branch_input;
        loop {
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            match node.request(state, branch_input.clone()) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    let (next_state, emitted, completed) =
                        match node.try_complete_without_progress(next_state.clone()) {
                            Ok(result) => result,
                            Err(ExecutorError::ActionFailure(failure)) => {
                                self.lifecycle.fail();
                                return Err((next_state, ExecutorError::ActionFailure(failure)));
                            }
                            Err(err) => return Err((next_state, err)),
                        };
                    state = next_state;
                    if let Some(emitted) = emitted {
                        branch_input = emitted;
                    }
                    if completed {
                        self.cursor += 1;
                        if self.cursor >= self.branch_len() {
                            let active_branch = self
                                .active_branch
                                .expect("active branch is present while conditional is executing");
                            let emitted =
                                encode_conditional_emitted(active_branch, branch_input.clone());
                            self.deferred_emitted = Some(emitted);
                            self.lifecycle.succeed();
                            return Err((state, ExecutorError::Complete));
                        }
                        continue;
                    }
                    return Err((state, ExecutorError::Complete));
                }
                Err((state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((state, ExecutorError::ActionFailure(failure)));
                }
                Err((state, err)) => return Err((state, err)),
            }
        }
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.cursor >= self.branch_len() {
            return Err(ExecutorError::Complete);
        }

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.set_parent_activation_path(&parent_path);
        let (state, emitted) = match node.complete(state, completion) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        let node_complete = node.is_complete();
        if node_complete {
            self.cursor += 1;
        }
        if node_complete && self.cursor >= self.branch_len() {
            let active_branch = self
                .active_branch
                .expect("active branch is present while conditional is executing");
            let emitted = encode_conditional_emitted(active_branch, emitted);
            self.deferred_emitted = Some(emitted.clone());
            self.lifecycle.succeed();
            return Ok((state, emitted));
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let (choose_left, branch_input) = if self.active_branch.is_none() {
            match decode_controlled_input::<In, _>(&input, |carry| {
                (self.choose_left)(&state, carry)
            }) {
                Ok(pair) => pair,
                Err(err) => return Err((state, err)),
            }
        } else {
            let request_input = input.clone();
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            return node.request_executable(state, request_input);
        };
        if self.active_branch.is_none() {
            self.lifecycle.enter();
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

        let mut state = state;
        let mut branch_input = branch_input;
        loop {
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            match node.request_executable(state, branch_input.clone()) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    let (next_state, emitted, completed) =
                        match node.try_complete_without_progress(next_state.clone()) {
                            Ok(result) => result,
                            Err(ExecutorError::ActionFailure(failure)) => {
                                self.lifecycle.fail();
                                return Err((next_state, ExecutorError::ActionFailure(failure)));
                            }
                            Err(err) => return Err((next_state, err)),
                        };
                    state = next_state;
                    if let Some(emitted) = emitted {
                        branch_input = emitted;
                    }
                    if completed {
                        self.cursor += 1;
                        if self.cursor >= self.branch_len() {
                            let active_branch = self
                                .active_branch
                                .expect("active branch is present while conditional is executing");
                            let emitted =
                                encode_conditional_emitted(active_branch, branch_input.clone());
                            self.deferred_emitted = Some(emitted);
                            self.lifecycle.succeed();
                            return Err((state, ExecutorError::Complete));
                        }
                        continue;
                    }
                    return Err((state, ExecutorError::Complete));
                }
                Err((state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((state, ExecutorError::ActionFailure(failure)));
                }
                Err((state, err)) => return Err((state, err)),
            }
        }
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

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.left, journey_id);
        assign_flow_journey_id(&mut self.right, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.left));
        updates.extend(take_flow_node_lifecycle_updates(&mut self.right));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
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
    lifecycle: LifecycleState,
    node_id: u32,
    should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
    build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    active_body: DynFlow<State>,
    body_node_id_start: u32,
    body_cursor: usize,
    complete: bool,
    deferred_state: Option<State>,
    deferred_emitted: Option<Serialized>,
    active_control_input: Option<Serialized>,
    marker: core::marker::PhantomData<fn() -> In>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SelectRequestEnvelope {
    left: Serialized,
    right: Serialized,
}

struct SelectErasedFlow<State> {
    lifecycle: LifecycleState,
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    build_left: Box<dyn Fn() -> DynFlow<State> + Send>,
    build_right: Box<dyn Fn() -> DynFlow<State> + Send>,
    pending_input: Option<Serialized>,
    waiting_completion: bool,
    complete: bool,
    suppress_child_lifecycle_replay: bool,
}

impl<State> SelectErasedFlow<State> {
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        build_left: Box<dyn Fn() -> DynFlow<State> + Send>,
        build_right: Box<dyn Fn() -> DynFlow<State> + Send>,
    ) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            left,
            right,
            build_left,
            build_right,
            pending_input: None,
            waiting_completion: false,
            complete: false,
            suppress_child_lifecycle_replay: false,
        }
    }
}

struct SelectContextErasedFlow<State> {
    lifecycle: LifecycleState,
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    build_left: Box<dyn Fn() -> DynFlow<State> + Send>,
    build_right: Box<dyn Fn() -> DynFlow<State> + Send>,
    pending_input: Option<Serialized>,
    waiting_completion: bool,
    complete: bool,
    suppress_child_lifecycle_replay: bool,
}

impl<State> SelectContextErasedFlow<State> {
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        build_left: Box<dyn Fn() -> DynFlow<State> + Send>,
        build_right: Box<dyn Fn() -> DynFlow<State> + Send>,
    ) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            left,
            right,
            build_left,
            build_right,
            pending_input: None,
            waiting_completion: false,
            complete: false,
            suppress_child_lifecycle_replay: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FlowCompletionTrace {
    completions: Vec<SerializedCompletion>,
    emitted: Serialized,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum SelectTraceEnvelope {
    Left(FlowCompletionTrace),
    Right(FlowCompletionTrace),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum JoinCompletionEnvelope {
    Left(SerializedCompletion),
    Right(SerializedCompletion),
}

struct JoinBranchProgress<State> {
    state: State,
    cursor: usize,
    last_input: Serialized,
    complete: bool,
}

impl<State> JoinBranchProgress<State> {
    fn new(state: State, input: Serialized) -> Self {
        Self {
            state,
            cursor: 0,
            last_input: input,
            complete: false,
        }
    }
}

trait JoinFocusMarker<State> {
    const ENABLED: bool;

    fn merge_into(_target: &mut State, _branch_state: State) {}
}

impl<State> JoinFocusMarker<State> for list::Empty {
    const ENABLED: bool = true;
}

impl<State, Head, Tail> JoinFocusMarker<State> for TList<(Head, Tail)>
where
    State: Clone,
    Head: JoinFocusMarker<State>,
    Tail: JoinFocusMarker<State>,
{
    const ENABLED: bool =
        <Head as JoinFocusMarker<State>>::ENABLED && <Tail as JoinFocusMarker<State>>::ENABLED;

    fn merge_into(target: &mut State, branch_state: State) {
        if !Self::ENABLED {
            return;
        }

        <Head as JoinFocusMarker<State>>::merge_into(target, branch_state.clone());
        <Tail as JoinFocusMarker<State>>::merge_into(target, branch_state);
    }
}

impl<State, T, A> JoinFocusMarker<State> for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    const ENABLED: bool = false;
}

impl<State, P, L, R, M> JoinFocusMarker<State> for Conditional<P, L, R, M> {
    const ENABLED: bool = false;
}

impl<State, C, F, M> JoinFocusMarker<State> for While<C, F, M> {
    const ENABLED: bool = false;
}

impl<State, M, F> JoinFocusMarker<State> for Transparent<M, F> {
    const ENABLED: bool = false;
}

impl<State, L, R, M> JoinFocusMarker<State> for Select<L, R, M> {
    const ENABLED: bool = false;
}

impl<State, L, R, M> JoinFocusMarker<State> for Join<L, R, M>
where
    State: Clone,
    L: JoinFocusMarker<State>,
    R: JoinFocusMarker<State>,
{
    const ENABLED: bool =
        <L as JoinFocusMarker<State>>::ENABLED && <R as JoinFocusMarker<State>>::ENABLED;

    fn merge_into(target: &mut State, branch_state: State) {
        if !Self::ENABLED {
            return;
        }

        <L as JoinFocusMarker<State>>::merge_into(target, branch_state.clone());
        <R as JoinFocusMarker<State>>::merge_into(target, branch_state);
    }
}

impl<State, M, F> JoinFocusMarker<State> for Attempt<F, M> {
    const ENABLED: bool = false;
}

impl<State, Carrier, F> JoinFocusMarker<State> for crate::FocusedBoundFlow<Carrier, F>
where
    State: Clone,
    Carrier: crate::Aspect<State>,
    <Carrier as crate::StateCarrier<State>>::Focus: Clone,
{
    const ENABLED: bool = true;

    fn merge_into(target: &mut State, mut branch_state: State) {
        let focused = <Carrier as crate::StateCarrier<State>>::focus(&mut branch_state).clone();
        *<Carrier as crate::StateCarrier<State>>::focus(target) = focused;
    }
}

fn settle_subflow_without_progress<State>(
    flow: &mut DynFlow<State>,
    cursor: &mut usize,
    mut state: State,
    last_input: &mut Serialized,
) -> Result<State, ExecutorError> {
    loop {
        if *cursor >= flow.len() {
            break;
        }

        let node = flow
            .get_mut(*cursor)
            .expect("cursor was checked against steps len");
        if node.is_waiting_completion() {
            break;
        }
        let (next, emitted, completed) = node.try_complete_without_progress(state)?;
        state = next;
        if let Some(emitted) = emitted {
            *last_input = emitted;
        }
        if completed {
            *cursor += 1;
            continue;
        }
        break;
    }
    Ok(state)
}

fn subflow_next_executable_request<State>(
    flow: &mut DynFlow<State>,
    cursor: &mut usize,
    mut state: State,
    last_input: &mut Serialized,
) -> RequestResult<State, ExecutableEffectRequest>
where
    State: Clone,
{
    loop {
        state = match settle_subflow_without_progress(flow, cursor, state.clone(), last_input) {
            Ok(state) => state,
            Err(err) => return Err((state, err)),
        };
        if *cursor >= flow.len() {
            return Err((state, ExecutorError::Complete));
        }

        let node = flow
            .get_mut(*cursor)
            .expect("cursor was checked against steps len");
        match node.request_executable(state, last_input.clone()) {
            Ok((next_state, request)) => {
                return Ok((next_state, request));
            }
            Err((next_state, ExecutorError::Complete)) => {
                state = next_state;
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(state.clone()) {
                        Ok(result) => result,
                        Err(err) => return Err((state, err)),
                    };
                state = next_state;
                if let Some(emitted) = emitted {
                    *last_input = emitted;
                }
                if completed {
                    *cursor += 1;
                    continue;
                }
                state = match settle_subflow_without_progress(
                    flow,
                    cursor,
                    state.clone(),
                    last_input,
                ) {
                    Ok(state) => state,
                    Err(err) => return Err((state, err)),
                };
                if *cursor >= flow.len() {
                    return Err((state, ExecutorError::Complete));
                }
                continue;
            }
            Err((next_state, err)) => {
                return Err((next_state, err));
            }
        }
    }
}

fn subflow_complete_serialized<State>(
    flow: &mut DynFlow<State>,
    cursor: &mut usize,
    state: State,
    last_input: &mut Serialized,
    completion: SerializedCompletion,
) -> Result<(State, Serialized), ExecutorError> {
    let state = settle_subflow_without_progress(flow, cursor, state, last_input)?;
    if *cursor >= flow.len() {
        return Err(ExecutorError::Complete);
    }

    let node = flow
        .get_mut(*cursor)
        .expect("cursor was checked against steps len");
    let (next_state, emitted) = node.complete(state, completion)?;
    if node.is_complete() {
        *cursor += 1;
    }
    *last_input = emitted.clone();
    let next_state = settle_subflow_without_progress(flow, cursor, next_state, last_input)?;
    Ok((next_state, emitted))
}

fn emit_subflow_history(
    live_history_tx: Option<&UnboundedSender<RunnerOut>>,
    event: RunnerOut,
) -> Result<(), ExecutorError> {
    if let Some(tx) = live_history_tx {
        let _ = tx.unbounded_send(event);
    }
    Ok(())
}

fn emit_subflow_lifecycle_updates<State>(
    flow: &mut DynFlow<State>,
    live_history_tx: Option<&UnboundedSender<RunnerOut>>,
) -> Result<(), ExecutorError> {
    let Some(tx) = live_history_tx else {
        return Ok(());
    };
    for update in take_flow_node_lifecycle_updates(flow) {
        emit_subflow_history(Some(tx), RunnerOut::NodeLifecycle(update))?;
    }
    Ok(())
}

async fn run_request_with_live_history(
    mut request: ExecutableEffectRequest,
    live_history_tx: Option<UnboundedSender<RunnerOut>>,
) -> Result<SerializedCompletion, ExecutorError> {
    let Some(mut live_history_rx) = request.take_live_history() else {
        return request.run().await;
    };
    let mut completion = Box::pin(request.run());
    loop {
        match futures::future::select(completion, live_history_rx.next()).await {
            futures::future::Either::Left((result, _)) => {
                while let Some(event) = live_history_rx.next().await {
                    emit_subflow_history(live_history_tx.as_ref(), event)?;
                }
                return result;
            }
            futures::future::Either::Right((maybe_event, next_completion)) => {
                completion = next_completion;
                match maybe_event {
                    Some(event) => emit_subflow_history(live_history_tx.as_ref(), event)?,
                    None => return completion.await,
                }
            }
        }
    }
}

async fn run_subflow_to_end_with_state<State>(
    mut flow: DynFlow<State>,
    mut state: State,
    input: Serialized,
    start_node_id: u32,
    journey_id: Option<uuid::Uuid>,
    parent_activation_path: &[u64],
    live_history_tx: Option<UnboundedSender<RunnerOut>>,
) -> Result<(State, FlowCompletionTrace), ExecutorError>
where
    State: Clone,
{
    assign_flow_node_ids_starting_at(&mut flow, start_node_id);
    if let Some(journey_id) = journey_id {
        assign_flow_journey_id(&mut flow, journey_id);
    }
    assign_flow_parent_activation_path(&mut flow, parent_activation_path);
    let mut cursor = 0_usize;
    let mut completions = Vec::new();
    let mut last_emitted = input;

    loop {
        let request =
            match subflow_next_executable_request(&mut flow, &mut cursor, state, &mut last_emitted)
            {
                Ok((next_state, request)) => {
                    state = next_state;
                    emit_subflow_lifecycle_updates(&mut flow, live_history_tx.as_ref())?;
                    request
                }
                Err((next_state, ExecutorError::Complete)) => {
                    let settled = settle_subflow_without_progress(
                        &mut flow,
                        &mut cursor,
                        next_state,
                        &mut last_emitted,
                    )?;
                    emit_subflow_lifecycle_updates(&mut flow, live_history_tx.as_ref())?;
                    if cursor < flow.len() {
                        return Err(ExecutorError::NoPendingRequest);
                    }
                    return Ok((
                        settled,
                        FlowCompletionTrace {
                            completions,
                            emitted: last_emitted,
                        },
                    ));
                }
                Err((_next_state, err)) => {
                    emit_subflow_lifecycle_updates(&mut flow, live_history_tx.as_ref())?;
                    return Err(err);
                }
            };
        if let Some(journey_id) = journey_id {
            emit_subflow_history(
                live_history_tx.as_ref(),
                RunnerOut::EffectInput {
                    node_id: request.node_id(),
                    data: request.request_bytes().to_vec(),
                    uuid: journey_id,
                },
            )?;
        }
        let request_node_id = request.node_id();
        let completion = run_request_with_live_history(request, live_history_tx.clone()).await?;
        if let Some(journey_id) = journey_id {
            match &completion {
                Ok(output) => emit_subflow_history(
                    live_history_tx.as_ref(),
                    RunnerOut::EffectSuccessOutput {
                        node_id: request_node_id,
                        data: output.clone(),
                        uuid: journey_id,
                    },
                )?,
                Err(error) => emit_subflow_history(
                    live_history_tx.as_ref(),
                    RunnerOut::EffectFailureOutput {
                        node_id: request_node_id,
                        data: error.clone(),
                        uuid: journey_id,
                    },
                )?,
            }
        }
        let (next_state, emitted) = subflow_complete_serialized(
            &mut flow,
            &mut cursor,
            state,
            &mut last_emitted,
            completion.clone(),
        )?;
        emit_subflow_lifecycle_updates(&mut flow, live_history_tx.as_ref())?;
        state = next_state;
        completions.push(completion);
        last_emitted = emitted;
    }
}

async fn run_subflow_to_end<State>(
    flow: DynFlow<State>,
    state: State,
    input: Serialized,
    start_node_id: u32,
    journey_id: Option<uuid::Uuid>,
    parent_activation_path: &[u64],
    live_history_tx: Option<UnboundedSender<RunnerOut>>,
) -> Result<FlowCompletionTrace, ExecutorError>
where
    State: Clone,
{
    let (_state, trace) = run_subflow_to_end_with_state(
        flow,
        state,
        input,
        start_node_id,
        journey_id,
        parent_activation_path,
        live_history_tx,
    )
    .await?;
    Ok(trace)
}

async fn run_select_race_to_completion<State>(
    left_flow: DynFlow<State>,
    left_state: State,
    left_input: Serialized,
    left_start_node_id: u32,
    right_flow: DynFlow<State>,
    right_state: State,
    right_input: Serialized,
    right_start_node_id: u32,
    journey_id: Option<uuid::Uuid>,
    parent_activation_path: Vec<u64>,
    live_history_tx: Option<UnboundedSender<RunnerOut>>,
) -> Result<SelectTraceEnvelope, ExecutorError>
where
    State: Clone + Send + 'static,
{
    let (result_tx, result_rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("jungle-select-race".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    let _ = result_tx.send(Err(ExecutorError::ClientTransport(format!(
                        "select race runtime build failed: {err}"
                    ))));
                    return;
                }
            };

            let result = runtime.block_on(async move {
                let left_parent_activation_path = parent_activation_path.clone();
                let right_parent_activation_path = parent_activation_path;
                let left_live_history_tx = live_history_tx.clone();
                let right_live_history_tx = live_history_tx;
                let left = tokio::spawn(async move {
                    run_subflow_to_end(
                        left_flow,
                        left_state,
                        left_input,
                        left_start_node_id,
                        journey_id,
                        &left_parent_activation_path,
                        left_live_history_tx,
                    )
                    .await
                });
                let right = tokio::spawn(async move {
                    run_subflow_to_end(
                        right_flow,
                        right_state,
                        right_input,
                        right_start_node_id,
                        journey_id,
                        &right_parent_activation_path,
                        right_live_history_tx,
                    )
                    .await
                });

                tokio::pin!(left);
                tokio::pin!(right);

                tokio::select! {
                    left_result = &mut left => {
                        right.abort();
                        let trace = left_result.map_err(|err| {
                            ExecutorError::ClientTransport(format!(
                                "select left task join failed: {err}"
                            ))
                        })??;
                        Ok(SelectTraceEnvelope::Left(trace))
                    }
                    right_result = &mut right => {
                        left.abort();
                        let trace = right_result.map_err(|err| {
                            ExecutorError::ClientTransport(format!(
                                "select right task join failed: {err}"
                            ))
                        })??;
                        Ok(SelectTraceEnvelope::Right(trace))
                    }
                }
            });

            let _ = result_tx.send(result);
        })
        .map_err(|err| {
            ExecutorError::ClientTransport(format!("select race thread spawn failed: {err}"))
        })?;

    result_rx.await.map_err(|_| {
        ExecutorError::ClientTransport("select race thread dropped before sending a result".into())
    })?
}

fn replay_subflow_trace<State>(
    flow: &mut DynFlow<State>,
    state: State,
    input: Serialized,
    trace: FlowCompletionTrace,
) -> Result<(State, Serialized), ExecutorError>
where
    State: Clone,
{
    let mut cursor = 0_usize;
    let mut state = state;
    let mut next_input = input;

    for completion in trace.completions {
        let (next_state, _request) =
            subflow_next_executable_request(flow, &mut cursor, state, &mut next_input)
                .map_err(|(_, err)| err)?;
        state = next_state;
        let (next_state, emitted) =
            subflow_complete_serialized(flow, &mut cursor, state, &mut next_input, completion)?;
        state = next_state;
        next_input = emitted;
    }

    loop {
        match subflow_next_executable_request(flow, &mut cursor, state, &mut next_input) {
            Ok((_next_state, _request)) => {
                // Trace replay has no extra completions to apply; seeing another external
                // request means the trace did not include all required completions.
                return Err(ExecutorError::NoPendingRequest);
            }
            Err((next_state, ExecutorError::Complete)) => {
                state = next_state;
                break;
            }
            Err((next_state, err)) => {
                let _ = next_state;
                return Err(err);
            }
        }
    }

    state = settle_subflow_without_progress(flow, &mut cursor, state, &mut next_input)?;
    if cursor < flow.len() {
        return Err(ExecutorError::NoPendingRequest);
    }
    Ok((state, trace.emitted))
}

struct TransparentErasedFlow<State> {
    lifecycle: LifecycleState,
    node_id: u32,
    inner: DynFlow<State>,
    cursor: usize,
    pending_input: Option<Serialized>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> TransparentErasedFlow<State> {
    fn new(inner: DynFlow<State>) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            inner,
            cursor: 0,
            pending_input: None,
            waiting_completion: false,
            complete: false,
        }
    }
}

impl<State> ErasedFlow<State> for TransparentErasedFlow<State>
where
    State: Clone + Send + 'static,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err((state, ExecutorError::ActionFailure(failure)));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.pending_input = Some(last_input);
            self.lifecycle.succeed();
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            self.lifecycle.fail();
                            return Err((next_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                self.pending_input = Some(emitted.unwrap_or(last_input));
                if completed {
                    self.cursor += 1;
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                self.lifecycle.fail();
                Err((next_state, ExecutorError::ActionFailure(failure)))
            }
            Err((next_state, err)) => Err((next_state, err)),
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
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }
        if self.cursor >= self.inner.len() {
            return Err(ExecutorError::Complete);
        }

        let last_input = self
            .pending_input
            .take()
            .ok_or(ExecutorError::NoPendingRequest)?;
        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.complete(state.clone(), completion) {
            Ok((next_state, emitted)) => {
                self.waiting_completion = false;
                let mut last_input = emitted;
                if node.is_complete() {
                    self.cursor += 1;
                }
                let next_state = match settle_subflow_without_progress(
                    &mut self.inner,
                    &mut self.cursor,
                    next_state.clone(),
                    &mut last_input,
                ) {
                    Ok(state) => state,
                    Err(ExecutorError::ActionFailure(failure)) => {
                        self.lifecycle.fail();
                        return Err(ExecutorError::ActionFailure(failure));
                    }
                    Err(err) => return Err(err),
                };
                if self.cursor >= self.inner.len() {
                    self.complete = true;
                    self.lifecycle.succeed();
                }
                Ok((next_state, last_input))
            }
            Err(ExecutorError::ActionFailure(failure)) => {
                self.waiting_completion = false;
                self.lifecycle.fail();
                Err(ExecutorError::ActionFailure(failure))
            }
            Err(err) => {
                self.waiting_completion = false;
                self.pending_input = Some(last_input);
                Err(err)
            }
        }
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
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err((state, ExecutorError::ActionFailure(failure)));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.pending_input = Some(last_input);
            self.lifecycle.succeed();
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request_executable(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            self.lifecycle.fail();
                            return Err((next_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                self.pending_input = Some(emitted.unwrap_or(last_input));
                if completed {
                    self.cursor += 1;
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                self.lifecycle.fail();
                Err((next_state, ExecutorError::ActionFailure(failure)))
            }
            Err((next_state, err)) => Err((next_state, err)),
        }
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.complete {
            return Ok((state, self.pending_input.take(), true));
        }
        if self.waiting_completion {
            return Ok((state, None, false));
        }

        let Some(mut last_input) = self.pending_input.take() else {
            return Ok((state, None, false));
        };
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.lifecycle.succeed();
            return Ok((state, Some(last_input), true));
        }

        self.pending_input = Some(last_input);
        Ok((state, None, false))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.inner, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.inner));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
        for node in &mut self.inner {
            node.assign_node_ids(next_id);
        }
    }
}

struct AttemptErasedFlow<State, Out> {
    lifecycle: LifecycleState,
    node_id: u32,
    inner: DynFlow<State>,
    cursor: usize,
    pending_input: Option<Serialized>,
    waiting_completion: bool,
    complete: bool,
    marker: core::marker::PhantomData<fn() -> Out>,
}

impl<State, Out> AttemptErasedFlow<State, Out>
where
    Out: Serialize + DeserializeOwned,
{
    fn new(inner: DynFlow<State>) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            inner,
            cursor: 0,
            pending_input: None,
            waiting_completion: false,
            complete: false,
            marker: core::marker::PhantomData,
        }
    }

    fn encode_success(emitted: Serialized) -> Result<Serialized, ExecutorError> {
        let output = deserialize_emitted::<Out>(emitted)?;
        postcard::to_allocvec(&Ok::<Out, Failure>(output))
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))
    }

    fn encode_failure(failure: Failure) -> Result<Serialized, ExecutorError> {
        postcard::to_allocvec(&Err::<Out, Failure>(failure))
            .map_err(|err| ExecutorError::EmitSerialize(err.to_string()))
    }
}

impl<State, Out> ErasedFlow<State> for AttemptErasedFlow<State, Out>
where
    State: Clone + Send + 'static,
    Out: Serialize + DeserializeOwned + Send + 'static,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                let emitted = match Self::encode_failure(failure) {
                    Ok(emitted) => emitted,
                    Err(err) => return Err((state, err)),
                };
                self.complete = true;
                self.pending_input = Some(emitted);
                return Err((state, ExecutorError::Complete));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            let emitted = match Self::encode_success(last_input) {
                Ok(emitted) => emitted,
                Err(err) => return Err((state, err)),
            };
            self.complete = true;
            self.pending_input = Some(emitted);
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            let emitted = match Self::encode_failure(failure) {
                                Ok(emitted) => emitted,
                                Err(err) => return Err((next_state, err)),
                            };
                            self.complete = true;
                            self.pending_input = Some(emitted);
                            return Err((next_state, ExecutorError::Complete));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                if let Some(emitted) = emitted {
                    self.pending_input = Some(emitted);
                } else {
                    self.pending_input = Some(last_input);
                }
                if completed {
                    self.cursor += 1;
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                let emitted = match Self::encode_failure(failure) {
                    Ok(emitted) => emitted,
                    Err(err) => return Err((next_state, err)),
                };
                self.complete = true;
                self.pending_input = Some(emitted);
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, err)) => Err((next_state, err)),
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
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }
        if self.cursor >= self.inner.len() {
            return Err(ExecutorError::Complete);
        }

        let last_input = self
            .pending_input
            .take()
            .ok_or(ExecutorError::NoPendingRequest)?;
        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.complete(state.clone(), completion) {
            Ok((next_state, emitted)) => {
                self.waiting_completion = false;
                let mut last_input = emitted;
                if node.is_complete() {
                    self.cursor += 1;
                }
                let next_state = match settle_subflow_without_progress(
                    &mut self.inner,
                    &mut self.cursor,
                    next_state.clone(),
                    &mut last_input,
                ) {
                    Ok(state) => state,
                    Err(ExecutorError::ActionFailure(failure)) => {
                        let emitted = Self::encode_failure(failure)?;
                        self.complete = true;
                        self.lifecycle.succeed();
                        return Ok((next_state, emitted));
                    }
                    Err(err) => return Err(err),
                };
                if self.cursor >= self.inner.len() {
                    self.complete = true;
                    self.lifecycle.succeed();
                    return Ok((next_state, Self::encode_success(last_input)?));
                }
                Ok((next_state, last_input))
            }
            Err(ExecutorError::ActionFailure(failure)) => {
                self.waiting_completion = false;
                self.complete = true;
                self.lifecycle.succeed();
                Ok((state, Self::encode_failure(failure)?))
            }
            Err(err) => {
                self.waiting_completion = false;
                self.pending_input = Some(last_input);
                Err(err)
            }
        }
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
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                let emitted = match Self::encode_failure(failure) {
                    Ok(emitted) => emitted,
                    Err(err) => return Err((state, err)),
                };
                self.complete = true;
                self.pending_input = Some(emitted);
                return Err((state, ExecutorError::Complete));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            let emitted = match Self::encode_success(last_input) {
                Ok(emitted) => emitted,
                Err(err) => return Err((state, err)),
            };
            self.complete = true;
            self.pending_input = Some(emitted);
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request_executable(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            let emitted = match Self::encode_failure(failure) {
                                Ok(emitted) => emitted,
                                Err(err) => return Err((next_state, err)),
                            };
                            self.complete = true;
                            self.pending_input = Some(emitted);
                            return Err((next_state, ExecutorError::Complete));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                if let Some(emitted) = emitted {
                    self.pending_input = Some(emitted);
                } else {
                    self.pending_input = Some(last_input);
                }
                if completed {
                    self.cursor += 1;
                    return Err((next_state, ExecutorError::Complete));
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                let emitted = match Self::encode_failure(failure) {
                    Ok(emitted) => emitted,
                    Err(err) => return Err((next_state, err)),
                };
                self.complete = true;
                self.pending_input = Some(emitted);
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, err)) => Err((next_state, err)),
        }
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.complete {
            if let Some(emitted) = self.pending_input.take() {
                return Ok((state, Some(emitted), true));
            }
            return Ok((state, None, true));
        }
        if self.waiting_completion {
            return Ok((state, None, false));
        }

        let Some(mut last_input) = self.pending_input.take() else {
            return Ok((state, None, false));
        };
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.complete = true;
                self.lifecycle.succeed();
                return Ok((state, Some(Self::encode_failure(failure)?), true));
            }
            Err(err) => return Err(err),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            return Ok((state, Some(Self::encode_success(last_input)?), true));
        }

        self.pending_input = Some(last_input);
        Ok((state, None, false))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.inner, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.inner));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
        for node in &mut self.inner {
            node.assign_node_ids(next_id);
        }
    }
}

struct TransparentContextErasedFlow<State> {
    lifecycle: LifecycleState,
    node_id: u32,
    inner: DynFlow<State>,
    cursor: usize,
    pending_input: Option<Serialized>,
    waiting_completion: bool,
    complete: bool,
}

impl<State> TransparentContextErasedFlow<State> {
    fn new(inner: DynFlow<State>) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            inner,
            cursor: 0,
            pending_input: None,
            waiting_completion: false,
            complete: false,
        }
    }
}

impl<State> ErasedFlow<State> for TransparentContextErasedFlow<State>
where
    State: Clone + Send + 'static,
{
    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if self.waiting_completion {
            return Err((state, ExecutorError::AwaitingCompletion));
        }
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err((state, ExecutorError::ActionFailure(failure)));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.pending_input = Some(last_input);
            self.lifecycle.succeed();
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            self.lifecycle.fail();
                            return Err((next_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                self.pending_input = Some(emitted.unwrap_or(last_input));
                if completed {
                    self.cursor += 1;
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                self.lifecycle.fail();
                Err((next_state, ExecutorError::ActionFailure(failure)))
            }
            Err((next_state, err)) => Err((next_state, err)),
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
        if !self.waiting_completion {
            return Err(ExecutorError::NoPendingRequest);
        }
        if self.cursor >= self.inner.len() {
            return Err(ExecutorError::Complete);
        }

        let last_input = self
            .pending_input
            .take()
            .ok_or(ExecutorError::NoPendingRequest)?;
        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.complete(state.clone(), completion) {
            Ok((next_state, emitted)) => {
                self.waiting_completion = false;
                let mut last_input = emitted;
                if node.is_complete() {
                    self.cursor += 1;
                }
                let next_state = match settle_subflow_without_progress(
                    &mut self.inner,
                    &mut self.cursor,
                    next_state.clone(),
                    &mut last_input,
                ) {
                    Ok(state) => state,
                    Err(ExecutorError::ActionFailure(failure)) => {
                        self.lifecycle.fail();
                        return Err(ExecutorError::ActionFailure(failure));
                    }
                    Err(err) => return Err(err),
                };
                if self.cursor >= self.inner.len() {
                    self.complete = true;
                    self.lifecycle.succeed();
                }
                Ok((next_state, last_input))
            }
            Err(ExecutorError::ActionFailure(failure)) => {
                self.waiting_completion = false;
                self.lifecycle.fail();
                Err(ExecutorError::ActionFailure(failure))
            }
            Err(err) => {
                self.waiting_completion = false;
                self.pending_input = Some(last_input);
                Err(err)
            }
        }
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
        self.lifecycle.enter();

        let mut last_input = input.clone();
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err((state, ExecutorError::ActionFailure(failure)));
            }
            Err(err) => return Err((state, err)),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.pending_input = Some(last_input);
            self.lifecycle.succeed();
            return Err((state, ExecutorError::Complete));
        }

        let node = self
            .inner
            .get_mut(self.cursor)
            .expect("cursor was checked against steps len");
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        node.set_parent_activation_path(&parent_path);
        match node.request_executable(state, last_input.clone()) {
            Ok((next_state, request)) => {
                self.pending_input = Some(last_input);
                self.waiting_completion = true;
                Ok((next_state, request))
            }
            Err((next_state, ExecutorError::Complete)) => {
                let (next_state, emitted, completed) =
                    match node.try_complete_without_progress(next_state.clone()) {
                        Ok(result) => result,
                        Err(ExecutorError::ActionFailure(failure)) => {
                            self.lifecycle.fail();
                            return Err((next_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err(err) => return Err((next_state, err)),
                    };
                self.pending_input = Some(emitted.unwrap_or(last_input));
                if completed {
                    self.cursor += 1;
                }
                Err((next_state, ExecutorError::Complete))
            }
            Err((next_state, ExecutorError::ActionFailure(failure))) => {
                self.lifecycle.fail();
                Err((next_state, ExecutorError::ActionFailure(failure)))
            }
            Err((next_state, err)) => Err((next_state, err)),
        }
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.complete {
            return Ok((state, self.pending_input.take(), true));
        }
        if self.waiting_completion {
            return Ok((state, None, false));
        }

        let Some(mut last_input) = self.pending_input.take() else {
            return Ok((state, None, false));
        };
        let state = match settle_subflow_without_progress(
            &mut self.inner,
            &mut self.cursor,
            state.clone(),
            &mut last_input,
        ) {
            Ok(state) => state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if self.cursor >= self.inner.len() {
            self.complete = true;
            self.lifecycle.succeed();
            return Ok((state, Some(last_input), true));
        }

        self.pending_input = Some(last_input);
        Ok((state, None, false))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.inner, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.inner));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
        for node in &mut self.inner {
            node.assign_node_ids(next_id);
        }
    }
}

impl<State> ErasedFlow<State> for SelectErasedFlow<State>
where
    State: Clone + Send + 'static,
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

        let envelope: SelectTraceEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "select completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let input = self
            .pending_input
            .take()
            .ok_or(ExecutorError::NoPendingRequest)?;
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        assign_flow_parent_activation_path(&mut self.left, &parent_path);
        assign_flow_parent_activation_path(&mut self.right, &parent_path);
        let emitted = match envelope {
            SelectTraceEnvelope::Left(left_trace) => {
                let (left_state, left_emitted) =
                    replay_subflow_trace(&mut self.left, state, input.clone(), left_trace)?;
                let mut serialized = Vec::with_capacity(1 + left_emitted.len());
                serialized.push(0);
                serialized.extend_from_slice(&left_emitted);
                (left_state, serialized)
            }
            SelectTraceEnvelope::Right(right_trace) => {
                let (right_state, right_emitted) =
                    replay_subflow_trace(&mut self.right, state, input.clone(), right_trace)?;
                let mut serialized = Vec::with_capacity(1 + right_emitted.len());
                serialized.push(1);
                serialized.extend_from_slice(&right_emitted);
                (right_state, serialized)
            }
        };
        if self.suppress_child_lifecycle_replay {
            let _ = take_flow_node_lifecycle_updates(&mut self.left);
            let _ = take_flow_node_lifecycle_updates(&mut self.right);
        }

        self.waiting_completion = false;
        self.complete = true;
        self.lifecycle.succeed();
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

        let payload = SelectRequestEnvelope {
            left: input.clone(),
            right: input.clone(),
        };
        self.lifecycle.enter();
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let left_flow = (self.build_left)();
        let right_flow = (self.build_right)();
        let left_state = state.clone();
        let right_state = state.clone();
        let left_input = input.clone();
        let right_input = input.clone();
        let left_start_node_id = first_flow_node_id(&self.left);
        let right_start_node_id = first_flow_node_id(&self.right);
        let journey_id = self.lifecycle.journey_id();
        let parent_activation_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let live_history = journey_id.map(|_| futures::channel::mpsc::unbounded());
        let live_history_tx = live_history.as_ref().map(|(tx, _)| tx.clone());
        self.suppress_child_lifecycle_replay = live_history_tx.is_some();

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let envelope = run_select_race_to_completion(
                    left_flow,
                    left_state,
                    left_input,
                    left_start_node_id,
                    right_flow,
                    right_state,
                    right_input,
                    right_start_node_id,
                    journey_id,
                    parent_activation_path,
                    live_history_tx,
                )
                .await?;
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        self.pending_input = Some(input);
        let request =
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Select", request, runner);
        let request = match live_history {
            Some((_tx, rx)) => request.with_live_history(rx),
            None => request,
        };
        Ok((state, request))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.left, journey_id);
        assign_flow_journey_id(&mut self.right, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.left));
        updates.extend(take_flow_node_lifecycle_updates(&mut self.right));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
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
    State: Clone + Send + 'static,
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

        let envelope: SelectTraceEnvelope = match completion {
            Ok(bytes) => postcard::from_bytes(&bytes)
                .map_err(|err| ExecutorError::OutputDeserialize(err.to_string()))?,
            Err(bytes) => {
                return Err(ExecutorError::ErrorDeserialize(format!(
                    "select completion envelope failed: {}",
                    String::from_utf8_lossy(&bytes)
                )))
            }
        };

        let input = self
            .pending_input
            .take()
            .ok_or(ExecutorError::NoPendingRequest)?;
        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        assign_flow_parent_activation_path(&mut self.left, &parent_path);
        assign_flow_parent_activation_path(&mut self.right, &parent_path);
        let emitted = match envelope {
            SelectTraceEnvelope::Left(left_trace) => {
                let (left_state, left_emitted) =
                    replay_subflow_trace(&mut self.left, state, input.clone(), left_trace)?;
                let mut serialized = Vec::with_capacity(1 + left_emitted.len());
                serialized.push(0);
                serialized.extend_from_slice(&left_emitted);
                (left_state, serialized)
            }
            SelectTraceEnvelope::Right(right_trace) => {
                let (right_state, right_emitted) =
                    replay_subflow_trace(&mut self.right, state, input.clone(), right_trace)?;
                let mut serialized = Vec::with_capacity(1 + right_emitted.len());
                serialized.push(1);
                serialized.extend_from_slice(&right_emitted);
                (right_state, serialized)
            }
        };
        if self.suppress_child_lifecycle_replay {
            let _ = take_flow_node_lifecycle_updates(&mut self.left);
            let _ = take_flow_node_lifecycle_updates(&mut self.right);
        }

        self.waiting_completion = false;
        self.complete = true;
        self.lifecycle.succeed();
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

        let payload = SelectRequestEnvelope {
            left: input.clone(),
            right: input.clone(),
        };
        self.lifecycle.enter();
        let request = match postcard::to_allocvec(&payload) {
            Ok(request) => request,
            Err(err) => return Err((state, ExecutorError::RequestSerialize(err.to_string()))),
        };
        let left_flow = (self.build_left)();
        let right_flow = (self.build_right)();
        let left_state = state.clone();
        let right_state = state.clone();
        let left_input = input.clone();
        let right_input = input.clone();
        let left_start_node_id = first_flow_node_id(&self.left);
        let right_start_node_id = first_flow_node_id(&self.right);
        let journey_id = self.lifecycle.journey_id();
        let parent_activation_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let live_history = journey_id.map(|_| futures::channel::mpsc::unbounded());
        let live_history_tx = live_history.as_ref().map(|(tx, _)| tx.clone());
        self.suppress_child_lifecycle_replay = live_history_tx.is_some();

        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let envelope = run_select_race_to_completion(
                    left_flow,
                    left_state,
                    left_input,
                    left_start_node_id,
                    right_flow,
                    right_state,
                    right_input,
                    right_start_node_id,
                    journey_id,
                    parent_activation_path,
                    live_history_tx,
                )
                .await?;
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });

        self.waiting_completion = true;
        self.pending_input = Some(input);
        let request =
            ExecutableEffectRequest::new(self.node_id, "jungle_types::Select", request, runner);
        let request = match live_history {
            Some((_tx, rx)) => request.with_live_history(rx),
            None => request,
        };
        Ok((state, request))
    }

    fn is_waiting_completion(&self) -> bool {
        self.waiting_completion
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.left, journey_id);
        assign_flow_journey_id(&mut self.right, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.left));
        updates.extend(take_flow_node_lifecycle_updates(&mut self.right));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
        for node in &mut self.left {
            node.assign_node_ids(next_id);
        }
        for node in &mut self.right {
            node.assign_node_ids(next_id);
        }
    }
}

struct JoinErasedFlow<State> {
    lifecycle: LifecycleState,
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    complete: bool,
    focused_merge: bool,
    merge_left: Box<dyn Fn(&mut State, State) + Send>,
    merge_right: Box<dyn Fn(&mut State, State) + Send>,
    join_input: Option<Serialized>,
    base_state: Option<State>,
    left_branch: Option<JoinBranchProgress<State>>,
    right_branch: Option<JoinBranchProgress<State>>,
    active_request: Option<JoinSide>,
    left_requests_in_flight: usize,
    right_requests_in_flight: usize,
    completed_emitted: Option<Serialized>,
}

impl<State> JoinErasedFlow<State>
where
    State: Clone,
{
    fn new(
        left: DynFlow<State>,
        right: DynFlow<State>,
        focused_merge: bool,
        merge_left: Box<dyn Fn(&mut State, State) + Send>,
        merge_right: Box<dyn Fn(&mut State, State) + Send>,
    ) -> Self {
        Self {
            lifecycle: LifecycleState::default(),
            node_id: 0,
            left,
            right,
            complete: false,
            focused_merge,
            merge_left,
            merge_right,
            join_input: None,
            base_state: None,
            left_branch: None,
            right_branch: None,
            active_request: None,
            left_requests_in_flight: 0,
            right_requests_in_flight: 0,
            completed_emitted: None,
        }
    }

    fn initialize_if_needed(&mut self, state: State, input: Serialized) {
        if self.join_input.is_some() {
            return;
        }

        self.join_input = Some(input.clone());
        self.left_branch = Some(JoinBranchProgress::new(state.clone(), input.clone()));
        if self.focused_merge {
            self.base_state = Some(state.clone());
            self.right_branch = Some(JoinBranchProgress::new(state, input));
        }
    }

    fn ensure_right_branch_started(&mut self) {
        if self.right_branch.is_some() {
            return;
        }

        let right_input = self
            .join_input
            .clone()
            .expect("join input should be initialized before right branch start");
        let right_state = self
            .left_branch
            .as_ref()
            .expect("left branch should exist before right branch start")
            .state
            .clone();
        self.right_branch = Some(JoinBranchProgress::new(right_state, right_input));
    }

    fn current_state(&self, fallback: State) -> State {
        if self.focused_merge {
            let Some(base_state) = self.base_state.as_ref() else {
                return fallback;
            };
            let mut merged = base_state.clone();
            if let Some(left) = self.left_branch.as_ref() {
                (self.merge_left)(&mut merged, left.state.clone());
            }
            if let Some(right) = self.right_branch.as_ref() {
                (self.merge_right)(&mut merged, right.state.clone());
            }
            merged
        } else if let Some(right) = self.right_branch.as_ref() {
            right.state.clone()
        } else if let Some(left) = self.left_branch.as_ref() {
            left.state.clone()
        } else {
            fallback
        }
    }

    fn child_parent_path(&self) -> Vec<u64> {
        self.lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec()
    }

    fn apply_parent_path(&mut self) {
        let parent_path = self.child_parent_path();
        assign_flow_parent_activation_path(&mut self.left, &parent_path);
        assign_flow_parent_activation_path(&mut self.right, &parent_path);
    }

    fn all_complete(&self) -> bool {
        let left_complete = self
            .left_branch
            .as_ref()
            .is_some_and(|branch| branch.complete);
        let right_complete = self
            .right_branch
            .as_ref()
            .is_some_and(|branch| branch.complete);
        left_complete && right_complete
    }

    fn final_emitted(&self) -> Serialized {
        let left = self
            .left_branch
            .as_ref()
            .expect("left branch should exist when join completes");
        let right = self
            .right_branch
            .as_ref()
            .expect("right branch should exist when join completes");
        let mut emitted = Vec::with_capacity(left.last_input.len() + right.last_input.len());
        emitted.extend_from_slice(&left.last_input);
        emitted.extend_from_slice(&right.last_input);
        emitted
    }

    fn mark_complete(&mut self) {
        if self.complete {
            return;
        }
        self.complete = true;
        self.completed_emitted = Some(self.final_emitted());
        self.lifecycle.succeed();
    }

    fn wrap_branch_request(
        side: JoinSide,
        mut request: ExecutableEffectRequest,
    ) -> Result<ExecutableEffectRequest, ExecutorError> {
        let node_id = request.node_id();
        let effect_type = request.effect_type();
        let sleep_effect_type = core::any::type_name::<Sleep>();
        let request_bytes = request.request_bytes().to_vec();
        let live_history = request.take_live_history();
        let suspended_completion = if effect_type == sleep_effect_type {
            let child_completion = Ok(postcard::to_allocvec(&())
                .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?);
            let envelope = match side {
                JoinSide::Left => JoinCompletionEnvelope::Left(child_completion),
                JoinSide::Right => JoinCompletionEnvelope::Right(child_completion),
            };
            Some(
                postcard::to_allocvec(&envelope)
                    .map(Ok)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?,
            )
        } else {
            None
        };
        let runner: EffectRunner = Box::new(move || {
            Box::pin(async move {
                let completion = request.run().await?;
                let envelope = match side {
                    JoinSide::Left => JoinCompletionEnvelope::Left(completion),
                    JoinSide::Right => JoinCompletionEnvelope::Right(completion),
                };
                let bytes = postcard::to_allocvec(&envelope)
                    .map_err(|err| ExecutorError::OutputSerialize(err.to_string()))?;
                Ok(Ok(bytes))
            })
        });
        let wrapped = ExecutableEffectRequest::new(node_id, effect_type, request_bytes, runner);
        let wrapped = if let Some(completion) = suspended_completion {
            wrapped.with_suspended_completion(completion)
        } else {
            wrapped
        };
        Ok(match live_history {
            Some(rx) => wrapped.with_live_history(rx),
            None => wrapped,
        })
    }

    fn settle_branch_without_progress(
        flow: &mut DynFlow<State>,
        branch: &mut JoinBranchProgress<State>,
    ) -> Result<(), ExecutorError> {
        if branch.complete {
            return Ok(());
        }

        branch.state = settle_subflow_without_progress(
            flow,
            &mut branch.cursor,
            branch.state.clone(),
            &mut branch.last_input,
        )?;
        if branch.cursor >= flow.len() {
            branch.complete = true;
        }
        Ok(())
    }

    fn settle_without_external_requests(
        &mut self,
        fallback: State,
    ) -> Result<State, ExecutorError> {
        if self.focused_merge {
            if let Some(left) = self.left_branch.as_mut() {
                Self::settle_branch_without_progress(&mut self.left, left)?;
            }
            if let Some(right) = self.right_branch.as_mut() {
                Self::settle_branch_without_progress(&mut self.right, right)?;
            }
        } else {
            if let Some(left) = self.left_branch.as_mut() {
                Self::settle_branch_without_progress(&mut self.left, left)?;
                if left.complete {
                    self.ensure_right_branch_started();
                }
            }
            if let Some(right) = self.right_branch.as_mut() {
                Self::settle_branch_without_progress(&mut self.right, right)?;
            }
        }

        let current = self.current_state(fallback);
        if self.all_complete() {
            self.mark_complete();
        }
        Ok(current)
    }
}

impl<State> ErasedFlow<State> for JoinErasedFlow<State>
where
    State: Clone + Send + 'static,
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
        mut completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.complete {
            return Err(ExecutorError::Complete);
        }

        let side = if self.focused_merge {
            match &completion {
                Ok(bytes) => {
                    if let Ok(envelope) = postcard::from_bytes::<JoinCompletionEnvelope>(bytes) {
                        match envelope {
                            JoinCompletionEnvelope::Left(child_completion) => {
                                completion = child_completion;
                                self.left_requests_in_flight =
                                    self.left_requests_in_flight.saturating_sub(1);
                                JoinSide::Left
                            }
                            JoinCompletionEnvelope::Right(child_completion) => {
                                completion = child_completion;
                                self.right_requests_in_flight =
                                    self.right_requests_in_flight.saturating_sub(1);
                                JoinSide::Right
                            }
                        }
                    } else if self.left_requests_in_flight + self.right_requests_in_flight == 1 {
                        if self.left_requests_in_flight == 1 {
                            self.left_requests_in_flight = 0;
                            JoinSide::Left
                        } else {
                            self.right_requests_in_flight = 0;
                            JoinSide::Right
                        }
                    } else {
                        return Err(ExecutorError::OutputDeserialize(
                            "focused join completion envelope failed".to_string(),
                        ));
                    }
                }
                Err(bytes) => {
                    if self.left_requests_in_flight + self.right_requests_in_flight == 1 {
                        if self.left_requests_in_flight == 1 {
                            self.left_requests_in_flight = 0;
                            JoinSide::Left
                        } else {
                            self.right_requests_in_flight = 0;
                            JoinSide::Right
                        }
                    } else {
                        return Err(ExecutorError::ErrorDeserialize(format!(
                            "join completion envelope failed: {}",
                            String::from_utf8_lossy(bytes)
                        )));
                    }
                }
            }
        } else {
            self.active_request
                .take()
                .ok_or(ExecutorError::NoPendingRequest)?
        };
        self.apply_parent_path();

        let (current_state, emitted) = match side {
            JoinSide::Left => {
                let left = self
                    .left_branch
                    .as_mut()
                    .expect("left branch should exist while active");
                let (next_state, emitted) = subflow_complete_serialized(
                    &mut self.left,
                    &mut left.cursor,
                    left.state.clone(),
                    &mut left.last_input,
                    completion,
                )?;
                left.state = next_state;
                if left.cursor >= self.left.len() {
                    left.complete = true;
                    if !self.focused_merge {
                        self.ensure_right_branch_started();
                    }
                }
                (self.current_state(state), emitted)
            }
            JoinSide::Right => {
                let right = self
                    .right_branch
                    .as_mut()
                    .expect("right branch should exist while active");
                let (next_state, emitted) = subflow_complete_serialized(
                    &mut self.right,
                    &mut right.cursor,
                    right.state.clone(),
                    &mut right.last_input,
                    completion,
                )?;
                right.state = next_state;
                if right.cursor >= self.right.len() {
                    right.complete = true;
                }
                (self.current_state(state), emitted)
            }
        };

        let current_state = self.settle_without_external_requests(current_state)?;
        if self.complete {
            let emitted = self
                .completed_emitted
                .clone()
                .expect("completed join should store its final emitted tuple");
            return Ok((current_state, emitted));
        }

        Ok((current_state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        if self.complete {
            return Err((state, ExecutorError::Complete));
        }
        if !self.focused_merge && self.active_request.is_some() {
            return Err((state, ExecutorError::AwaitingCompletion));
        }

        self.lifecycle.enter();
        self.initialize_if_needed(state.clone(), input);
        self.apply_parent_path();
        let mut current_state = match self.settle_without_external_requests(state.clone()) {
            Ok(current_state) => current_state,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err((
                    self.current_state(state),
                    ExecutorError::ActionFailure(failure),
                ));
            }
            Err(err) => return Err((self.current_state(state), err)),
        };
        if self.complete {
            return Err((current_state, ExecutorError::Complete));
        }

        loop {
            if self.focused_merge {
                let left_complete = self
                    .left_branch
                    .as_ref()
                    .is_some_and(|branch| branch.complete);
                if !left_complete {
                    let left = self
                        .left_branch
                        .as_mut()
                        .expect("left branch should exist for focused join");
                    match subflow_next_executable_request(
                        &mut self.left,
                        &mut left.cursor,
                        left.state.clone(),
                        &mut left.last_input,
                    ) {
                        Ok((next_state, request)) => {
                            left.state = next_state;
                            self.left_requests_in_flight =
                                self.left_requests_in_flight.saturating_add(1);
                            current_state = self.current_state(current_state);
                            let request = match Self::wrap_branch_request(JoinSide::Left, request) {
                                Ok(request) => request,
                                Err(err) => return Err((current_state, err)),
                            };
                            return Ok((current_state, request));
                        }
                        Err((next_state, ExecutorError::Complete)) => {
                            left.state = next_state;
                            left.complete = true;
                            current_state = self.current_state(current_state);
                            if self.all_complete() {
                                self.mark_complete();
                                return Err((current_state, ExecutorError::Complete));
                            }
                            continue;
                        }
                        Err((next_state, ExecutorError::ActionFailure(failure))) => {
                            left.state = next_state;
                            self.lifecycle.fail();
                            current_state = self.current_state(current_state);
                            return Err((current_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err((next_state, ExecutorError::AwaitingCompletion)) => {
                            left.state = next_state;
                            current_state = self.current_state(current_state);
                        }
                        Err((next_state, err)) => {
                            left.state = next_state;
                            current_state = self.current_state(current_state);
                            return Err((current_state, err));
                        }
                    }
                }

                if !self
                    .right_branch
                    .as_ref()
                    .is_some_and(|branch| branch.complete)
                {
                    let right = self
                        .right_branch
                        .as_mut()
                        .expect("right branch should exist for focused join");
                    match subflow_next_executable_request(
                        &mut self.right,
                        &mut right.cursor,
                        right.state.clone(),
                        &mut right.last_input,
                    ) {
                        Ok((next_state, request)) => {
                            right.state = next_state;
                            self.right_requests_in_flight =
                                self.right_requests_in_flight.saturating_add(1);
                            current_state = self.current_state(current_state);
                            let request = match Self::wrap_branch_request(JoinSide::Right, request)
                            {
                                Ok(request) => request,
                                Err(err) => return Err((current_state, err)),
                            };
                            return Ok((current_state, request));
                        }
                        Err((next_state, ExecutorError::Complete)) => {
                            right.state = next_state;
                            right.complete = true;
                            current_state = self.current_state(current_state);
                            if self.all_complete() {
                                self.mark_complete();
                                return Err((current_state, ExecutorError::Complete));
                            }
                        }
                        Err((next_state, ExecutorError::ActionFailure(failure))) => {
                            right.state = next_state;
                            self.lifecycle.fail();
                            current_state = self.current_state(current_state);
                            return Err((current_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err((next_state, ExecutorError::AwaitingCompletion)) => {
                            right.state = next_state;
                            current_state = self.current_state(current_state);
                        }
                        Err((next_state, err)) => {
                            right.state = next_state;
                            current_state = self.current_state(current_state);
                            return Err((current_state, err));
                        }
                    }
                }

                return Err((current_state, ExecutorError::AwaitingCompletion));
            } else {
                let left_complete = self
                    .left_branch
                    .as_ref()
                    .is_some_and(|branch| branch.complete);
                if !left_complete {
                    let left = self
                        .left_branch
                        .as_mut()
                        .expect("left branch should exist for sequential join");
                    match subflow_next_executable_request(
                        &mut self.left,
                        &mut left.cursor,
                        left.state.clone(),
                        &mut left.last_input,
                    ) {
                        Ok((next_state, request)) => {
                            left.state = next_state;
                            self.active_request = Some(JoinSide::Left);
                            current_state = self.current_state(current_state);
                            return Ok((current_state, request));
                        }
                        Err((next_state, ExecutorError::Complete)) => {
                            left.state = next_state;
                            left.complete = true;
                            self.ensure_right_branch_started();
                            current_state = self.current_state(current_state);
                            continue;
                        }
                        Err((next_state, ExecutorError::ActionFailure(failure))) => {
                            left.state = next_state;
                            self.lifecycle.fail();
                            current_state = self.current_state(current_state);
                            return Err((current_state, ExecutorError::ActionFailure(failure)));
                        }
                        Err((next_state, err)) => {
                            left.state = next_state;
                            current_state = self.current_state(current_state);
                            return Err((current_state, err));
                        }
                    }
                }

                self.ensure_right_branch_started();
                let right = self
                    .right_branch
                    .as_mut()
                    .expect("right branch should exist once left completes");
                match subflow_next_executable_request(
                    &mut self.right,
                    &mut right.cursor,
                    right.state.clone(),
                    &mut right.last_input,
                ) {
                    Ok((next_state, request)) => {
                        right.state = next_state;
                        self.active_request = Some(JoinSide::Right);
                        current_state = self.current_state(current_state);
                        return Ok((current_state, request));
                    }
                    Err((next_state, ExecutorError::Complete)) => {
                        right.state = next_state;
                        right.complete = true;
                        current_state = self.current_state(current_state);
                        if self.all_complete() {
                            self.mark_complete();
                            return Err((current_state, ExecutorError::Complete));
                        }
                    }
                    Err((next_state, ExecutorError::ActionFailure(failure))) => {
                        right.state = next_state;
                        self.lifecycle.fail();
                        current_state = self.current_state(current_state);
                        return Err((current_state, ExecutorError::ActionFailure(failure)));
                    }
                    Err((next_state, err)) => {
                        right.state = next_state;
                        current_state = self.current_state(current_state);
                        return Err((current_state, err));
                    }
                }
            }
        }
    }

    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.complete {
            return Ok((state, self.completed_emitted.take(), true));
        }
        if self.active_request.is_some()
            || self.left_requests_in_flight > 0
            || self.right_requests_in_flight > 0
            || self.join_input.is_none()
        {
            return Ok((state, None, false));
        }

        self.apply_parent_path();
        let current_state = self.settle_without_external_requests(state)?;
        if self.complete {
            return Ok((current_state, self.completed_emitted.take(), true));
        }

        Ok((current_state, None, false))
    }

    fn is_waiting_completion(&self) -> bool {
        self.active_request.is_some()
            || self.left_requests_in_flight > 0
            || self.right_requests_in_flight > 0
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.left, journey_id);
        assign_flow_journey_id(&mut self.right, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.left));
        updates.extend(take_flow_node_lifecycle_updates(&mut self.right));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
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
            lifecycle: LifecycleState::default(),
            node_id: 0,
            should_continue,
            build_body,
            active_body: Vec::new(),
            body_node_id_start: 0,
            body_cursor: 0,
            complete: false,
            deferred_state: None,
            deferred_emitted: None,
            active_control_input: None,
            marker: core::marker::PhantomData,
        }
    }

    fn ensure_iteration_ready(&mut self) {
        if self.active_body.is_empty() {
            self.active_body = (self.build_body)();
            if let Some(journey_id) = self.lifecycle.journey_id {
                assign_flow_journey_id(&mut self.active_body, journey_id);
            }
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
        let mut state = state;
        let mut body_input = input;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            if self.active_body.is_empty() {
                let control_input = self
                    .active_control_input
                    .get_or_insert_with(|| body_input.clone())
                    .clone();
                let (should_continue, branch_input) =
                    match decode_loop_input::<In, _>(&control_input, |carry| {
                        (self.should_continue)(&state, carry)
                    }) {
                        Ok(pair) => pair,
                        Err(err) => return Err((state, err)),
                    };
                if !should_continue {
                    self.complete = true;
                    self.deferred_state = Some(state);
                    self.active_control_input = None;
                    self.lifecycle.succeed();
                    let state = self
                        .deferred_state
                        .take()
                        .expect("deferred state was just set");
                    return Err((state, ExecutorError::Complete));
                }
                body_input = match postcard::to_allocvec(&branch_input) {
                    Ok(branch_input) => branch_input,
                    Err(err) => {
                        return Err((state, ExecutorError::InputSerialize(err.to_string())));
                    }
                };
            }

            self.lifecycle.enter();
            self.ensure_iteration_ready();

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            node.set_parent_activation_path(&parent_path);
            match node.request(state, body_input.clone()) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    let (next_state, emitted, completed) = node
                        .try_complete_without_progress(next_state)
                        .expect("while child inline completion should succeed");
                    state = next_state;
                    if let Some(emitted) = emitted {
                        body_input = emitted.clone();
                        self.deferred_emitted = Some(emitted.clone());
                        if completed && self.body_cursor + 1 >= self.active_body.len() {
                            self.active_control_input = Some(emitted);
                        }
                    }
                    if completed {
                        self.body_cursor += 1;
                        if self.body_cursor >= self.active_body.len() {
                            self.active_body.clear();
                            self.body_cursor = 0;
                        }
                        continue;
                    }
                    return Err((state, ExecutorError::Complete));
                }
                Err((next_state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((next_state, ExecutorError::ActionFailure(failure)));
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

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        node.set_parent_activation_path(&parent_path);
        let (state, emitted) = match node.complete(state, completion) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if node.is_complete() {
            self.body_cursor += 1;
            if self.body_cursor >= self.active_body.len() {
                self.active_body.clear();
                self.body_cursor = 0;
                self.active_control_input = Some(emitted.clone());
            }
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let mut state = state;
        let mut body_input = input;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            if self.active_body.is_empty() {
                let control_input = self
                    .active_control_input
                    .get_or_insert_with(|| body_input.clone())
                    .clone();
                let (should_continue, branch_input) =
                    match decode_loop_input::<In, _>(&control_input, |carry| {
                        (self.should_continue)(&state, carry)
                    }) {
                        Ok(pair) => pair,
                        Err(err) => return Err((state, err)),
                    };
                if !should_continue {
                    self.complete = true;
                    self.deferred_state = Some(state);
                    self.active_control_input = None;
                    self.lifecycle.succeed();
                    let state = self
                        .deferred_state
                        .take()
                        .expect("deferred state was just set");
                    return Err((state, ExecutorError::Complete));
                }
                body_input = match postcard::to_allocvec(&branch_input) {
                    Ok(branch_input) => branch_input,
                    Err(err) => {
                        return Err((state, ExecutorError::InputSerialize(err.to_string())));
                    }
                };
            }

            self.lifecycle.enter();
            self.ensure_iteration_ready();
            enum NodeAdvance<S> {
                Request((S, ExecutableEffectRequest)),
                Completed(S, Option<Serialized>),
                Progress(S, Option<Serialized>),
                Bubble((S, ExecutorError)),
            }

            let advance = {
                let parent_path = self
                    .lifecycle
                    .current_activation_path()
                    .unwrap_or(&[])
                    .to_vec();
                let node = self
                    .active_body
                    .get_mut(self.body_cursor)
                    .expect("body cursor always points to an active body node");
                node.set_parent_activation_path(&parent_path);
                match node.request_executable(state, body_input.clone()) {
                    Ok((next_state, request)) => NodeAdvance::Request((next_state, request)),
                    Err((next_state, ExecutorError::Complete)) => {
                        if node.is_complete() {
                            let (next_state, emitted, _completed) = node
                                .try_complete_without_progress(next_state)
                                .expect("while child post-complete settle should succeed");
                            NodeAdvance::Completed(next_state, emitted)
                        } else {
                            // Handles inline-completable children (e.g. NoEffect) inside While bodies.
                            let (next_state, emitted, completed) = node
                                .try_complete_without_progress(next_state)
                                .expect("while child inline completion should succeed");
                            if completed {
                                NodeAdvance::Completed(next_state, emitted)
                            } else if emitted.is_some() {
                                NodeAdvance::Progress(next_state, emitted)
                            } else {
                                NodeAdvance::Bubble((next_state, ExecutorError::Complete))
                            }
                        }
                    }
                    Err((next_state, ExecutorError::ActionFailure(failure))) => {
                        self.lifecycle.fail();
                        NodeAdvance::Bubble((next_state, ExecutorError::ActionFailure(failure)))
                    }
                    Err((next_state, err)) => NodeAdvance::Bubble((next_state, err)),
                }
            };

            match advance {
                NodeAdvance::Request(ok) => {
                    self.deferred_emitted = None;
                    return Ok(ok);
                }
                NodeAdvance::Completed(next_state, emitted) => {
                    self.body_cursor += 1;
                    let mut iteration_completed = false;
                    if self.body_cursor >= self.active_body.len() {
                        self.active_body.clear();
                        self.body_cursor = 0;
                        iteration_completed = true;
                    }
                    state = next_state;
                    if let Some(emitted) = emitted {
                        if iteration_completed {
                            self.active_control_input = Some(emitted.clone());
                        }
                        self.deferred_emitted = Some(emitted.clone());
                        body_input = emitted;
                    }
                    continue;
                }
                NodeAdvance::Progress(next_state, emitted) => {
                    state = next_state;
                    if let Some(emitted) = emitted {
                        self.deferred_emitted = Some(emitted.clone());
                        body_input = emitted;
                    }
                    continue;
                }
                NodeAdvance::Bubble(err) => return Err(err),
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
        mut state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if let Some(saved) = self.deferred_state.take() {
            let emitted = self.deferred_emitted.take();
            self.active_control_input = None;
            self.lifecycle.succeed();
            return Ok((saved, emitted, true));
        }
        if self.complete {
            let emitted = self.deferred_emitted.take();
            self.active_control_input = None;
            self.lifecycle.succeed();
            return Ok((state, emitted, true));
        }
        let mut emitted_out = self.deferred_emitted.take();

        loop {
            if self.complete {
                return Ok((state, emitted_out, true));
            }
            if self.active_body.is_empty() || self.body_cursor >= self.active_body.len() {
                break;
            }

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            node.set_parent_activation_path(&parent_path);
            if node.is_waiting_completion() {
                break;
            }

            let (next_state, emitted, completed) = match node.try_complete_without_progress(state) {
                Ok(result) => result,
                Err(ExecutorError::ActionFailure(failure)) => {
                    self.lifecycle.fail();
                    return Err(ExecutorError::ActionFailure(failure));
                }
                Err(err) => return Err(err),
            };
            state = next_state;
            if emitted.is_some() {
                emitted_out = emitted;
            }
            if completed {
                self.body_cursor += 1;
                if self.body_cursor >= self.active_body.len() {
                    if let Some(emitted) = emitted_out.clone() {
                        self.active_control_input = Some(emitted);
                    }
                    self.active_body.clear();
                    self.body_cursor = 0;
                    break;
                }
                continue;
            }
            break;
        }

        if let Some(emitted) = emitted_out {
            return Ok((state, Some(emitted), false));
        }
        Ok((state, None, false))
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.active_body, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.active_body));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        self.body_node_id_start = *next_id;
        *next_id = next_id.saturating_add(1);
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

pub trait FlowCarry<State> {
    type In;
    type Out;
}

trait NonEmptyFlowList {}

impl<Head, Tail> NonEmptyFlowList for TList<(Head, Tail)> {}

impl<State> FlowCarry<State> for list::Empty {
    type In = ();
    type Out = ();
}

impl<State, Head> FlowCarry<State> for TList<(Head, list::Empty)>
where
    Head: FlowCarry<State>,
{
    type In = <Head as FlowCarry<State>>::In;
    type Out = <Head as FlowCarry<State>>::Out;
}

impl<State, Head, Tail> FlowCarry<State> for TList<(Head, Tail)>
where
    Tail: NonEmptyFlowList,
    Head: FlowCarry<State>,
    Tail: FlowCarry<State, In = <Head as FlowCarry<State>>::Out>,
{
    type In = <Head as FlowCarry<State>>::In;
    type Out = <Tail as FlowCarry<State>>::Out;
}

impl<T, A> FlowCarry<T::State> for BoundFlowStep<T, A>
where
    T: Animal,
    A: BoundAction<T>,
{
    type In = A::Input;
    type Out = A::Output;
}

impl<State, P, L, R, M> FlowCarry<State> for Conditional<P, L, R, M>
where
    L: FlowCarry<State>,
    R: FlowCarry<State, In = <L as FlowCarry<State>>::In>,
{
    type In = <L as FlowCarry<State>>::In;
    type Out = Either<<L as FlowCarry<State>>::Out, <R as FlowCarry<State>>::Out>;
}

impl<State, In, C, F, M> FlowCarry<State> for While<C, F, M>
where
    F: FlowCarry<State, In = In>,
{
    type In = In;
    type Out = <F as FlowCarry<State>>::Out;
}

impl<State, M, F> FlowCarry<State> for Transparent<M, F>
where
    F: FlowCarry<State>,
{
    type In = <F as FlowCarry<State>>::In;
    type Out = <F as FlowCarry<State>>::Out;
}

impl<State, View, F> FlowCarry<State> for Scoped<View, F>
where
    F: FlowCarry<State>,
{
    type In = <F as FlowCarry<State>>::In;
    type Out = <F as FlowCarry<State>>::Out;
}

impl<State, Carrier, F> FlowCarry<State> for crate::FocusedBoundFlow<Carrier, F>
where
    F: FlowCarry<State>,
{
    type In = <F as FlowCarry<State>>::In;
    type Out = <F as FlowCarry<State>>::Out;
}

impl<State, In, L, R, M> FlowCarry<State> for Select<L, R, M>
where
    L: FlowCarry<State, In = In>,
    R: FlowCarry<State, In = In>,
{
    type In = In;
    type Out = Either<<L as FlowCarry<State>>::Out, <R as FlowCarry<State>>::Out>;
}

impl<State, In, L, R, M> FlowCarry<State> for Join<L, R, M>
where
    L: FlowCarry<State, In = In>,
    R: FlowCarry<State, In = In>,
{
    type In = In;
    type Out = (<L as FlowCarry<State>>::Out, <R as FlowCarry<State>>::Out);
}

impl<State, M, F> FlowCarry<State> for Attempt<F, M>
where
    F: FlowCarry<State>,
{
    type In = <F as FlowCarry<State>>::In;
    type Out = Result<<F as FlowCarry<State>>::Out, Failure>;
}

impl<State> BuildFlow<DynFlow<State>> for list::Empty {
    type Output = DynFlow<State>;

    fn push_steps(steps: DynFlow<State>) -> Self::Output {
        steps
    }
}

macro_rules! dynflow_list_chain {
    ($h:ty) => {
        TList<($h, list::Empty)>
    };
    ($h:ty, $($rest:ty),+) => {
        TList<($h, dynflow_list_chain!($($rest),+))>
    };
}
macro_rules! dynflow_list_chain_tail {
    ($h:ty ; $tail:ty) => {
        TList<($h, $tail)>
    };
    ($h:ty, $($rest:ty),+ ; $tail:ty) => {
        TList<($h, dynflow_list_chain_tail!($($rest),+ ; $tail))>
    };
}

macro_rules! build_flow_len_impl {
    ($h0:ident) => {
        impl<State, $h0> BuildFlow<DynFlow<State>> for dynflow_list_chain!($h0)
        where
            $h0: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
        {
            type Output = DynFlow<State>;

            fn push_steps(steps: DynFlow<State>) -> Self::Output {
                <$h0 as BuildFlow<DynFlow<State>>>::push_steps(steps)
            }
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<State, $h0, $($rest,)+> BuildFlow<DynFlow<State>>
            for dynflow_list_chain!($h0, $($rest),+)
        where
            dynflow_list_chain!($h0, $($rest),+): FlowCarry<State>,
            $h0: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
            dynflow_list_chain!($($rest),+): BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
        {
            type Output = DynFlow<State>;

            fn push_steps(steps: DynFlow<State>) -> Self::Output {
                let steps = <$h0 as BuildFlow<DynFlow<State>>>::push_steps(steps);
                <dynflow_list_chain!($($rest),+) as BuildFlow<DynFlow<State>>>::push_steps(steps)
            }
        }
    };
}
build_flow_len_impl!(H0);
build_flow_len_impl!(H0; H1);
build_flow_len_impl!(H0; H1, H2);
build_flow_len_impl!(H0; H1, H2, H3);
build_flow_len_impl!(H0; H1, H2, H3, H4);
build_flow_len_impl!(H0; H1, H2, H3, H4, H5);
build_flow_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<State, H0, H1, H2, H3, H4, H5, H6, H7, Tail> BuildFlow<DynFlow<State>> for dynflow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail)
where
    dynflow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail): FlowCarry<State>,
    H0: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H1: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H2: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H3: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H4: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H5: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H6: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    H7: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
    Tail: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
{
    type Output = DynFlow<State>;

    fn push_steps(steps: DynFlow<State>) -> Self::Output {
        let steps = <H0 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H1 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H2 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H3 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H4 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H5 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H6 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        let steps = <H7 as BuildFlow<DynFlow<State>>>::push_steps(steps);
        <Tail as BuildFlow<DynFlow<State>>>::push_steps(steps)
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<T, A> BuildFlow<DynFlow<T::State>> for BoundFlowStep<T, A>
where
    T: Animal + 'static,
    A: BoundAction<T> + 'static,
    <A as BoundAction<T>>::Carry: Send + 'static,
    <A as BoundAction<T>>::Effect: Effect<()> + 'static,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Err: Serialize,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Out: DeserializeOwned,
    <<A as BoundAction<T>>::Effect as EffectSchema>::Err: DeserializeOwned,
    A::Input: DeserializeOwned,
    A::Output: Serialize,
{
    type Output = DynFlow<T::State>;

    fn push_steps(mut steps: DynFlow<T::State>) -> Self::Output {
        steps.push(Box::new(TypedErasedStep::<BoundFlowStep<T, A>>::new()));
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
    P: crate::Predicate<(State, <L as ArgputForState<State>>::Carry)> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let choose_left = Box::new(
            |state: &State, input: &<L as ArgputForState<State>>::Carry| {
                <P as crate::Predicate<(State, <L as ArgputForState<State>>::Carry)>>::eval(&(
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
    C: for<'a> crate::Predicate<(&'a State, &'a In)> + 'static,
    In: DeserializeOwned + Serialize + 'static,
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + FlowCarry<State, In = In> + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let _marker = core::marker::PhantomData::<(C, In)>;
        let should_continue = Box::new(|state: &State, input: &In| {
            <C as crate::Predicate<(&State, &In)>>::eval(&(state, input))
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
    State: Clone + Send + 'static,
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let inner = <F as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        steps.push(Box::new(TransparentErasedFlow::<State>::new(inner)));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, L, R, M> BuildFlow<DynFlow<State>> for Select<L, R, M>
where
    State: Clone + Send + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + ArgputForState<State>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>>
        + ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let build_left = Box::new(|| <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new()));
        let build_right = Box::new(|| <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new()));
        steps.push(Box::new(SelectErasedFlow::<State>::new(
            left,
            right,
            build_left,
            build_right,
        )));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, L, R, M> BuildFlow<DynFlow<State>> for Join<L, R, M>
where
    State: Clone + Send + 'static,
    L: BuildFlow<DynFlow<State>, Output = DynFlow<State>>
        + ArgputForState<State>
        + JoinFocusMarker<State>,
    R: BuildFlow<DynFlow<State>, Output = DynFlow<State>>
        + ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>
        + JoinFocusMarker<State>,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let left = <L as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let right = <R as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        let focused_merge =
            <L as JoinFocusMarker<State>>::ENABLED && <R as JoinFocusMarker<State>>::ENABLED;
        let merge_left = Box::new(|target: &mut State, branch_state: State| {
            <L as JoinFocusMarker<State>>::merge_into(target, branch_state);
        });
        let merge_right = Box::new(|target: &mut State, branch_state: State| {
            <R as JoinFocusMarker<State>>::merge_into(target, branch_state);
        });
        steps.push(Box::new(JoinErasedFlow::<State>::new(
            left,
            right,
            focused_merge,
            merge_left,
            merge_right,
        )));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, M, F> BuildFlow<DynFlow<State>> for Attempt<F, M>
where
    State: Clone + Send + 'static,
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>> + FlowCarry<State>,
    <F as FlowCarry<State>>::Out: Serialize + DeserializeOwned + Send + 'static,
{
    type Output = DynFlow<State>;

    fn push_steps(mut steps: DynFlow<State>) -> Self::Output {
        let inner = <F as BuildFlow<DynFlow<State>>>::push_steps(Vec::new());
        steps.push(Box::new(AttemptErasedFlow::<
            State,
            <F as FlowCarry<State>>::Out,
        >::new(inner)));
        steps
    }
}

#[inception::primitive(property = crate::JungleDynFlow)]
impl<State, Carrier, F> BuildFlow<DynFlow<State>> for crate::FocusedBoundFlow<Carrier, F>
where
    F: BuildFlow<DynFlow<State>, Output = DynFlow<State>>,
{
    type Output = DynFlow<State>;

    fn push_steps(steps: DynFlow<State>) -> Self::Output {
        <F as BuildFlow<DynFlow<State>>>::push_steps(steps)
    }
}

#[derive(Clone, Copy)]
enum ActiveContextBranch {
    Left,
    Right,
}

fn encode_conditional_context_emitted(
    active_branch: ActiveContextBranch,
    emitted: Serialized,
) -> Serialized {
    let tagged = match active_branch {
        ActiveContextBranch::Left => Either::Left(emitted),
        ActiveContextBranch::Right => Either::Right(emitted),
    };
    postcard::to_allocvec(&tagged).expect("conditional context emitted envelope should serialize")
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

impl<Context, State> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)> for list::Empty {
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps(input: (Arc<Context>, DynFlow<State>)) -> Self::Output {
        input
    }
}

macro_rules! build_flow_with_context_len_impl {
    ($h0:ident) => {
        impl<Context, State, $h0> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
            for dynflow_list_chain!($h0)
        where
            $h0: BuildFlowWithContext<
                (Arc<Context>, DynFlow<State>),
                Output = (Arc<Context>, DynFlow<State>),
            >,
        {
            type Output = (Arc<Context>, DynFlow<State>);

            fn push_steps(input: (Arc<Context>, DynFlow<State>)) -> Self::Output {
                <$h0 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input)
            }
        }
    };
    ($h0:ident ; $($rest:ident),+) => {
        impl<Context, State, $h0, $($rest,)+> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
            for dynflow_list_chain!($h0, $($rest),+)
        where
            dynflow_list_chain!($h0, $($rest),+): FlowCarry<State>,
            $h0: BuildFlowWithContext<
                (Arc<Context>, DynFlow<State>),
                Output = (Arc<Context>, DynFlow<State>),
            >,
            dynflow_list_chain!($($rest),+): BuildFlowWithContext<
                (Arc<Context>, DynFlow<State>),
                Output = (Arc<Context>, DynFlow<State>),
            >,
        {
            type Output = (Arc<Context>, DynFlow<State>);

            fn push_steps(input: (Arc<Context>, DynFlow<State>)) -> Self::Output {
                let input =
                    <$h0 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(
                        input,
                    );
                <dynflow_list_chain!($($rest),+) as BuildFlowWithContext<
                    (Arc<Context>, DynFlow<State>),
                >>::push_steps(input)
            }
        }
    };
}
build_flow_with_context_len_impl!(H0);
build_flow_with_context_len_impl!(H0; H1);
build_flow_with_context_len_impl!(H0; H1, H2);
build_flow_with_context_len_impl!(H0; H1, H2, H3);
build_flow_with_context_len_impl!(H0; H1, H2, H3, H4);
build_flow_with_context_len_impl!(H0; H1, H2, H3, H4, H5);
build_flow_with_context_len_impl!(H0; H1, H2, H3, H4, H5, H6);
impl<Context, State, H0, H1, H2, H3, H4, H5, H6, H7, Tail>
    BuildFlowWithContext<(Arc<Context>, DynFlow<State>)> for dynflow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail)
where
    dynflow_list_chain_tail!(H0, H1, H2, H3, H4, H5, H6, H7 ; Tail): FlowCarry<State>,
    H0: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H1: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H2: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H3: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H4: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H5: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H6: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    H7: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
    Tail: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps(input: (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let input = <H0 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H1 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H2 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H3 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H4 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H5 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H6 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        let input = <H7 as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input);
        <Tail as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(input)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, T, A> BuildFlowWithContext<(Arc<Context>, DynFlow<T::State>)> for BoundFlowStep<T, A>
where
    Context: Send + Sync + 'static,
    T: Animal + 'static,
    A: BoundAction<T> + 'static,
    <A as BoundAction<T>>::Carry: Send + 'static,
    <A as BoundAction<T>>::Effect: Effect<Context> + 'static,
    <A as BoundAction<T>>::Effect: EffectSchema<
        Context,
        In = <<A as BoundAction<T>>::Effect as EffectSchema>::In,
        Out = <<A as BoundAction<T>>::Effect as EffectSchema>::Out,
        Err = <<A as BoundAction<T>>::Effect as EffectSchema>::Err,
    >,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Out: Serialize,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Err: Serialize,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Out: DeserializeOwned,
    <<A as BoundAction<T>>::Effect as EffectSchema<Context>>::Err: DeserializeOwned,
    A::Input: DeserializeOwned,
    A::Output: Serialize,
{
    type Output = (Arc<Context>, DynFlow<T::State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<T::State>)) -> Self::Output {
        steps.push(Box::new(ContextualTypedErasedStep::<
            Context,
            BoundFlowStep<T, A>,
        >::new(Arc::clone(&context))));
        (context, steps)
    }
}

struct ConditionalContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    lifecycle: LifecycleState,
    node_id: u32,
    left: DynFlow<State>,
    right: DynFlow<State>,
    choose_left: Box<dyn Fn(&State, &In) -> bool + Send>,
    active_branch: Option<ActiveContextBranch>,
    cursor: usize,
    deferred_emitted: Option<Serialized>,
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
            lifecycle: LifecycleState::default(),
            node_id: 0,
            left,
            right,
            choose_left,
            active_branch: None,
            cursor: 0,
            deferred_emitted: None,
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

impl<State: Clone, In> ErasedFlow<State> for ConditionalContextErasedFlow<State, In>
where
    In: DeserializeOwned + Serialize,
{
    fn try_complete_without_progress(
        &mut self,
        state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if self.active_branch.is_some() && self.cursor >= self.branch_len() {
            self.lifecycle.succeed();
            return Ok((state, self.deferred_emitted.take(), true));
        }
        if self.active_branch.is_none() {
            return Ok((state, None, false));
        }

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.set_parent_activation_path(&parent_path);
        if node.is_waiting_completion() {
            return Ok((state, None, false));
        }

        let (state, emitted, completed) = match node.try_complete_without_progress(state) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if !completed {
            return Ok((state, emitted, false));
        }

        self.cursor += 1;
        if self.cursor >= self.branch_len() {
            let active_branch = self
                .active_branch
                .expect("active branch is present while conditional is executing");
            let emitted =
                emitted.map(|bytes| encode_conditional_context_emitted(active_branch, bytes));
            self.deferred_emitted = emitted.clone();
            self.lifecycle.succeed();
            return Ok((state, emitted, true));
        }

        Ok((state, emitted, false))
    }

    fn request(&mut self, state: State, input: Serialized) -> RequestResult<State, Serialized> {
        let (choose_left, branch_input) = if self.active_branch.is_none() {
            match decode_controlled_input::<In, _>(&input, |carry| {
                (self.choose_left)(&state, carry)
            }) {
                Ok(pair) => pair,
                Err(err) => return Err((state, err)),
            }
        } else {
            let request_input = input.clone();
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            return node.request(state, request_input);
        };
        if self.active_branch.is_none() {
            self.lifecycle.enter();
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

        let mut state = state;
        let mut branch_input = branch_input;
        loop {
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            match node.request(state, branch_input.clone()) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    let (next_state, emitted, completed) =
                        match node.try_complete_without_progress(next_state.clone()) {
                            Ok(result) => result,
                            Err(ExecutorError::ActionFailure(failure)) => {
                                self.lifecycle.fail();
                                return Err((next_state, ExecutorError::ActionFailure(failure)));
                            }
                            Err(err) => return Err((next_state, err)),
                        };
                    state = next_state;
                    if let Some(emitted) = emitted {
                        branch_input = emitted;
                    }
                    if completed {
                        self.cursor += 1;
                        if self.cursor >= self.branch_len() {
                            let active_branch = self.active_branch.expect(
                                "active branch is present while conditional context is executing",
                            );
                            let emitted = encode_conditional_context_emitted(
                                active_branch,
                                branch_input.clone(),
                            );
                            self.deferred_emitted = Some(emitted);
                            self.lifecycle.succeed();
                            return Err((state, ExecutorError::Complete));
                        }
                        continue;
                    }
                    return Err((state, ExecutorError::Complete));
                }
                Err((state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((state, ExecutorError::ActionFailure(failure)));
                }
                Err((state, err)) => return Err((state, err)),
            }
        }
    }

    fn complete(
        &mut self,
        state: State,
        completion: SerializedCompletion,
    ) -> Result<(State, Serialized), ExecutorError> {
        if self.cursor >= self.branch_len() {
            return Err(ExecutorError::Complete);
        }

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_node_mut()
            .expect("cursor was checked against active branch length");
        node.set_parent_activation_path(&parent_path);
        let (state, emitted) = match node.complete(state, completion) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        let node_complete = node.is_complete();
        if node_complete {
            self.cursor += 1;
        }
        if node_complete && self.cursor >= self.branch_len() {
            let active_branch = self
                .active_branch
                .expect("active branch is present while conditional is executing");
            let emitted = encode_conditional_context_emitted(active_branch, emitted);
            self.deferred_emitted = Some(emitted.clone());
            self.lifecycle.succeed();
            return Ok((state, emitted));
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let (choose_left, branch_input) = if self.active_branch.is_none() {
            match decode_controlled_input::<In, _>(&input, |carry| {
                (self.choose_left)(&state, carry)
            }) {
                Ok(pair) => pair,
                Err(err) => return Err((state, err)),
            }
        } else {
            let request_input = input.clone();
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            return node.request_executable(state, request_input);
        };
        if self.active_branch.is_none() {
            self.lifecycle.enter();
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

        let mut state = state;
        let mut branch_input = branch_input;
        loop {
            if self.cursor >= self.branch_len() {
                return Err((state, ExecutorError::Complete));
            }

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_node_mut()
                .expect("cursor was checked against active branch length");
            node.set_parent_activation_path(&parent_path);
            match node.request_executable(state, branch_input.clone()) {
                Ok((next_state, request)) => return Ok((next_state, request)),
                Err((next_state, ExecutorError::Complete)) => {
                    let (next_state, emitted, completed) =
                        match node.try_complete_without_progress(next_state.clone()) {
                            Ok(result) => result,
                            Err(ExecutorError::ActionFailure(failure)) => {
                                self.lifecycle.fail();
                                return Err((next_state, ExecutorError::ActionFailure(failure)));
                            }
                            Err(err) => return Err((next_state, err)),
                        };
                    state = next_state;
                    if let Some(emitted) = emitted {
                        branch_input = emitted;
                    }
                    if completed {
                        self.cursor += 1;
                        if self.cursor >= self.branch_len() {
                            let active_branch = self.active_branch.expect(
                                "active branch is present while conditional context is executing",
                            );
                            let emitted = encode_conditional_context_emitted(
                                active_branch,
                                branch_input.clone(),
                            );
                            self.deferred_emitted = Some(emitted);
                            self.lifecycle.succeed();
                            return Err((state, ExecutorError::Complete));
                        }
                        continue;
                    }
                    return Err((state, ExecutorError::Complete));
                }
                Err((state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((state, ExecutorError::ActionFailure(failure)));
                }
                Err((state, err)) => return Err((state, err)),
            }
        }
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

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.left, journey_id);
        assign_flow_journey_id(&mut self.right, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.left));
        updates.extend(take_flow_node_lifecycle_updates(&mut self.right));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        *next_id = next_id.saturating_add(1);
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
    Context: Send + Sync + 'static,
    State: Clone + Send + 'static,
    L: ArgputForState<State>,
    <L as ArgputForState<State>>::Carry: Clone + DeserializeOwned + Serialize + 'static,
    P: crate::Predicate<(State, <L as ArgputForState<State>>::Carry)> + 'static,
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
                <P as crate::Predicate<(State, <L as ArgputForState<State>>::Carry)>>::eval(&(
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
    lifecycle: LifecycleState,
    node_id: u32,
    should_continue: Box<dyn Fn(&State, &In) -> bool + Send>,
    build_body: Box<dyn Fn() -> DynFlow<State> + Send>,
    active_body: DynFlow<State>,
    body_node_id_start: u32,
    body_cursor: usize,
    complete: bool,
    deferred_state: Option<State>,
    deferred_emitted: Option<Serialized>,
    active_control_input: Option<Serialized>,
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
            lifecycle: LifecycleState::default(),
            node_id: 0,
            should_continue,
            build_body,
            active_body: Vec::new(),
            body_node_id_start: 0,
            body_cursor: 0,
            complete: false,
            deferred_state: None,
            deferred_emitted: None,
            active_control_input: None,
            marker: core::marker::PhantomData,
        }
    }

    fn ensure_iteration_ready(&mut self) {
        if self.active_body.is_empty() {
            self.active_body = (self.build_body)();
            if let Some(journey_id) = self.lifecycle.journey_id {
                assign_flow_journey_id(&mut self.active_body, journey_id);
            }
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
        let mut state = state;
        let mut body_input = input;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            if self.active_body.is_empty() {
                let control_input = self
                    .active_control_input
                    .get_or_insert_with(|| body_input.clone())
                    .clone();
                let (should_continue, branch_input) =
                    match decode_loop_input::<In, _>(&control_input, |carry| {
                        (self.should_continue)(&state, carry)
                    }) {
                        Ok(pair) => pair,
                        Err(err) => return Err((state, err)),
                    };
                if !should_continue {
                    self.complete = true;
                    self.deferred_state = Some(state);
                    self.active_control_input = None;
                    self.lifecycle.succeed();
                    let state = self
                        .deferred_state
                        .take()
                        .expect("deferred state was just set");
                    return Err((state, ExecutorError::Complete));
                }
                body_input = match postcard::to_allocvec(&branch_input) {
                    Ok(branch_input) => branch_input,
                    Err(err) => {
                        return Err((state, ExecutorError::InputSerialize(err.to_string())));
                    }
                };
            }

            self.lifecycle.enter();
            self.ensure_iteration_ready();

            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            node.set_parent_activation_path(&parent_path);
            match node.request(state, body_input.clone()) {
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
                Err((next_state, ExecutorError::ActionFailure(failure))) => {
                    self.lifecycle.fail();
                    return Err((next_state, ExecutorError::ActionFailure(failure)));
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

        let parent_path = self
            .lifecycle
            .current_activation_path()
            .unwrap_or(&[])
            .to_vec();
        let node = self
            .active_body
            .get_mut(self.body_cursor)
            .expect("body cursor always points to an active body node");
        node.set_parent_activation_path(&parent_path);
        let (state, emitted) = match node.complete(state, completion) {
            Ok(result) => result,
            Err(ExecutorError::ActionFailure(failure)) => {
                self.lifecycle.fail();
                return Err(ExecutorError::ActionFailure(failure));
            }
            Err(err) => return Err(err),
        };
        if node.is_complete() {
            self.body_cursor += 1;
            if self.body_cursor >= self.active_body.len() {
                self.active_body.clear();
                self.body_cursor = 0;
                self.active_control_input = Some(emitted.clone());
            }
        }
        Ok((state, emitted))
    }

    fn request_executable(
        &mut self,
        state: State,
        input: Serialized,
    ) -> RequestResult<State, ExecutableEffectRequest> {
        let mut state = state;
        let mut body_input = input;
        loop {
            if self.complete {
                return Err((state, ExecutorError::Complete));
            }
            if self.active_body.is_empty() {
                let control_input = self
                    .active_control_input
                    .get_or_insert_with(|| body_input.clone())
                    .clone();
                let (should_continue, branch_input) =
                    match decode_loop_input::<In, _>(&control_input, |carry| {
                        (self.should_continue)(&state, carry)
                    }) {
                        Ok(pair) => pair,
                        Err(err) => return Err((state, err)),
                    };
                if !should_continue {
                    self.complete = true;
                    self.deferred_state = Some(state);
                    self.active_control_input = None;
                    self.lifecycle.succeed();
                    let state = self
                        .deferred_state
                        .take()
                        .expect("deferred state was just set");
                    return Err((state, ExecutorError::Complete));
                }
                body_input = match postcard::to_allocvec(&branch_input) {
                    Ok(branch_input) => branch_input,
                    Err(err) => {
                        return Err((state, ExecutorError::InputSerialize(err.to_string())));
                    }
                };
            }

            self.lifecycle.enter();
            self.ensure_iteration_ready();
            enum NodeAdvance<S> {
                Request((S, ExecutableEffectRequest)),
                Completed(S, Option<Serialized>),
                Progress(S, Option<Serialized>),
                Bubble((S, ExecutorError)),
            }

            let advance = {
                let parent_path = self
                    .lifecycle
                    .current_activation_path()
                    .unwrap_or(&[])
                    .to_vec();
                let node = self
                    .active_body
                    .get_mut(self.body_cursor)
                    .expect("body cursor always points to an active body node");
                node.set_parent_activation_path(&parent_path);
                match node.request_executable(state, body_input.clone()) {
                    Ok((next_state, request)) => NodeAdvance::Request((next_state, request)),
                    Err((next_state, ExecutorError::Complete)) => {
                        if node.is_complete() {
                            let (next_state, emitted, _completed) = node
                                .try_complete_without_progress(next_state)
                                .expect("while child post-complete settle should succeed");
                            NodeAdvance::Completed(next_state, emitted)
                        } else {
                            // Handles inline-completable children (e.g. NoEffect) inside While bodies.
                            let (next_state, emitted, completed) = node
                                .try_complete_without_progress(next_state)
                                .expect("while child inline completion should succeed");
                            if completed {
                                NodeAdvance::Completed(next_state, emitted)
                            } else if emitted.is_some() {
                                NodeAdvance::Progress(next_state, emitted)
                            } else {
                                NodeAdvance::Bubble((next_state, ExecutorError::Complete))
                            }
                        }
                    }
                    Err((next_state, ExecutorError::ActionFailure(failure))) => {
                        self.lifecycle.fail();
                        NodeAdvance::Bubble((next_state, ExecutorError::ActionFailure(failure)))
                    }
                    Err((next_state, err)) => NodeAdvance::Bubble((next_state, err)),
                }
            };

            match advance {
                NodeAdvance::Request(ok) => {
                    self.deferred_emitted = None;
                    return Ok(ok);
                }
                NodeAdvance::Completed(next_state, emitted) => {
                    self.body_cursor += 1;
                    let mut iteration_completed = false;
                    if self.body_cursor >= self.active_body.len() {
                        self.active_body.clear();
                        self.body_cursor = 0;
                        iteration_completed = true;
                    }
                    state = next_state;
                    if let Some(emitted) = emitted {
                        if iteration_completed {
                            self.active_control_input = Some(emitted.clone());
                        }
                        self.deferred_emitted = Some(emitted.clone());
                        body_input = emitted;
                    }
                    continue;
                }
                NodeAdvance::Progress(next_state, emitted) => {
                    state = next_state;
                    if let Some(emitted) = emitted {
                        self.deferred_emitted = Some(emitted.clone());
                        body_input = emitted;
                    }
                    continue;
                }
                NodeAdvance::Bubble(err) => return Err(err),
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
        mut state: State,
    ) -> Result<(State, Option<Serialized>, bool), ExecutorError> {
        if let Some(saved) = self.deferred_state.take() {
            let emitted = self.deferred_emitted.take();
            self.active_control_input = None;
            self.lifecycle.succeed();
            return Ok((saved, emitted, true));
        }
        if self.complete {
            let emitted = self.deferred_emitted.take();
            self.active_control_input = None;
            self.lifecycle.succeed();
            return Ok((state, emitted, true));
        }
        let mut emitted_out = self.deferred_emitted.take();

        loop {
            if self.complete {
                return Ok((state, emitted_out, true));
            }
            if self.active_body.is_empty() || self.body_cursor >= self.active_body.len() {
                break;
            }

            let node = self
                .active_body
                .get_mut(self.body_cursor)
                .expect("body cursor always points to an active body node");
            let parent_path = self
                .lifecycle
                .current_activation_path()
                .unwrap_or(&[])
                .to_vec();
            node.set_parent_activation_path(&parent_path);
            if node.is_waiting_completion() {
                break;
            }

            let (next_state, emitted, completed) = match node.try_complete_without_progress(state) {
                Ok(result) => result,
                Err(ExecutorError::ActionFailure(failure)) => {
                    self.lifecycle.fail();
                    return Err(ExecutorError::ActionFailure(failure));
                }
                Err(err) => return Err(err),
            };
            state = next_state;
            if emitted.is_some() {
                emitted_out = emitted;
            }
            if completed {
                self.body_cursor += 1;
                if self.body_cursor >= self.active_body.len() {
                    if let Some(emitted) = emitted_out.clone() {
                        self.active_control_input = Some(emitted);
                    }
                    self.active_body.clear();
                    self.body_cursor = 0;
                    break;
                }
                continue;
            }
            break;
        }

        if let Some(emitted) = emitted_out {
            return Ok((state, Some(emitted), false));
        }
        Ok((state, None, false))
    }

    fn set_parent_activation_path(&mut self, path: &[u64]) {
        self.lifecycle.set_parent_activation_path(path);
    }

    fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.lifecycle.set_journey_id(journey_id);
        assign_flow_journey_id(&mut self.active_body, journey_id);
    }

    fn current_activation_path(&self) -> Option<&[u64]> {
        self.lifecycle.current_activation_path()
    }

    fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        let mut updates = self.lifecycle.take_updates();
        updates.extend(take_flow_node_lifecycle_updates(&mut self.active_body));
        updates
    }

    fn node_id(&self) -> u32 {
        self.node_id
    }

    fn assign_node_ids(&mut self, next_id: &mut u32) {
        self.node_id = *next_id;
        self.lifecycle.set_node_id(self.node_id);
        self.body_node_id_start = *next_id;
        *next_id = next_id.saturating_add(1);
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
    C: for<'a> crate::Predicate<(&'a State, &'a In)> + 'static,
    In: DeserializeOwned + Serialize + 'static,
    F: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + FlowCarry<State, In = In>
        + 'static,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let _marker = core::marker::PhantomData::<(C, In)>;
        let should_continue = Box::new(|state: &State, input: &In| {
            <C as crate::Predicate<(&State, &In)>>::eval(&(state, input))
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
    Context: Send + Sync + 'static,
    State: Clone + Send + 'static,
    F: BuildFlowWithContext<
        (Arc<Context>, DynFlow<State>),
        Output = (Arc<Context>, DynFlow<State>),
    >,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let (_, inner) = <F as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        steps.push(Box::new(TransparentContextErasedFlow::<State>::new(inner)));
        (context, steps)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, M, F> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)> for Attempt<F, M>
where
    Context: Send + Sync + 'static,
    State: Clone + Send + 'static,
    F: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + FlowCarry<State>,
    <F as FlowCarry<State>>::Out: Serialize + DeserializeOwned + Send + 'static,
{
    type Output = (Arc<Context>, DynFlow<State>);

    fn push_steps((context, mut steps): (Arc<Context>, DynFlow<State>)) -> Self::Output {
        let (_, inner) = <F as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps((
            Arc::clone(&context),
            Vec::new(),
        ));
        steps.push(Box::new(AttemptErasedFlow::<
            State,
            <F as FlowCarry<State>>::Out,
        >::new(inner)));
        (context, steps)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, Carrier, F> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for crate::FocusedBoundFlow<Carrier, F>
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
impl<Context, State, L, R, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>
    for Select<L, R, M>
where
    Context: Send + Sync + 'static,
    State: Clone + Send + 'static,
    L: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + ArgputForState<State>,
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
        let context_for_left = Arc::clone(&context);
        let context_for_right = Arc::clone(&context);
        let build_left = Box::new(move || {
            let (_, flow) = <L as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(
                (Arc::clone(&context_for_left), Vec::new()),
            );
            flow
        });
        let build_right = Box::new(move || {
            let (_, flow) = <R as BuildFlowWithContext<(Arc<Context>, DynFlow<State>)>>::push_steps(
                (Arc::clone(&context_for_right), Vec::new()),
            );
            flow
        });
        steps.push(Box::new(SelectContextErasedFlow::<State>::new(
            left,
            right,
            build_left,
            build_right,
        )));
        (context, steps)
    }
}

#[inception::primitive(property = JungleDynFlowContext)]
impl<Context, State, L, R, M> BuildFlowWithContext<(Arc<Context>, DynFlow<State>)> for Join<L, R, M>
where
    Context: Send + Sync + 'static,
    State: Clone + Send + 'static,
    L: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + ArgputForState<State>
        + JoinFocusMarker<State>,
    R: BuildFlowWithContext<
            (Arc<Context>, DynFlow<State>),
            Output = (Arc<Context>, DynFlow<State>),
        > + ArgputForState<State, Carry = <L as ArgputForState<State>>::Carry>
        + JoinFocusMarker<State>,
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
        let focused_merge =
            <L as JoinFocusMarker<State>>::ENABLED && <R as JoinFocusMarker<State>>::ENABLED;
        let merge_left = Box::new(|target: &mut State, branch_state: State| {
            <L as JoinFocusMarker<State>>::merge_into(target, branch_state);
        });
        let merge_right = Box::new(|target: &mut State, branch_state: State| {
            <R as JoinFocusMarker<State>>::merge_into(target, branch_state);
        });
        steps.push(Box::new(JoinErasedFlow::<State>::new(
            left,
            right,
            focused_merge,
            merge_left,
            merge_right,
        )));
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
    deserialize_exact(&request).map_err(ExecutorError::RequestDeserialize)
}

fn deserialize_emitted<Emitted>(emitted: Serialized) -> Result<Emitted, ExecutorError>
where
    Emitted: DeserializeOwned,
{
    if let Ok(parsed) = deserialize_exact::<Emitted>(&emitted) {
        return Ok(parsed);
    }

    if let Ok(either) = postcard::from_bytes::<Either<Serialized, Serialized>>(&emitted) {
        let mut candidate = Vec::new();
        match either {
            Either::Left(payload) => {
                candidate.reserve(1 + payload.len());
                candidate.push(0_u8);
                candidate.extend_from_slice(&payload);
            }
            Either::Right(payload) => {
                candidate.reserve(1 + payload.len());
                candidate.push(1_u8);
                candidate.extend_from_slice(&payload);
            }
        }
        return deserialize_exact::<Emitted>(&candidate).map_err(ExecutorError::EmitDeserialize);
    }

    deserialize_exact(&emitted).map_err(ExecutorError::EmitDeserialize)
}

fn assign_flow_node_ids<State>(steps: &mut DynFlow<State>) {
    assign_flow_node_ids_starting_at(steps, 0);
}

fn assign_flow_node_ids_starting_at<State>(steps: &mut DynFlow<State>, start_id: u32) {
    let mut next_id = start_id;
    for node in steps {
        node.assign_node_ids(&mut next_id);
    }
}

fn first_flow_node_id<State>(steps: &DynFlow<State>) -> u32 {
    steps.first().map(|node| node.node_id()).unwrap_or(0)
}

fn assign_flow_journey_id<State>(steps: &mut DynFlow<State>, journey_id: uuid::Uuid) {
    for node in steps {
        node.set_journey_id(journey_id);
    }
}

fn take_flow_node_lifecycle_updates<State>(steps: &mut DynFlow<State>) -> Vec<NodeLifecycle> {
    let mut updates = Vec::new();
    for node in steps {
        updates.extend(node.take_node_lifecycle_updates());
    }
    updates
}

fn assign_flow_parent_activation_path<State>(steps: &mut DynFlow<State>, path: &[u64]) {
    for node in steps {
        node.set_parent_activation_path(path);
    }
}

pub struct ContextExecutor<Context, A>
where
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlowWithContext<
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
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlowWithContext<
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
            let (state, emitted, completed) = node.try_complete_without_progress(state)?;
            self.state = Some(state);
            if let Some(emitted) = emitted {
                self.last_emitted = Some(emitted);
            }
            if completed {
                self.cursor += 1;
                continue;
            }
            break;
        }
        Ok(())
    }

    pub fn new(context: Arc<Context>, state: A::State) -> Self {
        let (_, mut steps) = <BoundAnimalJourney<A> as BuildFlowWithContext<(
            Arc<Context>,
            DynFlow<A::State>,
        )>>::push_steps((context, Vec::new()));
        assign_flow_node_ids(&mut steps);
        Self {
            _context: core::marker::PhantomData,
            state: Some(state),
            steps,
            cursor: 0,
            last_emitted: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.cursor >= self.steps.len()
    }

    pub fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        assign_flow_journey_id(&mut self.steps, journey_id);
    }

    pub fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        take_flow_node_lifecycle_updates(&mut self.steps)
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
        let mut input = match self.last_emitted.take() {
            Some(input) => input,
            None => serialize_input(initial_input)?,
        };
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }
            if let Some(emitted) = self.last_emitted.take() {
                input = emitted;
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
                    let (state, emitted, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    let made_progress = emitted.is_some();
                    if let Some(emitted) = emitted {
                        self.last_emitted = Some(emitted);
                    }
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    if made_progress {
                        continue;
                    }
                    self.settle_without_progress()?;
                    if self.cursor >= self.steps.len() {
                        return Err(ExecutorError::Complete);
                    }
                    continue;
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
            match self.next_and_complete_with(initial_input.clone()).await {
                Ok(step_emitted) => emitted.push(step_emitted),
                Err(ExecutorError::Complete) => {
                    self.settle_without_progress()?;
                    if self.is_complete() {
                        break;
                    }
                    return Err(ExecutorError::Complete);
                }
                Err(err) => return Err(err),
            }
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
        let mut input = input;
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }
            if let Some(emitted) = self.last_emitted.take() {
                input = emitted;
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
                    let (state, emitted, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    let made_progress = emitted.is_some();
                    if let Some(emitted) = emitted {
                        self.last_emitted = Some(emitted);
                    }
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    if made_progress {
                        continue;
                    }
                    self.settle_without_progress()?;
                    if self.cursor >= self.steps.len() {
                        return Err(ExecutorError::Complete);
                    }
                    continue;
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
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    state: Option<A::State>,
    steps: DynFlow<A::State>,
    cursor: usize,
    last_emitted: Option<Serialized>,
}

impl<A> ManualExecutor<A>
where
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
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
            let (state, emitted, completed) = node.try_complete_without_progress(state)?;
            self.state = Some(state);
            if let Some(emitted) = emitted {
                self.last_emitted = Some(emitted);
            }
            if completed {
                self.cursor += 1;
                continue;
            }
            break;
        }
        Ok(())
    }

    pub fn new(state: A::State) -> Self {
        let mut steps =
            <BoundAnimalJourney<A> as BuildFlow<DynFlow<A::State>>>::push_steps(Vec::new());
        assign_flow_node_ids(&mut steps);
        Self {
            state: Some(state),
            steps,
            cursor: 0,
            last_emitted: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.cursor >= self.steps.len()
    }

    pub fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        assign_flow_journey_id(&mut self.steps, journey_id);
    }

    pub fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        take_flow_node_lifecycle_updates(&mut self.steps)
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
        let mut input = self.last_emitted.take().unwrap_or(input);
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }
            if let Some(emitted) = self.last_emitted.take() {
                input = emitted;
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
                    let (state, emitted, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    let made_progress = emitted.is_some();
                    if let Some(emitted) = emitted {
                        self.last_emitted = Some(emitted);
                    }
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    if made_progress {
                        continue;
                    }
                    self.settle_without_progress()?;
                    if self.cursor >= self.steps.len() {
                        return Err(ExecutorError::Complete);
                    }
                    continue;
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
        let mut input = self.last_emitted.take().unwrap_or(input);
        loop {
            self.settle_without_progress()?;
            if self.is_complete() {
                return Err(ExecutorError::Complete);
            }
            if let Some(emitted) = self.last_emitted.take() {
                input = emitted;
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
                    let (state, emitted, completed) = node.try_complete_without_progress(state)?;
                    self.state = Some(state);
                    let made_progress = emitted.is_some();
                    if let Some(emitted) = emitted {
                        self.last_emitted = Some(emitted);
                    }
                    if completed {
                        self.cursor += 1;
                        continue;
                    }
                    if made_progress {
                        continue;
                    }
                    self.settle_without_progress()?;
                    if self.cursor >= self.steps.len() {
                        return Err(ExecutorError::Complete);
                    }
                    continue;
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
        self.last_emitted = Some(emitted.clone());
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
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
{
    manual: ManualExecutor<A>,
    last_emitted: Option<Serialized>,
}

impl<A> Executor<A>
where
    A: BoundAnimal,
    BoundAnimalJourney<A>: BuildFlow<DynFlow<A::State>, Output = DynFlow<A::State>>,
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

    pub fn set_journey_id(&mut self, journey_id: uuid::Uuid) {
        self.manual.set_journey_id(journey_id);
    }

    pub fn take_node_lifecycle_updates(&mut self) -> Vec<NodeLifecycle> {
        self.manual.take_node_lifecycle_updates()
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
