use crate::{JourneyHandle, JungleClient};
use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use jungle_types::{
    Animal, AnimalIdValue, AnimalSet, Animals, BackendError, ClaimedPerturbable, Ecosystem,
    ExecutorError, JourneyRecord, JourneyReplayPage, JourneyStatus, JourneyUpdateEvent, OwnerWake,
    RunnerOut, StripAnimalHeaders, SupportedAnimal, WireIn, WireOut, Work,
};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use std::fs;
use std::io;
use std::marker::PhantomData;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncRead, ReadBuf};
use tracing::{error, info};
use typosaurus::collections::sp::{FlattenNodes, SPFlatten};
use typosaurus::collections::Container;
use typosaurus::num::Unsigned;
use uuid::Uuid;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

#[derive(Debug, Clone)]
pub enum StepUpdate {
    Started {
        sequence_id: u64,
        journey_id: Uuid,
        node_id: u32,
        data: Vec<u8>,
    },
    Succeeded {
        sequence_id: u64,
        journey_id: Uuid,
        node_id: u32,
        data: Vec<u8>,
    },
    Failed {
        sequence_id: u64,
        journey_id: Uuid,
        node_id: u32,
        data: Vec<u8>,
    },
}

pub struct JourneyUpdateSubscription {
    inner: Pin<Box<dyn Stream<Item = Result<JourneyUpdateEvent, ExecutorError>> + Send + 'static>>,
}

impl JourneyUpdateSubscription {
    pub fn from_stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<JourneyUpdateEvent, ExecutorError>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }
}

impl Stream for JourneyUpdateSubscription {
    type Item = Result<JourneyUpdateEvent, ExecutorError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

struct QuinnJourneyUpdateStream {
    recv: quinn::RecvStream,
    frame_state: SubscriptionFrameState,
    closed: bool,
}

enum SubscriptionFrameState {
    ReadingFrameLen { bytes: [u8; 4], read: usize },
    ReadingPayload { payload: Vec<u8>, read: usize },
}

impl Default for SubscriptionFrameState {
    fn default() -> Self {
        Self::ReadingFrameLen {
            bytes: [0_u8; 4],
            read: 0,
        }
    }
}

impl QuinnJourneyUpdateStream {
    fn decode_update(payload: &[u8]) -> Result<JourneyUpdateEvent, ExecutorError> {
        let response: Result<WireOut, BackendError> =
            postcard::from_bytes(payload).map_err(|err| {
                ExecutorError::ClientTransport(format!("failed to decode wire output: {err}"))
            })?;

        match response {
            Ok(WireOut::JourneyUpdate(update)) => Ok(update),
            Ok(other) => Err(ExecutorError::ClientTransport(format!(
                "unexpected response for journey update subscription: {other:?}"
            ))),
            Err(err) => Err(ExecutorError::Backend(err)),
        }
    }
}

impl Stream for QuinnJourneyUpdateStream {
    type Item = Result<JourneyUpdateEvent, ExecutorError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        let this = self.as_mut().get_mut();

        loop {
            match &mut this.frame_state {
                SubscriptionFrameState::ReadingFrameLen { bytes, read } => {
                    let mut read_buf = ReadBuf::new(&mut bytes[*read..]);
                    match std::pin::Pin::new(&mut this.recv).poll_read(cx, &mut read_buf) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.closed = true;
                            return Poll::Ready(Some(Err(ExecutorError::ClientTransport(
                                err.to_string(),
                            ))));
                        }
                        Poll::Ready(Ok(())) => {
                            let just_read = read_buf.filled().len();
                            if just_read == 0 {
                                if *read == 0 {
                                    this.closed = true;
                                    return Poll::Ready(None);
                                }
                                this.closed = true;
                                return Poll::Ready(Some(Err(ExecutorError::ClientTransport(
                                    "unexpected EOF while reading journey update frame length"
                                        .to_string(),
                                ))));
                            }
                            *read += just_read;
                            if *read < 4 {
                                continue;
                            }

                            let frame_len = u32::from_be_bytes(*bytes) as usize;
                            this.frame_state = SubscriptionFrameState::ReadingPayload {
                                payload: vec![0_u8; frame_len],
                                read: 0,
                            };
                        }
                    }
                }
                SubscriptionFrameState::ReadingPayload { payload, read } => {
                    let mut read_buf = ReadBuf::new(&mut payload[*read..]);
                    match std::pin::Pin::new(&mut this.recv).poll_read(cx, &mut read_buf) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => {
                            this.closed = true;
                            return Poll::Ready(Some(Err(ExecutorError::ClientTransport(
                                err.to_string(),
                            ))));
                        }
                        Poll::Ready(Ok(())) => {
                            let just_read = read_buf.filled().len();
                            if just_read == 0 {
                                this.closed = true;
                                return Poll::Ready(Some(Err(ExecutorError::ClientTransport(
                                    "unexpected EOF while reading journey update payload"
                                        .to_string(),
                                ))));
                            }

                            *read += just_read;
                            if *read < payload.len() {
                                continue;
                            }

                            let decode_result = QuinnJourneyUpdateStream::decode_update(payload);
                            this.frame_state = SubscriptionFrameState::default();
                            return Poll::Ready(Some(decode_result));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientBuilder<J = DefaultJungle> {
    keylog: bool,
    ca: Option<PathBuf>,
    namespace: String,
    rebind: bool,
    bind: SocketAddr,
    remote: Option<SocketAddr>,
    server_name: Option<String>,
    _jungle: PhantomData<fn() -> J>,
}

pub struct DefaultAnimals;
impl Animals for DefaultAnimals {
    type List = typosaurus::collections::list::Empty;
}

pub struct DefaultJungle;
impl Ecosystem for DefaultJungle {
    const NAME: &'static str = "default";
    type Animals = DefaultAnimals;
}

impl<J> Default for ClientBuilder<J> {
    fn default() -> Self {
        Self {
            keylog: false,
            ca: None,
            namespace: "default".to_string(),
            rebind: false,
            bind: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
            remote: None,
            server_name: None,
            _jungle: PhantomData,
        }
    }
}

impl<J> ClientBuilder<J> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keylog(mut self, enabled: bool) -> Self {
        self.keylog = enabled;
        self
    }

    pub fn ca(mut self, ca_path: impl Into<PathBuf>) -> Self {
        self.ca = Some(ca_path.into());
        self
    }

    pub fn rebind(mut self, enabled: bool) -> Self {
        self.rebind = enabled;
        self
    }

    pub fn namespace(mut self, value: impl Into<String>) -> Self {
        self.namespace = value.into();
        self
    }

    pub fn ecosystem<TNextJungle: Ecosystem>(self) -> ClientBuilder<TNextJungle> {
        ClientBuilder {
            keylog: self.keylog,
            ca: self.ca,
            namespace: TNextJungle::NAME.to_string(),
            rebind: self.rebind,
            bind: self.bind,
            remote: self.remote,
            server_name: self.server_name,
            _jungle: PhantomData,
        }
    }

    pub fn bind(mut self, bind: SocketAddr) -> Self {
        self.bind = bind;
        self
    }

    pub fn remote(mut self, remote: SocketAddr) -> Self {
        self.remote = Some(remote);
        self
    }

    pub fn server_name(mut self, server_name: impl Into<String>) -> Self {
        self.server_name = Some(server_name.into());
        self
    }

    pub async fn build(self) -> ClientResult<Client<J>> {
        let remote = self.remote.ok_or(ClientError::MissingRemote)?;
        let server_name = self.server_name.ok_or(ClientError::MissingServerName)?;

        let mut roots = rustls::RootCertStore::empty();
        if let Some(ca_path) = self.ca {
            roots
                .add(CertificateDer::from(
                    fs::read(ca_path).map_err(ClientError::ReadCa)?,
                ))
                .map_err(ClientError::AddTrustedCert)?;
        } else {
            let dirs = directories_next::ProjectDirs::from("org", "jungle", "jungle-server")
                .ok_or(ClientError::ProjectDirsUnavailable)?;
            match fs::read(dirs.data_local_dir().join("cert.der")) {
                Ok(cert) => {
                    roots
                        .add(CertificateDer::from(cert))
                        .map_err(ClientError::AddTrustedCert)?;
                }
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                    info!("local server certificate not found");
                }
                Err(e) => {
                    error!("failed to open local server certificate: {e}");
                }
            }
        }

        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
        if self.keylog {
            client_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
        }

        let client_config = quinn::ClientConfig::new(Arc::new(
            QuicClientConfig::try_from(client_crypto).map_err(ClientError::QuicClientConfig)?,
        ));
        let mut endpoint = quinn::Endpoint::client(self.bind).map_err(ClientError::BindEndpoint)?;
        endpoint.set_default_client_config(client_config);

        let connecting = endpoint
            .connect(remote, &server_name)
            .map_err(ClientError::Connect)?;
        let conn = connecting.await.map_err(ClientError::Connection)?;

        if self.rebind {
            let socket =
                UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0)).map_err(ClientError::RebindSocket)?;
            endpoint
                .rebind(socket)
                .map_err(ClientError::RebindEndpoint)?;
        }

        Ok(Client {
            transport: Arc::new(ClientTransport { endpoint, conn }),
            namespace: self.namespace,
            _jungle: PhantomData,
        })
    }
}

pub struct Client<J = DefaultJungle> {
    transport: Arc<ClientTransport>,
    namespace: String,
    _jungle: PhantomData<fn() -> J>,
}

struct ClientTransport {
    endpoint: quinn::Endpoint,
    conn: quinn::Connection,
}

impl Drop for ClientTransport {
    fn drop(&mut self) {
        self.conn.close(0u32.into(), b"done");
        self.endpoint.close(0u32.into(), b"done");
    }
}

impl<J> Clone for Client<J> {
    fn clone(&self) -> Self {
        Self {
            transport: Arc::clone(&self.transport),
            namespace: self.namespace.clone(),
            _jungle: PhantomData,
        }
    }
}

impl Client<DefaultJungle> {
    pub fn builder() -> ClientBuilder<DefaultJungle> {
        ClientBuilder::default()
    }
}

impl<J> Client<J> {
    async fn send_wire_message(&self, input: WireIn) -> ClientResult<WireOut> {
        let (mut tx, mut rx) = self
            .transport
            .conn
            .open_bi()
            .await
            .map_err(ClientError::OpenStream)?;

        let payload = postcard::to_allocvec(&input).map_err(ClientError::EncodeWireIn)?;
        let frame_len = u32::try_from(payload.len())
            .map_err(|_| ClientError::WireFrameTooLarge(payload.len()))?;
        tx.write_all(&frame_len.to_be_bytes())
            .await
            .map_err(ClientError::WriteWireFrame)?;
        tx.write_all(&payload)
            .await
            .map_err(ClientError::WriteWireFrame)?;
        tx.finish().map_err(ClientError::FinishWireIn)?;

        let response = rx
            .read_to_end(usize::MAX)
            .await
            .map_err(ClientError::ReadWireOut)?;
        if response.len() < 4 {
            return Err(ClientError::InvalidWireFrameLength(response.len()));
        }

        let mut frame_len = [0_u8; 4];
        frame_len.copy_from_slice(&response[..4]);
        let expected = u32::from_be_bytes(frame_len) as usize;
        let payload = &response[4..];
        if payload.len() != expected {
            return Err(ClientError::MismatchedWireFrameLength {
                expected,
                actual: payload.len(),
            });
        }

        let response: Result<WireOut, BackendError> =
            postcard::from_bytes(payload).map_err(ClientError::DecodeWireOut)?;
        response.map_err(ClientError::Backend)
    }

    fn transport_error(err: ClientError) -> ExecutorError {
        match err {
            ClientError::Backend(err) => ExecutorError::Backend(err),
            other => ExecutorError::ClientTransport(other.to_string()),
        }
    }

    async fn send_wire_subscription(&self, input: WireIn) -> ClientResult<quinn::RecvStream> {
        let (mut tx, rx) = self
            .transport
            .conn
            .open_bi()
            .await
            .map_err(ClientError::OpenStream)?;

        let payload = postcard::to_allocvec(&input).map_err(ClientError::EncodeWireIn)?;
        let frame_len = u32::try_from(payload.len())
            .map_err(|_| ClientError::WireFrameTooLarge(payload.len()))?;
        tx.write_all(&frame_len.to_be_bytes())
            .await
            .map_err(ClientError::WriteWireFrame)?;
        tx.write_all(&payload)
            .await
            .map_err(ClientError::WriteWireFrame)?;
        tx.finish().map_err(ClientError::FinishWireIn)?;

        Ok(rx)
    }

    async fn send_history_event(
        &self,
        event: RunnerOut,
        action_name: &'static str,
    ) -> Result<(), ExecutorError> {
        let event_unix_ms = Utc::now().timestamp_millis();
        let response = self
            .send_wire_message(WireIn::HistoryEvent {
                event,
                event_unix_ms,
            })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_) => Err(ExecutorError::ClientTransport(format!(
                "unexpected non-ack response for {action_name}"
            ))),
        }
    }

    pub async fn subscribe_step_updates(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        let recv = self
            .send_wire_subscription(WireIn::SubscribeJourneyUpdates {
                journey_id,
                after_sequence_id,
            })
            .await
            .map_err(Self::transport_error)?;
        Ok(JourneyUpdateSubscription::from_stream(
            QuinnJourneyUpdateStream {
                recv,
                frame_state: SubscriptionFrameState::default(),
                closed: false,
            },
        ))
    }

    pub(crate) async fn spawn_by_id(
        &self,
        animal_id: u32,
        generation: u32,
        seed: Vec<u8>,
    ) -> Result<Uuid, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::CreateJourney {
                namespace: self.namespace.clone(),
                animal_id,
                generation,
                seed,
            })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyCreated(journey_id) => Ok(journey_id),
            WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-created response for spawn_by_id".to_string(),
            )),
        }
    }
}

#[async_trait]
impl<J> JungleClient for Client<J>
where
    J: Ecosystem + 'static,
    J::Animals: Animals,
    <J::Animals as Animals>::List: FlattenNodes,
    SPFlatten<<J::Animals as Animals>::List>: StripAnimalHeaders,
    AnimalSet<J::Animals>: Container,
{
    async fn spawn<A>(&self, seed: &A::Seed) -> Result<JourneyHandle, ExecutorError>
    where
        Self: Sized,
        A: Animal,
        A::Id: AnimalIdValue,
        A::Generation: Unsigned,
        A::Seed: Sync,
    {
        let seed = postcard::to_allocvec(seed)
            .map_err(|err| ExecutorError::InputSerialize(err.to_string()))?;
        let journey_id = self
            .spawn_by_id(
                <A::Id as AnimalIdValue>::U32,
                <A::Generation as Unsigned>::U32,
                seed,
            )
            .await?;
        Ok(JourneyHandle::new(journey_id, Box::new(self.clone())))
    }

    async fn journey_history(&self, id: Uuid) -> Result<Vec<RunnerOut>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyHistory(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyHistory(history) => Ok(history),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-history response for journey_history".to_string(),
            )),
        }
    }

    async fn journey_replay_page(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
        snapshot_end_sequence_id: Option<u64>,
        limit: u32,
    ) -> Result<JourneyReplayPage, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyReplayPage {
                journey_id,
                after_sequence_id,
                snapshot_end_sequence_id,
                limit,
            })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyReplayPage(page) => Ok(page),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-replay-page response for journey_replay_page".to_string(),
            )),
        }
    }

    async fn list_journeys(&self, namespace: String) -> Result<Vec<JourneyRecord>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::ListJourneys { namespace })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Journeys(journeys) => Ok(journeys),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journeys response for list_journeys".to_string(),
            )),
        }
    }

    async fn subscribe_step_updates(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        Client::subscribe_step_updates(self, journey_id, after_sequence_id).await
    }

    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyStatus(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyStatus(status) => Ok(status),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-status response for journey_details".to_string(),
            )),
        }
    }

    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::AnimalAppearance(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::AnimalAppearance(appearance) => Ok(appearance),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
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
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for animal_appearance_update".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for animal_appearance_update".to_string(),
                ))
            }
        }
    }

    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PerturbAnimal {
                journey_id: id,
                data: payload,
            })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for perturb_animal".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for perturb_animal".to_string(),
                ))
            }
        }
    }

    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<ClaimedPerturbable>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::ClaimPerturbable(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::ClaimedPerturbable(claimed) => Ok(claimed),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
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
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for ack_animal_perturbation".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for ack_animal_perturbation".to_string(),
                ))
            }
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
            .await
            .map_err(Self::transport_error)?;

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
            .await
            .map_err(Self::transport_error)?;

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
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for schedule_sleep_timer".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for schedule_sleep_timer".to_string(),
                ))
            }
        }
    }

    async fn complete_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyComplete(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for complete_journey".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for complete_journey".to_string(),
                ))
            }
        }
    }

    async fn dead_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyDead(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for dead_journey".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for dead_journey".to_string(),
                ))
            }
        }
    }

    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PollTimers)
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(Some(())),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for poll_timers".to_string(),
            )),
            WireOut::JourneyUpdate(_) | WireOut::Journeys(_) => {
                Err(ExecutorError::ClientTransport(
                    "unexpected non-ack response for poll_timers".to_string(),
                ))
            }
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
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::NoAvailableSteps => Ok(None),
            WireOut::PendingStep(work) => Ok(Some(work)),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
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
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyHistory(_)
            | WireOut::JourneyReplayPage(_)
            | WireOut::JourneyStatus(_)
            | WireOut::AnimalAppearance(_)
            | WireOut::ClaimedPerturbable(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::OwnerWake(_)
            | WireOut::JourneyUpdate(_)
            | WireOut::Journeys(_) => Err(ExecutorError::ClientTransport(
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
        self.send_history_event(
            RunnerOut::EffectInput {
                node_id,
                data: input,
                uuid: id,
            },
            "effect_input",
        )
        .await
    }

    async fn effect_success_output(
        &self,
        id: Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.send_history_event(
            RunnerOut::EffectSuccessOutput {
                node_id,
                data: output,
                uuid: id,
            },
            "effect_success_output",
        )
        .await
    }

    async fn effect_failure_output(
        &self,
        id: Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.send_history_event(
            RunnerOut::EffectFailureOutput {
                node_id,
                data: err,
                uuid: id,
            },
            "effect_failure_output",
        )
        .await
    }

    async fn submit_history_event(&self, event: RunnerOut) -> Result<(), ExecutorError> {
        self.send_history_event(event, "submit_history_event").await
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("remote address must be configured")]
    MissingRemote,
    #[error("server name must be configured")]
    MissingServerName,
    #[error("project dirs are unavailable for default cert lookup")]
    ProjectDirsUnavailable,
    #[error("failed to read custom CA certificate: {0}")]
    ReadCa(#[source] io::Error),
    #[error("failed to add trusted certificate: {0}")]
    AddTrustedCert(#[source] rustls::Error),
    #[error("failed to build QUIC client rustls config: {0}")]
    QuicClientConfig(#[source] quinn::crypto::rustls::NoInitialCipherSuite),
    #[error("failed to bind client endpoint: {0}")]
    BindEndpoint(#[source] io::Error),
    #[error("failed to start connection: {0}")]
    Connect(#[source] quinn::ConnectError),
    #[error("connection failed: {0}")]
    Connection(#[source] quinn::ConnectionError),
    #[error("failed to open stream: {0}")]
    OpenStream(#[source] quinn::ConnectionError),
    #[error("failed to bind rebind socket: {0}")]
    RebindSocket(#[source] io::Error),
    #[error("failed to rebind endpoint: {0}")]
    RebindEndpoint(#[source] io::Error),
    #[error("failed to encode wire input: {0}")]
    EncodeWireIn(#[source] postcard::Error),
    #[error("wire frame payload exceeds u32 length: {0}")]
    WireFrameTooLarge(usize),
    #[error("failed to write wire frame: {0}")]
    WriteWireFrame(#[source] quinn::WriteError),
    #[error("failed to finish wire input stream: {0}")]
    FinishWireIn(#[source] quinn::ClosedStream),
    #[error("failed to read wire output: {0}")]
    ReadWireOut(#[source] quinn::ReadToEndError),
    #[error("invalid wire frame length buffer: {0}")]
    InvalidWireFrameLength(usize),
    #[error(
        "mismatched wire frame payload length, expected {expected} bytes but received {actual}"
    )]
    MismatchedWireFrameLength { expected: usize, actual: usize },
    #[error("failed to decode wire output: {0}")]
    DecodeWireOut(#[source] postcard::Error),
    #[error("backend error: {0}")]
    Backend(#[source] BackendError),
}

pub type ClientResult<T> = std::result::Result<T, ClientError>;
