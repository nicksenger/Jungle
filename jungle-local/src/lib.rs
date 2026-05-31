use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use jungle_client::{JourneyUpdateSubscription, JungleClient};
use jungle_server::{JungleServer, Server, ServerError, WireRx, WireTx};
use jungle_types::{
    Animal, AnimalIdValue, BackendError, ClaimedPerturbable, ExecutorError, JourneyStatus,
    OwnerWake, RunnerOut, SupportedAnimal, WireIn, WireOut, Work,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use typosaurus::num::Unsigned;
use uuid::Uuid;

const DEFAULT_NAMESPACE: &str = "default";
const LOCAL_SUBSCRIPTION_LOG_INTERVAL: usize = 512;
const LOCAL_SUBSCRIPTION_LAG_WARN_MS: i64 = 1_000;
const LOCAL_SUBSCRIPTION_BACKLOG_WARN: usize = 256;
const LOCAL_SUBSCRIPTION_SLOW_RECV_WARN_MS: u128 = 20;
const LOCAL_REQUEST_SLOW_BACKEND_HANDLE_WARN_MS: u128 = 100;
const LOCAL_REQUEST_SLOW_RESPONSE_WAIT_WARN_MS: u128 = 50;

static LOCAL_SUBSCRIPTION_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static LOCAL_SUBSCRIPTION_MAX_BACKLOG: AtomicUsize = AtomicUsize::new(0);
static LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS: AtomicUsize = AtomicUsize::new(0);
static LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct LocalClient {
    backend: Arc<dyn JungleServer>,
    namespace: String,
}

#[derive(Clone)]
pub struct LocalClientBuilder {
    namespace: String,
    backend: Option<Arc<dyn JungleServer>>,
}

impl Default for LocalClientBuilder {
    fn default() -> Self {
        Self {
            namespace: DEFAULT_NAMESPACE.to_string(),
            backend: None,
        }
    }
}

impl LocalClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn namespace(mut self, value: impl Into<String>) -> Self {
        self.namespace = value.into();
        self
    }

    pub fn backend<S>(mut self, backend: S) -> Self
    where
        S: JungleServer,
    {
        self.backend = Some(Arc::new(backend));
        self
    }

    pub async fn build(self) -> Result<LocalClient, LocalClientError> {
        let backend = if let Some(backend) = self.backend {
            backend
        } else {
            let server = Server::builder()
                .memory()
                .build()
                .await
                .map_err(|err| LocalClientError::BuildServer(err.to_string()))?;
            Arc::new(server)
        };

        Ok(LocalClient {
            backend,
            namespace: self.namespace,
        })
    }
}

impl LocalClient {
    pub fn builder() -> LocalClientBuilder {
        LocalClientBuilder::new()
    }

    pub fn from_backend<S>(backend: S) -> Self
    where
        S: JungleServer,
    {
        Self {
            backend: Arc::new(backend),
            namespace: DEFAULT_NAMESPACE.to_string(),
        }
    }

    pub fn namespace(mut self, value: impl Into<String>) -> Self {
        self.namespace = value.into();
        self
    }

    pub fn namespace_ref(&self) -> &str {
        &self.namespace
    }

    async fn send_wire_message(&self, input: WireIn) -> Result<WireOut, ExecutorError> {
        let request_kind = wire_in_kind(&input);
        let (req_tx, req_rx) = mpsc::unbounded_channel::<WireIn>();
        let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<Result<WireOut, BackendError>>();

        req_tx.send(input).map_err(|_| {
            ExecutorError::ClientTransport(
                "failed to send local request to in-process backend".to_string(),
            )
        })?;
        drop(req_tx);

        let tx = wire_tx_from_channel(resp_tx);
        let rx = wire_rx_from_channel(req_rx);
        let backend_handle_started_at = Instant::now();
        self.backend
            .handle_request((tx, rx))
            .await
            .map_err(|err| ExecutorError::ClientTransport(err.to_string()))?;
        let backend_handle_elapsed_ms = backend_handle_started_at.elapsed().as_millis();
        if backend_handle_elapsed_ms > LOCAL_REQUEST_SLOW_BACKEND_HANDLE_WARN_MS {
            debug!(
                namespace = %self.namespace,
                request_kind,
                backend_handle_elapsed_ms,
                "local request backend handle was slow"
            );
        }

        let response_wait_started_at = Instant::now();
        let response = resp_rx.recv().await.ok_or_else(|| {
            ExecutorError::ClientTransport(
                "in-process backend returned no response for request".to_string(),
            )
        })?;
        let response_wait_elapsed_ms = response_wait_started_at.elapsed().as_millis();
        if response_wait_elapsed_ms > LOCAL_REQUEST_SLOW_RESPONSE_WAIT_WARN_MS {
            warn!(
                namespace = %self.namespace,
                request_kind,
                response_wait_elapsed_ms,
                "local request response channel wait was slow"
            );
        }

        debug!(
            namespace = %self.namespace,
            request_kind,
            backend_handle_elapsed_ms,
            response_wait_elapsed_ms,
            "local request timing"
        );
        response.map_err(ExecutorError::Backend)
    }

    async fn send_wire_subscription(
        &self,
        input: WireIn,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        let subscribed_journey_id = match &input {
            WireIn::SubscribeJourneyUpdates { journey_id, .. } => Some(*journey_id),
            _ => None,
        };
        let (req_tx, req_rx) = mpsc::unbounded_channel::<WireIn>();
        let (resp_tx, resp_rx) = mpsc::unbounded_channel::<Result<WireOut, BackendError>>();

        req_tx.send(input).map_err(|_| {
            ExecutorError::ClientTransport(
                "failed to send local subscription request to in-process backend".to_string(),
            )
        })?;
        drop(req_tx);

        let backend = Arc::clone(&self.backend);
        let error_tx = resp_tx.clone();
        let namespace = self.namespace.clone();
        tokio::spawn(async move {
            let tx = wire_tx_from_channel(resp_tx);
            let rx = wire_rx_from_channel(req_rx);
            let backend_handle_started_at = Instant::now();
            if let Err(err) = backend.handle_request((tx, rx)).await {
                let _ = error_tx.send(Err(BackendError::Message(err.to_string())));
                warn!(
                    namespace = %namespace,
                    journey_id = ?subscribed_journey_id,
                    backend_handle_elapsed_ms = backend_handle_started_at.elapsed().as_millis(),
                    "local subscription backend task failed"
                );
                return;
            }

            debug!(
                namespace = %namespace,
                journey_id = ?subscribed_journey_id,
                backend_handle_elapsed_ms = backend_handle_started_at.elapsed().as_millis(),
                "local subscription backend task completed"
            );
        });

        let stream = stream::unfold(resp_rx, move |mut rx| {
            let subscribed_journey_id = subscribed_journey_id;
            async move {
                let queued_before_recv = rx.len();
                update_max_usize(&LOCAL_SUBSCRIPTION_MAX_BACKLOG, queued_before_recv);
                let recv_started_at = Instant::now();
                let next = rx.recv().await?;
                let recv_wait_ms = recv_started_at.elapsed().as_millis();
                update_max_usize(
                    &LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS,
                    usize::try_from(recv_wait_ms).unwrap_or(usize::MAX),
                );
                let mapped = match next {
                    Ok(WireOut::JourneyUpdate(update)) => {
                        let event_count =
                            LOCAL_SUBSCRIPTION_EVENT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                        let event_age_ms = current_unix_ms().saturating_sub(update.event_unix_ms);
                        update_max_usize(
                            &LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS,
                            usize::try_from(event_age_ms.max(0)).unwrap_or(usize::MAX),
                        );
                        if event_age_ms > LOCAL_SUBSCRIPTION_LAG_WARN_MS {
                            warn!(
                                journey_id = ?subscribed_journey_id,
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                queued_before_recv,
                                recv_wait_ms,
                                max_event_age_ms = LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS.load(Ordering::Relaxed),
                                max_backlog = LOCAL_SUBSCRIPTION_MAX_BACKLOG.load(Ordering::Relaxed),
                                max_recv_wait_ms = LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS.load(Ordering::Relaxed),
                                "local subscription received stale journey update"
                            );
                        } else if recv_wait_ms > LOCAL_SUBSCRIPTION_SLOW_RECV_WARN_MS {
                            debug!(
                                journey_id = ?subscribed_journey_id,
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                queued_before_recv,
                                recv_wait_ms,
                                max_event_age_ms = LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS.load(Ordering::Relaxed),
                                max_backlog = LOCAL_SUBSCRIPTION_MAX_BACKLOG.load(Ordering::Relaxed),
                                max_recv_wait_ms = LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS.load(Ordering::Relaxed),
                                "local subscription recv wait was unexpectedly slow"
                            );
                        } else if queued_before_recv >= LOCAL_SUBSCRIPTION_BACKLOG_WARN {
                            warn!(
                                journey_id = ?subscribed_journey_id,
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                queued_before_recv,
                                recv_wait_ms,
                                max_event_age_ms = LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS.load(Ordering::Relaxed),
                                max_backlog = LOCAL_SUBSCRIPTION_MAX_BACKLOG.load(Ordering::Relaxed),
                                max_recv_wait_ms = LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS.load(Ordering::Relaxed),
                                "local subscription queue backlog is growing"
                            );
                        } else if event_count % LOCAL_SUBSCRIPTION_LOG_INTERVAL == 0 {
                            debug!(
                                journey_id = ?subscribed_journey_id,
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                queued_before_recv,
                                recv_wait_ms,
                                max_event_age_ms = LOCAL_SUBSCRIPTION_MAX_EVENT_AGE_MS.load(Ordering::Relaxed),
                                max_backlog = LOCAL_SUBSCRIPTION_MAX_BACKLOG.load(Ordering::Relaxed),
                                max_recv_wait_ms = LOCAL_SUBSCRIPTION_MAX_RECV_WAIT_MS.load(Ordering::Relaxed),
                                "local subscription heartbeat"
                            );
                        }
                        Ok(update)
                    }
                    Ok(other) => Err(ExecutorError::ClientTransport(format!(
                        "unexpected response for journey update subscription: {other:?}"
                    ))),
                    Err(err) => Err(ExecutorError::Backend(err)),
                };
                Some((mapped, rx))
            }
        });

        Ok(JourneyUpdateSubscription::from_stream(stream))
    }
}

fn current_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

fn wire_in_kind(input: &WireIn) -> &'static str {
    match input {
        WireIn::CreateJourney { .. } => "CreateJourney",
        WireIn::JourneyHistory(..) => "JourneyHistory",
        WireIn::JourneyStatus(..) => "JourneyStatus",
        WireIn::SubscribeJourneyUpdates { .. } => "SubscribeJourneyUpdates",
        WireIn::AnimalAppearance(..) => "AnimalAppearance",
        WireIn::PerturbAnimal { .. } => "PerturbAnimal",
        WireIn::ClaimPerturbable(..) => "ClaimPerturbable",
        WireIn::AckPerturbable { .. } => "AckPerturbable",
        WireIn::HeartbeatJourneyLease { .. } => "HeartbeatJourneyLease",
        WireIn::PollOwnerWake { .. } => "PollOwnerWake",
        WireIn::ScheduleSleep { .. } => "ScheduleSleep",
        WireIn::JourneyComplete(..) => "JourneyComplete",
        WireIn::JourneyDead(..) => "JourneyDead",
        WireIn::PollStep { .. } => "PollStep",
        WireIn::WaitForWorkerWake { .. } => "WaitForWorkerWake",
        WireIn::PollTimers => "PollTimers",
        WireIn::HistoryEvent { .. } => "HistoryEvent",
    }
}

fn update_max_usize(max_value: &AtomicUsize, candidate: usize) {
    let mut current = max_value.load(Ordering::Relaxed);
    while candidate > current {
        match max_value.compare_exchange_weak(
            current,
            candidate,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(updated) => current = updated,
        }
    }
}

#[derive(Debug, Error)]
pub enum LocalClientError {
    #[error("failed to build in-memory local server backend: {0}")]
    BuildServer(String),
}

#[async_trait]
impl JungleClient for LocalClient {
    async fn start_journey<A>(&self, seed: Vec<u8>) -> Result<Uuid, ExecutorError>
    where
        Self: Sized,
        A: Animal,
        A::Id: AnimalIdValue,
        A::Generation: Unsigned,
    {
        let response = self
            .send_wire_message(WireIn::CreateJourney {
                namespace: self.namespace.clone(),
                animal_id: <A::Id as AnimalIdValue>::U32,
                generation: <A::Generation as Unsigned>::U32,
                seed,
            })
            .await?;

        match response {
            WireOut::JourneyCreated(journey_id) => Ok(journey_id),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-created response for start_journey".to_string(),
            )),
        }
    }

    async fn journey_history(&self, id: Uuid) -> Result<Vec<RunnerOut>, ExecutorError> {
        let response = self.send_wire_message(WireIn::JourneyHistory(id)).await?;
        match response {
            WireOut::JourneyHistory(history) => Ok(history),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-history response for journey_history".to_string(),
            )),
        }
    }

    async fn subscribe_step_updates(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        self.send_wire_subscription(WireIn::SubscribeJourneyUpdates {
            journey_id,
            after_sequence_id,
        })
        .await
    }

    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError> {
        let response = self.send_wire_message(WireIn::JourneyStatus(id)).await?;
        match response {
            WireOut::JourneyStatus(status) => Ok(status),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-status response for journey_details".to_string(),
            )),
        }
    }

    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        let response = self.send_wire_message(WireIn::AnimalAppearance(id)).await?;
        match response {
            WireOut::AnimalAppearance(appearance) => Ok(appearance),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-animal-appearance response for animal_appearance".to_string(),
            )),
        }
    }

    async fn animal_appearance_update(&self, id: Uuid, data: Vec<u8>) -> Result<(), ExecutorError> {
        let event_unix_ms = Utc::now().timestamp_millis();
        let response = self
            .send_wire_message(WireIn::HistoryEvent {
                event: RunnerOut::Appearance { data, uuid: id },
                event_unix_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for animal_appearance_update".to_string(),
            )),
        }
    }

    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PerturbAnimal {
                journey_id: id,
                data: payload,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for perturb_animal".to_string(),
            )),
        }
    }

    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>, ExecutorError> {
        let response = self.send_wire_message(WireIn::ClaimPerturbable(id)).await?;
        match response {
            WireOut::ClaimedPerturbable(claimed) => Ok(claimed),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected response for claim_animal_perturbation".to_string(),
            )),
        }
    }

    async fn ack_animal_perturbation(
        &self,
        id: Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::AckPerturbable {
                journey_id: id,
                perturbation_id,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for ack_animal_perturbation".to_string(),
            )),
        }
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::HeartbeatJourneyLease {
                journey_id,
                owner_id,
                lease_ttl_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for heartbeat_journey_lease".to_string(),
            )),
        }
    }

    async fn poll_owner_wake(&self, owner_id: Uuid) -> Result<Option<OwnerWake>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PollOwnerWake { owner_id })
            .await?;
        match response {
            WireOut::OwnerWake(wake) => Ok(wake),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected response for poll_owner_wake".to_string(),
            )),
        }
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::ScheduleSleep {
                journey_id,
                timer_id,
                wake_at_unix_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for schedule_sleep_timer".to_string(),
            )),
        }
    }

    async fn complete_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        let response = self.send_wire_message(WireIn::JourneyComplete(id)).await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for complete_journey".to_string(),
            )),
        }
    }

    async fn dead_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        let response = self.send_wire_message(WireIn::JourneyDead(id)).await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for dead_journey".to_string(),
            )),
        }
    }

    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError> {
        let response = self.send_wire_message(WireIn::PollTimers).await?;
        match response {
            WireOut::Ack => Ok(Some(())),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for poll_timers".to_string(),
            )),
        }
    }

    async fn poll_work(
        &self,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PollStep {
                namespace: self.namespace.clone(),
                supported_animals,
            })
            .await?;
        match response {
            WireOut::NoAvailableSteps => Ok(None),
            WireOut::PendingStep(work) => Ok(Some(work)),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected response for poll_work".to_string(),
            )),
        }
    }

    async fn wait_for_worker_wake(
        &self,
        owner_id: Uuid,
        supported_animals: Vec<SupportedAnimal>,
        timeout: Duration,
    ) -> Result<(), ExecutorError> {
        let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
        let response = self
            .send_wire_message(WireIn::WaitForWorkerWake {
                owner_id,
                namespace: self.namespace.clone(),
                supported_animals,
                timeout_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected response for wait_for_worker_wake".to_string(),
            )),
        }
    }

    async fn effect_input(
        &self,
        id: Uuid,
        node_id: u32,
        input: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        let event_unix_ms = Utc::now().timestamp_millis();
        let response = self
            .send_wire_message(WireIn::HistoryEvent {
                event: RunnerOut::EffectInput {
                    node_id,
                    data: input,
                    uuid: id,
                },
                event_unix_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for effect_input".to_string(),
            )),
        }
    }

    async fn effect_success_output(
        &self,
        id: Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        let event_unix_ms = Utc::now().timestamp_millis();
        let response = self
            .send_wire_message(WireIn::HistoryEvent {
                event: RunnerOut::EffectSuccessOutput {
                    node_id,
                    data: output,
                    uuid: id,
                },
                event_unix_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for effect_success_output".to_string(),
            )),
        }
    }

    async fn effect_failure_output(
        &self,
        id: Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        let event_unix_ms = Utc::now().timestamp_millis();
        let response = self
            .send_wire_message(WireIn::HistoryEvent {
                event: RunnerOut::EffectFailureOutput {
                    node_id,
                    data: err,
                    uuid: id,
                },
                event_unix_ms,
            })
            .await?;
        match response {
            WireOut::Ack => Ok(()),
            _ => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for effect_failure_output".to_string(),
            )),
        }
    }
}

fn wire_tx_from_channel(tx: mpsc::UnboundedSender<Result<WireOut, BackendError>>) -> WireTx {
    Box::pin(futures::sink::unfold(
        tx,
        |tx, message: Result<WireOut, BackendError>| async move {
            tx.send(message).map_err(|_| {
                ServerError::Backend(BackendError::Message(
                    "in-process wire response receiver dropped".to_string(),
                ))
            })?;
            Ok(tx)
        },
    ))
}

fn wire_rx_from_channel(rx: mpsc::UnboundedReceiver<WireIn>) -> WireRx {
    Box::pin(stream::unfold(rx, |mut rx| async move {
        let next = rx.recv().await?;
        Some((next, rx))
    }))
}
