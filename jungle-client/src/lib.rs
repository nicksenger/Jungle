//! Client contracts for the Jungle workspace.

use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use jungle_types::{ExecutorError, RunnerOut, Work};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

type HandlerFuture = Pin<Box<dyn Future<Output = Result<(), ExecutorError>> + Send + 'static>>;
type Handler = Arc<dyn Fn(Uuid, Vec<u8>) -> HandlerFuture + Send + Sync + 'static>;
type PollWorkHandlerFuture =
    Pin<Box<dyn Future<Output = Result<Option<Work>, ExecutorError>> + Send + 'static>>;
type PollWorkHandler = Arc<dyn Fn() -> PollWorkHandlerFuture + Send + Sync + 'static>;

#[async_trait]
pub trait JungleClient: Send + Sync {
    async fn poll_work(&self) -> Result<Option<Work>, ExecutorError>;
    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_success_output(
        &self,
        id: Uuid,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError>;
    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError>;
}

pub type RunnerChannelTx =
    mpsc::Sender<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;
pub type RunnerChannelRx =
    mpsc::Receiver<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;

pub struct MockClient {
    on_poll_work: PollWorkHandler,
    on_action_input: Handler,
    on_action_success_output: Handler,
    on_action_failure_output: Handler,
}

impl MockClient {
    pub fn builder() -> MockClientBuilder {
        MockClientBuilder::default()
    }

    pub async fn serve_runner_channel(&self, mut rx: RunnerChannelRx) {
        while let Some((message, done)) = rx.next().await {
            let result = match message {
                RunnerOut::ActionInput { data, uuid } => self.action_input(uuid, data).await,
                RunnerOut::ActionSuccessOutput { data, uuid } => {
                    self.action_success_output(uuid, data).await
                }
                RunnerOut::ActionFailureOutput { data, uuid } => {
                    self.action_failure_output(uuid, data).await
                }
            };
            let _ = done.send(result);
        }
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[async_trait]
impl JungleClient for MockClient {
    async fn poll_work(&self) -> Result<Option<Work>, ExecutorError> {
        (self.on_poll_work)().await
    }

    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_action_input)(id, input).await
    }

    async fn action_success_output(
        &self,
        id: Uuid,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        (self.on_action_success_output)(id, output).await
    }

    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_action_failure_output)(id, err).await
    }
}

#[derive(Default)]
pub struct MockClientBuilder {
    on_poll_work: Option<PollWorkHandler>,
    on_action_input: Option<Handler>,
    on_action_success_output: Option<Handler>,
    on_action_failure_output: Option<Handler>,
}

impl MockClientBuilder {
    pub fn on_poll_work<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<Work>, ExecutorError>> + Send + 'static,
    {
        self.on_poll_work = Some(Arc::new(move || Box::pin(f())));
        self
    }

    pub fn on_action_input<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_action_input = Some(Arc::new(move |id, input| Box::pin(f(id, input))));
        self
    }

    pub fn on_action_success_output<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_action_success_output =
            Some(Arc::new(move |id, output| Box::pin(f(id, output))));
        self
    }

    pub fn on_action_failure_output<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_action_failure_output =
            Some(Arc::new(move |id, err| Box::pin(f(id, err))));
        self
    }

    pub fn build(self) -> MockClient {
        let default_handler: Handler = Arc::new(|_, _| Box::pin(async { Ok(()) }));
        let default_poll_work_handler: PollWorkHandler = Arc::new(|| Box::pin(async { Ok(None) }));
        MockClient {
            on_poll_work: self
                .on_poll_work
                .unwrap_or_else(|| default_poll_work_handler.clone()),
            on_action_input: self.on_action_input.unwrap_or_else(|| default_handler.clone()),
            on_action_success_output: self
                .on_action_success_output
                .unwrap_or_else(|| default_handler.clone()),
            on_action_failure_output: self
                .on_action_failure_output
                .unwrap_or(default_handler),
        }
    }
}
