use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use std::fs;
use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

#[derive(Debug, Clone)]
pub struct ClientBuilder {
    keylog: bool,
    ca: Option<PathBuf>,
    rebind: bool,
    bind: SocketAddr,
    remote: Option<SocketAddr>,
    server_name: Option<String>,
    request: Vec<u8>,
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
            request: b"jungle-client request\n".to_vec(),
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

    pub fn request(mut self, request: impl Into<Vec<u8>>) -> Self {
        self.request = request.into();
        self
    }

    pub fn build(self) -> ClientResult<Client> {
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

        Ok(Client {
            endpoint,
            remote,
            server_name,
            request: self.request,
            rebind: self.rebind,
        })
    }

    pub async fn run(self) -> ClientResult<Vec<u8>> {
        self.build()?.run().await
    }
}

pub struct Client {
    endpoint: quinn::Endpoint,
    remote: SocketAddr,
    server_name: String,
    request: Vec<u8>,
    rebind: bool,
}

impl Client {
    pub async fn run(self) -> ClientResult<Vec<u8>> {
        let connecting = self
            .endpoint
            .connect(self.remote, &self.server_name)
            .map_err(ClientError::Connect)?;
        let conn = connecting.await.map_err(ClientError::Connection)?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(ClientError::OpenStream)?;

        if self.rebind {
            let socket =
                UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0)).map_err(ClientError::RebindSocket)?;
            self.endpoint
                .rebind(socket)
                .map_err(ClientError::RebindEndpoint)?;
        }

        send.write_all(&self.request)
            .await
            .map_err(ClientError::WriteRequest)?;
        send.finish().map_err(ClientError::FinishRequest)?;

        let response = recv
            .read_to_end(usize::MAX)
            .await
            .map_err(ClientError::ReadResponse)?;

        conn.close(0u32.into(), b"done");
        self.endpoint.wait_idle().await;
        Ok(response)
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
    #[error("failed to send request: {0}")]
    WriteRequest(#[source] quinn::WriteError),
    #[error("failed to finish request stream: {0}")]
    FinishRequest(#[source] quinn::ClosedStream),
    #[error("failed to read response: {0}")]
    ReadResponse(#[source] quinn::ReadToEndError),
}

pub type ClientResult<T> = std::result::Result<T, ClientError>;
