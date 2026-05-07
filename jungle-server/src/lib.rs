use std::{fs, io, net::SocketAddr, path::PathBuf, sync::Arc};

use clap::Parser;
use quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use thiserror::Error;
use tracing::{error, info, info_span};
use tracing_futures::Instrument as _;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

#[derive(Parser, Debug, Clone)]
#[clap(name = "jungle-server")]
pub struct Opt {
    /// file to log TLS keys to for debugging
    #[clap(long = "keylog")]
    pub keylog: bool,
    /// TLS private key in PEM or DER format
    #[clap(short = 'k', long = "key", requires = "cert")]
    pub key: Option<PathBuf>,
    /// TLS certificate in PEM or DER format
    #[clap(short = 'c', long = "cert", requires = "key")]
    pub cert: Option<PathBuf>,
    /// Enable stateless retries
    #[clap(long = "stateless-retry")]
    pub stateless_retry: bool,
    /// Address to listen on
    #[clap(long = "listen", default_value = "[::1]:4433")]
    pub listen: SocketAddr,
    /// Client address to block
    #[clap(long = "block")]
    pub block: Option<SocketAddr>,
    /// Maximum number of concurrent connections to allow
    #[clap(long = "connection-limit")]
    pub connection_limit: Option<usize>,
}

#[derive(Debug, Error)]
pub enum ServerError {
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
}

pub type Result<T> = std::result::Result<T, ServerError>;

pub fn init_tracing() -> std::result::Result<(), tracing::subscriber::SetGlobalDefaultError> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
}

pub async fn run(options: Opt) -> Result<()> {
    let (certs, key) = if let (Some(key_path), Some(cert_path)) = (&options.key, &options.cert) {
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

        (cert_chain, key)
    } else {
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
                fs::write(&key_path, key.secret_pkcs8_der())
                    .map_err(ServerError::WritePrivateKey)?;
                (cert, key.into())
            }
            Err(e) => {
                return Err(ServerError::ReadCertificate(e));
            }
        };

        (vec![cert], key)
    };

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(ServerError::RustlsCertConfig)?;
    server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    if options.keylog {
        server_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
    }

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        QuicServerConfig::try_from(server_crypto).map_err(ServerError::QuicServerConfig)?,
    ));
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());

    let endpoint = quinn::Endpoint::server(server_config, options.listen)
        .map_err(ServerError::BindEndpoint)?;
    eprintln!(
        "listening on {}",
        endpoint.local_addr().map_err(ServerError::LocalAddr)?
    );

    while let Some(conn) = endpoint.accept().await {
        if options
            .connection_limit
            .is_some_and(|n| endpoint.open_connections() >= n)
        {
            info!("refusing due to open connection limit");
            conn.refuse();
        } else if Some(conn.remote_address()) == options.block {
            info!("refusing blocked client IP address");
            conn.refuse();
        } else if options.stateless_retry && !conn.remote_address_validated() {
            info!("requiring connection to validate its address");
            let _ = conn.retry();
        } else {
            info!("accepting connection");
            let fut = handle_connection(conn);
            tokio::spawn(async move {
                if let Err(e) = fut.await {
                    error!("connection failed: {reason}", reason = e.to_string())
                }
            });
        }
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Incoming) -> Result<()> {
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
            let fut = handle_request(stream);
            tokio::spawn(
                async move {
                    if let Err(e) = fut.await {
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

async fn handle_request(
    (mut send, mut recv): (quinn::SendStream, quinn::RecvStream),
) -> Result<()> {
    let req = recv
        .read_to_end(64 * 1024)
        .await
        .map_err(ServerError::ReadRequest)?;
    info!(request_len = req.len(), "received request");

    // Stub handler: protocol/certificate setup is complete; app protocol is intentionally pending.
    let resp = b"jungle-server stub response\n";
    send.write_all(resp)
        .await
        .map_err(ServerError::WriteResponse)?;
    send.finish().map_err(ServerError::FinishResponse)?;
    info!("complete");
    Ok(())
}
