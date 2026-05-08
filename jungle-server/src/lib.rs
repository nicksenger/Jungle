use std::{fs, io, net::SocketAddr, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use dyn_clone::DynClone;
use quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use thiserror::Error;
use tracing::{error, info, info_span};
use tracing_futures::Instrument as _;

pub mod mock;
pub mod server;
pub use mock::MockServer;
pub use server::Server;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];
const DEFAULT_LISTEN_ADDR: &str = "[::1]:4433";

#[async_trait]
pub trait Backend: Send + Sync + 'static {
    async fn handle_request(&self, stream: (quinn::SendStream, quinn::RecvStream)) -> Result<()> {
        let (mut send, mut recv) = stream;
        let req = recv
            .read_to_end(64 * 1024)
            .await
            .map_err(ServerError::ReadRequest)?;
        info!(request_len = req.len(), "received request");

        send.write_all(&[])
            .await
            .map_err(ServerError::WriteResponse)?;
        send.finish().map_err(ServerError::FinishResponse)?;
        info!("complete");
        Ok(())
    }

    async fn handle_connection(self: Self, conn: quinn::Incoming) -> Result<()> {
        let connection = conn.await?;
        let span = info_span!(
            "connection",
            remote = %connection.remote_address(),
            protocol = %connection
                .handshake_data()
                .unwrap()
                .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
                .protocol
                .map_or_else(|| "<none>".into(), |x| String::from_utf8_lossy(&x).into_owned())
        );
        async {
            info!("established");

            // Each stream initiated by the client constitutes a new request.
            loop {
                let stream = connection.accept_bi().await;
                let stream = match stream {
                    Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                        info!("connection closed");
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(ServerError::Connection(e));
                    }
                    Ok(s) => s,
                };
                let backend = dyn_clone::clone_box(&*backend);
                let handler = Arc::clone(&self);
                tokio::spawn(
                    async move {
                        if let Err(e) = handler.handle_request(backend, stream).await {
                            error!("failed: {reason}", reason = e.to_string());
                        }
                    }
                    .instrument(info_span!("request")),
                );
            }
        }
        .instrument(span)
        .await?;
        Ok(())
    }
}

dyn_clone::clone_trait_object!(Backend);

#[derive(Clone)]
pub struct ServerBuilder {
    keylog: bool,
    key: Option<PathBuf>,
    cert: Option<PathBuf>,
    stateless_retry: bool,
    listen: SocketAddr,
    block: Option<SocketAddr>,
    connection_limit: Option<usize>,
    server: Arc<dyn Backend>,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self {
            keylog: false,
            key: None,
            cert: None,
            stateless_retry: false,
            listen: DEFAULT_LISTEN_ADDR
                .parse()
                .expect("default listen address must be valid"),
            block: None,
            connection_limit: None,
            backend: Box::new(MockServer::default()),
            server: Arc::new(Server),
        }
    }
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keylog(mut self, enabled: bool) -> Self {
        self.keylog = enabled;
        self
    }

    pub fn key(mut self, key: impl Into<PathBuf>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn cert(mut self, cert: impl Into<PathBuf>) -> Self {
        self.cert = Some(cert.into());
        self
    }

    pub fn stateless_retry(mut self, enabled: bool) -> Self {
        self.stateless_retry = enabled;
        self
    }

    pub fn listen(mut self, addr: SocketAddr) -> Self {
        self.listen = addr;
        self
    }

    pub fn block(mut self, addr: SocketAddr) -> Self {
        self.block = Some(addr);
        self
    }

    pub fn connection_limit(mut self, limit: usize) -> Self {
        self.connection_limit = Some(limit);
        self
    }

    pub fn server<S>(mut self, server: S) -> Self
    where
        S: Backend,
    {
        self.server = Arc::new(server);
        self
    }

    pub fn handler<S>(self, server: S) -> Self
    where
        S: Backend,
    {
        self.server(server)
    }

    pub async fn run(self) -> Result<()> {
        let (certs, key) = match (&self.key, &self.cert) {
            (Some(key_path), Some(cert_path)) => load_user_cert_chain_and_key(key_path, cert_path)?,
            (None, None) => load_or_generate_self_signed_cert()?,
            _ => return Err(ServerError::MissingKeyOrCertPair),
        };

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(ServerError::RustlsCertConfig)?;
        server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
        if self.keylog {
            server_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
        }

        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto).map_err(ServerError::QuicServerConfig)?,
        ));
        let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
        transport_config.max_concurrent_uni_streams(0_u8.into());

        let endpoint = quinn::Endpoint::server(server_config, self.listen)
            .map_err(ServerError::BindEndpoint)?;
        eprintln!(
            "listening on {}",
            endpoint.local_addr().map_err(ServerError::LocalAddr)?
        );

        while let Some(conn) = endpoint.accept().await {
            if self
                .connection_limit
                .is_some_and(|n| endpoint.open_connections() >= n)
            {
                info!("refusing due to open connection limit");
                conn.refuse();
            } else if Some(conn.remote_address()) == self.block {
                info!("refusing blocked client IP address");
                conn.refuse();
            } else if self.stateless_retry && !conn.remote_address_validated() {
                info!("requiring connection to validate its address");
                let _ = conn.retry();
            } else {
                info!("accepting connection");
                let backend = dyn_clone::clone_box(&*self.backend);
                let server = Arc::clone(&self.server);
                let fut = Arc::clone(&server).handle_connection(backend, conn);
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        error!("connection failed: {reason}", reason = e.to_string())
                    }
                });
            }
        }

        Ok(())
    }
}

fn load_user_cert_chain_and_key(
    key_path: &PathBuf,
    cert_path: &PathBuf,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let key = if key_path.extension().is_some_and(|x| x == "der") {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            fs::read(key_path).map_err(ServerError::ReadPrivateKeyFile)?,
        ))
    } else {
        PrivateKeyDer::from_pem_file(key_path).map_err(ServerError::ReadPrivateKeyPem)?
    };

    let cert_chain = if cert_path.extension().is_some_and(|x| x == "der") {
        vec![CertificateDer::from(
            fs::read(cert_path).map_err(ServerError::ReadCertChainFile)?,
        )]
    } else {
        CertificateDer::pem_file_iter(cert_path)
            .map_err(ServerError::ReadCertChainPem)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(ServerError::InvalidPemCert)?
    };

    Ok((cert_chain, key))
}

fn load_or_generate_self_signed_cert(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let dirs = directories_next::ProjectDirs::from("org", "jungle", "jungle-server").unwrap();
    let path = dirs.data_local_dir();
    let cert_path = path.join("cert.der");
    let key_path = path.join("key.der");

    let (cert, key) = match fs::read(&cert_path).and_then(|x| Ok((x, fs::read(&key_path)?))) {
        Ok((cert, key)) => (
            CertificateDer::from(cert),
            PrivateKeyDer::try_from(key).map_err(|e| ServerError::ParseDerKey(e.to_owned()))?,
        ),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            info!("generating self-signed certificate");
            let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
            let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
            let cert = cert.cert.into();
            fs::create_dir_all(path).map_err(ServerError::CreateCertDir)?;
            fs::write(&cert_path, &cert).map_err(ServerError::WriteCert)?;
            fs::write(&key_path, key.secret_pkcs8_der()).map_err(ServerError::WritePrivateKey)?;
            (cert, key.into())
        }
        Err(e) => {
            return Err(ServerError::ReadCertificate(e));
        }
    };

    Ok((vec![cert], key))
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("private key and certificate must either both be set or both omitted")]
    MissingKeyOrCertPair,
    #[error("failed to read private key file: {0}")]
    ReadPrivateKeyFile(#[source] io::Error),
    #[error("failed to read PEM from private key file: {0}")]
    ReadPrivateKeyPem(#[source] rustls::pki_types::pem::Error),
    #[error("failed to read certificate chain file: {0}")]
    ReadCertChainFile(#[source] io::Error),
    #[error("failed to read PEM from certificate chain file: {0}")]
    ReadCertChainPem(#[source] rustls::pki_types::pem::Error),
    #[error("invalid PEM-encoded certificate: {0}")]
    InvalidPemCert(#[source] rustls::pki_types::pem::Error),
    #[error("failed to parse DER private key: {0}")]
    ParseDerKey(String),
    #[error("failed to create certificate directory: {0}")]
    CreateCertDir(#[source] io::Error),
    #[error("failed to write certificate: {0}")]
    WriteCert(#[source] io::Error),
    #[error("failed to write private key: {0}")]
    WritePrivateKey(#[source] io::Error),
    #[error("failed to read certificate: {0}")]
    ReadCertificate(io::Error),
    #[error("failed to configure rustls certificate: {0}")]
    RustlsCertConfig(#[source] rustls::Error),
    #[error("failed to build QUIC rustls config: {0}")]
    QuicServerConfig(#[source] quinn::crypto::rustls::NoInitialCipherSuite),
    #[error("failed to bind QUIC endpoint: {0}")]
    BindEndpoint(#[source] io::Error),
    #[error("failed to fetch local listen address: {0}")]
    LocalAddr(#[source] io::Error),
    #[error("incoming connection failed: {0}")]
    Connection(#[from] quinn::ConnectionError),
    #[error("stream read failed: {0}")]
    ReadRequest(#[source] quinn::ReadToEndError),
    #[error("stream write failed: {0}")]
    WriteResponse(#[source] quinn::WriteError),
    #[error("stream finish failed: {0}")]
    FinishResponse(#[source] quinn::ClosedStream),
    #[error("backend request handling failed: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, ServerError>;

pub fn init_tracing() -> std::result::Result<(), tracing::subscriber::SetGlobalDefaultError> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
}
