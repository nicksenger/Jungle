//! Client contracts for the Jungle workspace.

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::channel::{mpsc, oneshot};
use futures::StreamExt;
use jungle_types::{ExecutorError, RunnerOut, Work};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use std::fs;
use std::future::Future;
use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

type HandlerFuture = Pin<Box<dyn Future<Output = Result<(), ExecutorError>> + Send + 'static>>;
type Handler = Arc<dyn Fn(Uuid, Vec<u8>) -> HandlerFuture + Send + Sync + 'static>;
type PollWorkHandlerFuture =
    Pin<Box<dyn Future<Output = Result<Option<Work>, ExecutorError>> + Send + 'static>>;
type PollWorkHandler = Arc<dyn Fn() -> PollWorkHandlerFuture + Send + Sync + 'static>;

#[async_trait]
pub trait JungleClient: DynClone + Send + Sync {
    async fn poll_work(&self) -> Result<Option<Work>, ExecutorError>;
    async fn action_input(&self, id: Uuid, input: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError>;
    async fn action_failure_output(&self, id: Uuid, err: Vec<u8>) -> Result<(), ExecutorError>;
}

dyn_clone::clone_trait_object!(JungleClient);

pub type RunnerChannelTx = mpsc::Sender<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;
pub type RunnerChannelRx = mpsc::Receiver<(RunnerOut, oneshot::Sender<Result<(), ExecutorError>>)>;

#[derive(Clone)]
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

    async fn action_success_output(&self, id: Uuid, output: Vec<u8>) -> Result<(), ExecutorError> {
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
        let default_poll_work_handler: PollWorkHandler = Arc::new(|| Box::pin(async { Ok(None) }));
        MockClient {
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

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

#[derive(Debug, Clone)]
pub struct QuinnClientBuilder {
    keylog: bool,
    ca: Option<PathBuf>,
    rebind: bool,
    bind: SocketAddr,
    remote: Option<SocketAddr>,
    server_name: Option<String>,
    request: Vec<u8>,
}

impl Default for QuinnClientBuilder {
    fn default() -> Self {
        Self {
            keylog: false,
            ca: None,
            rebind: false,
            bind: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
            remote: None,
            server_name: None,
            request: b"jungle-client request\n".to_vec(),
        }
    }
}

impl QuinnClientBuilder {
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

    pub fn request(mut self, request: impl Into<Vec<u8>>) -> Self {
        self.request = request.into();
        self
    }

    pub fn build(self) -> QuinnResult<QuinnClient> {
        let remote = self.remote.ok_or(QuinnClientError::MissingRemote)?;
        let server_name = self
            .server_name
            .ok_or(QuinnClientError::MissingServerName)?;

        let mut roots = rustls::RootCertStore::empty();
        if let Some(ca_path) = self.ca {
            roots
                .add(CertificateDer::from(
                    fs::read(ca_path).map_err(QuinnClientError::ReadCa)?,
                ))
                .map_err(QuinnClientError::AddTrustedCert)?;
        } else {
            let dirs = directories_next::ProjectDirs::from("org", "jungle", "jungle-server")
                .ok_or(QuinnClientError::ProjectDirsUnavailable)?;
            match fs::read(dirs.data_local_dir().join("cert.der")) {
                Ok(cert) => {
                    roots
                        .add(CertificateDer::from(cert))
                        .map_err(QuinnClientError::AddTrustedCert)?;
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
            QuicClientConfig::try_from(client_crypto)
                .map_err(QuinnClientError::QuicClientConfig)?,
        ));
        let mut endpoint =
            quinn::Endpoint::client(self.bind).map_err(QuinnClientError::BindEndpoint)?;
        endpoint.set_default_client_config(client_config);

        Ok(QuinnClient {
            endpoint,
            remote,
            server_name,
            request: self.request,
            rebind: self.rebind,
        })
    }

    pub async fn run(self) -> QuinnResult<Vec<u8>> {
        self.build()?.run().await
    }
}

pub struct QuinnClient {
    endpoint: quinn::Endpoint,
    remote: SocketAddr,
    server_name: String,
    request: Vec<u8>,
    rebind: bool,
}

impl QuinnClient {
    pub async fn run(self) -> QuinnResult<Vec<u8>> {
        let connecting = self
            .endpoint
            .connect(self.remote, &self.server_name)
            .map_err(QuinnClientError::Connect)?;
        let conn = connecting.await.map_err(QuinnClientError::Connection)?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(QuinnClientError::OpenStream)?;

        if self.rebind {
            let socket = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))
                .map_err(QuinnClientError::RebindSocket)?;
            self.endpoint
                .rebind(socket)
                .map_err(QuinnClientError::RebindEndpoint)?;
        }

        send.write_all(&self.request)
            .await
            .map_err(QuinnClientError::WriteRequest)?;
        send.finish().map_err(QuinnClientError::FinishRequest)?;

        let response = recv
            .read_to_end(usize::MAX)
            .await
            .map_err(QuinnClientError::ReadResponse)?;

        conn.close(0u32.into(), b"done");
        self.endpoint.wait_idle().await;
        Ok(response)
    }
}

#[derive(Debug, Error)]
pub enum QuinnClientError {
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
    #[error("failed to send request: {0}")]
    WriteRequest(#[source] quinn::WriteError),
    #[error("failed to finish request stream: {0}")]
    FinishRequest(#[source] quinn::ClosedStream),
    #[error("failed to read response: {0}")]
    ReadResponse(#[source] quinn::ReadToEndError),
}

pub type QuinnResult<T> = std::result::Result<T, QuinnClientError>;
