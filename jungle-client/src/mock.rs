use crate::{JungleClient, RunnerChannelMessage, RunnerChannelResponse, RunnerChannelRx};
use async_trait::async_trait;
use futures::StreamExt;
use jungle_types::{
    ClaimedAnimalPerturbation, ExecutorError, JourneyStatus, RunnerOut, RunnerStep,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

type HandlerFuture = Pin<Box<dyn Future<Output = Result<(), ExecutorError>> + Send + 'static>>;
type Handler = Arc<dyn Fn(Uuid, Vec<u8>) -> HandlerFuture + Send + Sync + 'static>;
type PollStepHandlerFuture =
    Pin<Box<dyn Future<Output = Result<Option<RunnerStep>, ExecutorError>> + Send + 'static>>;
type PollStepHandler = Arc<dyn Fn() -> PollStepHandlerFuture + Send + Sync + 'static>;
type CreateFlowHandlerFuture =
    Pin<Box<dyn Future<Output = Result<Uuid, ExecutorError>> + Send + 'static>>;
type CreateFlowHandler =
    Arc<dyn Fn(u32, Vec<u8>) -> CreateFlowHandlerFuture + Send + Sync + 'static>;
type FlowStatusHandlerFuture =
    Pin<Box<dyn Future<Output = Result<JourneyStatus, ExecutorError>> + Send + 'static>>;
type FlowStatusHandler = Arc<dyn Fn(Uuid) -> FlowStatusHandlerFuture + Send + Sync + 'static>;
type FlowAppearanceHandlerFuture =
    Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, ExecutorError>> + Send + 'static>>;
type FlowAppearanceHandler =
    Arc<dyn Fn(Uuid) -> FlowAppearanceHandlerFuture + Send + Sync + 'static>;
type FlowCompleteHandlerFuture =
    Pin<Box<dyn Future<Output = Result<(), ExecutorError>> + Send + 'static>>;
type FlowCompleteHandler = Arc<dyn Fn(Uuid) -> FlowCompleteHandlerFuture + Send + Sync + 'static>;
type FlowAppearanceUpdateHandler =
    Arc<dyn Fn(Uuid, Vec<u8>) -> HandlerFuture + Send + Sync + 'static>;
type ClaimPerturbationHandlerFuture = Pin<
    Box<
        dyn Future<Output = Result<Option<ClaimedAnimalPerturbation>, ExecutorError>>
            + Send
            + 'static,
    >,
>;
type ClaimPerturbationHandler =
    Arc<dyn Fn(Uuid) -> ClaimPerturbationHandlerFuture + Send + Sync + 'static>;
type AckPerturbationHandler = Arc<dyn Fn(Uuid, u64) -> HandlerFuture + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockClient {
    on_create_flow: CreateFlowHandler,
    on_flow_status: FlowStatusHandler,
    on_flow_appearance: FlowAppearanceHandler,
    on_flow_appearance_update: FlowAppearanceUpdateHandler,
    on_perturb_animal: Handler,
    on_claim_perturbation: ClaimPerturbationHandler,
    on_ack_perturbation: AckPerturbationHandler,
    on_flow_complete: FlowCompleteHandler,
    on_poll_work: PollStepHandler,
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
                RunnerChannelMessage::History(history) => {
                    let out = match history {
                        RunnerOut::ActionInput { data, uuid } => {
                            self.action_input(uuid, data).await
                        }
                        RunnerOut::ActionSuccessOutput { data, uuid } => {
                            self.action_success_output(uuid, data).await
                        }
                        RunnerOut::ActionFailureOutput { data, uuid } => {
                            self.action_failure_output(uuid, data).await
                        }
                        RunnerOut::Appearance { data, uuid } => {
                            self.animal_appearance_update(uuid, data).await
                        }
                    };
                    out.map(|_| RunnerChannelResponse::Ack)
                }
                RunnerChannelMessage::ClaimAnimalPerturbation { journey_id } => self
                    .claim_animal_perturbation(journey_id)
                    .await
                    .map(RunnerChannelResponse::ClaimedPerturbation),
                RunnerChannelMessage::AckAnimalPerturbation {
                    journey_id,
                    perturbation_id,
                } => self
                    .ack_animal_perturbation(journey_id, perturbation_id)
                    .await
                    .map(|_| RunnerChannelResponse::Ack),
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
    async fn start_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid, ExecutorError> {
        (self.on_create_flow)(ordinal, seed).await
    }

    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError> {
        (self.on_flow_status)(id).await
    }

    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        (self.on_flow_appearance)(id).await
    }

    async fn animal_appearance_update(&self, id: Uuid, data: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_flow_appearance_update)(id, data).await
    }

    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_perturb_animal)(id, payload).await
    }

    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<ClaimedAnimalPerturbation>, ExecutorError> {
        (self.on_claim_perturbation)(id).await
    }

    async fn ack_animal_perturbation(
        &self,
        id: Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError> {
        (self.on_ack_perturbation)(id, perturbation_id).await
    }

    async fn complete_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        (self.on_flow_complete)(id).await
    }

    async fn poll_work(&self) -> Result<Option<RunnerStep>, ExecutorError> {
        (self.on_poll_work)().await
    }

    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_action_input)(id, input).await
    }

    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_action_success_output)(id, output).await
    }

    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError> {
        (self.on_action_failure_output)(id, err).await
    }
}

#[derive(Default)]
pub struct MockClientBuilder {
    on_create_flow: Option<CreateFlowHandler>,
    on_flow_status: Option<FlowStatusHandler>,
    on_flow_appearance: Option<FlowAppearanceHandler>,
    on_flow_appearance_update: Option<FlowAppearanceUpdateHandler>,
    on_perturb_animal: Option<Handler>,
    on_claim_perturbation: Option<ClaimPerturbationHandler>,
    on_ack_perturbation: Option<AckPerturbationHandler>,
    on_flow_complete: Option<FlowCompleteHandler>,
    on_poll_work: Option<PollStepHandler>,
    on_action_input: Option<Handler>,
    on_action_success_output: Option<Handler>,
    on_action_failure_output: Option<Handler>,
}

impl MockClientBuilder {
    pub fn on_create_flow<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(u32, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Uuid, ExecutorError>> + Send + 'static,
    {
        self.on_create_flow = Some(Arc::new(move |ordinal, seed| Box::pin(f(ordinal, seed))));
        self
    }

    pub fn on_poll_work<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<RunnerStep>, ExecutorError>> + Send + 'static,
    {
        self.on_poll_work = Some(Arc::new(move || Box::pin(f())));
        self
    }

    pub fn on_flow_status<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<JourneyStatus, ExecutorError>> + Send + 'static,
    {
        self.on_flow_status = Some(Arc::new(move |id| Box::pin(f(id))));
        self
    }

    pub fn on_flow_appearance<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<Vec<u8>>, ExecutorError>> + Send + 'static,
    {
        self.on_flow_appearance = Some(Arc::new(move |id| Box::pin(f(id))));
        self
    }

    pub fn on_flow_appearance_update<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_flow_appearance_update = Some(Arc::new(move |id, data| Box::pin(f(id, data))));
        self
    }

    pub fn on_perturb_animal<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_perturb_animal = Some(Arc::new(move |id, payload| Box::pin(f(id, payload))));
        self
    }

    pub fn on_claim_perturbation<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<ClaimedAnimalPerturbation>, ExecutorError>>
            + Send
            + 'static,
    {
        self.on_claim_perturbation = Some(Arc::new(move |id| Box::pin(f(id))));
        self
    }

    pub fn on_ack_perturbation<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, u64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_ack_perturbation = Some(Arc::new(move |id, perturbation_id| {
            Box::pin(f(id, perturbation_id))
        }));
        self
    }

    pub fn on_flow_complete<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_flow_complete = Some(Arc::new(move |id| Box::pin(f(id))));
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
        self.on_action_success_output = Some(Arc::new(move |id, output| Box::pin(f(id, output))));
        self
    }

    pub fn on_action_failure_output<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Uuid, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), ExecutorError>> + Send + 'static,
    {
        self.on_action_failure_output = Some(Arc::new(move |id, err| Box::pin(f(id, err))));
        self
    }

    pub fn build(self) -> MockClient {
        let default_handler: Handler = Arc::new(|_, _| Box::pin(async { Ok(()) }));
        let default_create_flow_handler: CreateFlowHandler =
            Arc::new(|_, _| Box::pin(async { Ok(Uuid::new_v4()) }));
        let default_flow_status_handler: FlowStatusHandler =
            Arc::new(|_| Box::pin(async { Ok(JourneyStatus::Alive) }));
        let default_flow_appearance_handler: FlowAppearanceHandler =
            Arc::new(|_| Box::pin(async { Ok(None) }));
        let default_flow_appearance_update_handler: FlowAppearanceUpdateHandler =
            Arc::new(|_, _| Box::pin(async { Ok(()) }));
        let default_claim_perturbation_handler: ClaimPerturbationHandler =
            Arc::new(|_| Box::pin(async { Ok(None) }));
        let default_ack_perturbation_handler: AckPerturbationHandler =
            Arc::new(|_, _| Box::pin(async { Ok(()) }));
        let default_flow_complete_handler: FlowCompleteHandler =
            Arc::new(|_| Box::pin(async { Ok(()) }));
        let default_poll_work_handler: PollStepHandler = Arc::new(|| Box::pin(async { Ok(None) }));
        MockClient {
            on_create_flow: self
                .on_create_flow
                .unwrap_or_else(|| default_create_flow_handler.clone()),
            on_flow_status: self
                .on_flow_status
                .unwrap_or_else(|| default_flow_status_handler.clone()),
            on_flow_appearance: self
                .on_flow_appearance
                .unwrap_or_else(|| default_flow_appearance_handler.clone()),
            on_flow_appearance_update: self
                .on_flow_appearance_update
                .unwrap_or_else(|| default_flow_appearance_update_handler.clone()),
            on_perturb_animal: self
                .on_perturb_animal
                .unwrap_or_else(|| default_handler.clone()),
            on_claim_perturbation: self
                .on_claim_perturbation
                .unwrap_or_else(|| default_claim_perturbation_handler.clone()),
            on_ack_perturbation: self
                .on_ack_perturbation
                .unwrap_or_else(|| default_ack_perturbation_handler.clone()),
            on_flow_complete: self
                .on_flow_complete
                .unwrap_or_else(|| default_flow_complete_handler.clone()),
            on_poll_work: self
                .on_poll_work
                .unwrap_or_else(|| default_poll_work_handler.clone()),
            on_action_input: self
                .on_action_input
                .unwrap_or_else(|| default_handler.clone()),
            on_action_success_output: self
                .on_action_success_output
                .unwrap_or_else(|| default_handler.clone()),
            on_action_failure_output: self.on_action_failure_output.unwrap_or(default_handler),
        }
    }
}
