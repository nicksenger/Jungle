use crate::Jungle;
use jungle_types::{
    Animal, BuildFlowWithContext, ContextExecutor, DynFlow, ExecutableActionRequest, ExecutorError,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct JungleExecutor<'a, T, A>
where
    T: Jungle + 'static,
    A: Animal,
    A::Journey:
        for<'ctx> BuildFlowWithContext<(&'ctx T, DynFlow<A::State>), Output = DynFlow<A::State>>,
{
    inner: ContextExecutor<'a, T, A>,
}

impl<'a, T, A> JungleExecutor<'a, T, A>
where
    T: Jungle + 'static,
    A: Animal,
    A::Journey:
        for<'ctx> BuildFlowWithContext<(&'ctx T, DynFlow<A::State>), Output = DynFlow<A::State>>,
{
    pub fn new(jungle: &'a T, state: A::State) -> Self {
        Self {
            inner: ContextExecutor::new(jungle, state),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    pub fn state(&self) -> &A::State {
        self.inner.state()
    }

    pub fn state_mut(&mut self) -> &mut A::State {
        self.inner.state_mut()
    }

    pub fn next_request<Request>(&mut self) -> Result<Request, ExecutorError>
    where
        Request: DeserializeOwned + Default + Serialize,
    {
        self.inner.next_request()
    }

    pub fn next_executable_request<Initial>(
        &mut self,
        initial_input: Initial,
    ) -> Result<ExecutableActionRequest, ExecutorError>
    where
        Initial: Serialize,
    {
        self.inner.next_executable_request(initial_input)
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
        self.inner.complete(completion)
    }

    pub fn complete_serialized(
        &mut self,
        completion: Result<Vec<u8>, Vec<u8>>,
    ) -> Result<Vec<u8>, ExecutorError> {
        self.inner.complete_serialized(completion)
    }

    pub async fn next_and_complete_with(
        &mut self,
        initial_input: impl Serialize,
    ) -> Result<Vec<u8>, ExecutorError> {
        self.inner.next_and_complete_with(initial_input).await
    }

    pub async fn advance_to_end_with<Initial>(
        &mut self,
        initial_input: Initial,
    ) -> Result<Vec<Vec<u8>>, ExecutorError>
    where
        Initial: Serialize + Clone,
    {
        self.inner.advance_to_end_with(initial_input).await
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
        self.inner
            .advance_to_end::<Out, Err, Request, Emitted>(completions)
    }

    pub fn into_state(self) -> A::State {
        self.inner.into_state()
    }
}
