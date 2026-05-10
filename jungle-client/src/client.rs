use crate::JungleClient;
use async_trait::async_trait;
use jungle_types::{
    BackendError, ExecutorError, JourneyStatus, RunnerOut, RunnerStep, WireIn, WireOut,
};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use std::fs;
use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

#[derive(Debug, Clone)]
pub struct ClientBuilder {
    keylog: bool,
    ca: Option<PathBuf>,
    rebind: bool,
    bind: SocketAddr,
    remote: Option<SocketAddr>,
    server_name: Option<String>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            keylog: false,
            ca: None,
            rebind: false,
            bind: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
            remote: None,
            server_name: None,
        }
    }
}

impl ClientBuilder {
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

    pub async fn build(self) -> ClientResult<Client> {
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

        Ok(Client { endpoint, conn })
    }
}

#[derive(Clone)]
pub struct Client {
    endpoint: quinn::Endpoint,
    conn: quinn::Connection,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    async fn send_wire_message(&self, input: WireIn) -> ClientResult<WireOut> {
        let (mut tx, mut rx) = self.conn.open_bi().await.map_err(ClientError::OpenStream)?;

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
}

impl Drop for Client {
    fn drop(&mut self) {
        self.conn.close(0u32.into(), b"done");
        self.endpoint.close(0u32.into(), b"done");
    }
}

#[async_trait]
impl JungleClient for Client {
    async fn start_journey(&self, ordinal: u32, seed: Vec<u8>) -> Result<Uuid, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::CreateJourney { ordinal, seed })
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyCreated(journey_id) => Ok(journey_id),
            WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-created response for start_journey".to_string(),
            )),
        }
    }

    async fn journey_details(&self, id: Uuid) -> Result<JourneyStatus, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyStatus(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyStatus(status) => Ok(status),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-status response for journey_details".to_string(),
            )),
        }
    }

    async fn journey_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::JourneyAppearance(id))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::JourneyAppearance(appearance) => Ok(appearance),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected non-journey-appearance response for journey_appearance".to_string(),
            )),
        }
    }

    async fn journey_appearance_update(
        &self,
        id: Uuid,
        data: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::HistoryEvent(RunnerOut::Appearance {
                data,
                uuid: id,
            }))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for journey_appearance_update".to_string(),
            )),
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
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for complete_journey".to_string(),
            )),
        }
    }

    async fn poll_work(&self) -> Result<Option<RunnerStep>, ExecutorError> {
        let response = self
            .send_wire_message(WireIn::PollStep)
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::NoAvailableSteps => Ok(None),
            WireOut::PendingStep(work) => Ok(Some(work)),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::Ack => Err(ExecutorError::ClientTransport(
                "unexpected response for poll_work".to_string(),
            )),
        }
    }

    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::HistoryEvent(RunnerOut::ActionInput {
                data: input,
                uuid: id,
            }))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for action_input".to_string(),
            )),
        }
    }

    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::HistoryEvent(RunnerOut::ActionSuccessOutput {
                data: output,
                uuid: id,
            }))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for action_success_output".to_string(),
            )),
        }
    }

    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError> {
        let response = self
            .send_wire_message(WireIn::HistoryEvent(RunnerOut::ActionFailureOutput {
                data: err,
                uuid: id,
            }))
            .await
            .map_err(Self::transport_error)?;

        match response {
            WireOut::Ack => Ok(()),
            WireOut::JourneyCreated(_)
            | WireOut::JourneyStatus(_)
            | WireOut::JourneyAppearance(_)
            | WireOut::NoAvailableSteps
            | WireOut::PendingStep(_) => Err(ExecutorError::ClientTransport(
                "unexpected non-ack response for action_failure_output".to_string(),
            )),
        }
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
