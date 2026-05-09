use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[clap(name = "jungle-server")]
struct Opt {
    /// file to log TLS keys to for debugging
    #[clap(long = "keylog")]
    keylog: bool,
    /// TLS private key in PEM or DER format
    #[clap(short = 'k', long = "key", requires = "cert")]
    key: Option<PathBuf>,
    /// TLS certificate in PEM or DER format
    #[clap(short = 'c', long = "cert", requires = "key")]
    cert: Option<PathBuf>,
    /// Enable stateless retries
    #[clap(long = "stateless-retry")]
    stateless_retry: bool,
    /// Address to listen on
    #[clap(long = "listen", default_value = "[::1]:4433")]
    listen: SocketAddr,
    /// Client address to block
    #[clap(long = "block")]
    block: Option<SocketAddr>,
    /// Maximum number of concurrent connections to allow
    #[clap(long = "connection-limit")]
    connection_limit: Option<usize>,
    /// PostgreSQL connection string
    #[cfg(feature = "postgres")]
    #[clap(long = "postgres-connection-string")]
    postgres_connection_string: Option<String>,
    /// redb file path
    #[cfg(feature = "redb")]
    #[clap(long = "redb-path")]
    redb_path: Option<PathBuf>,
}

impl From<Opt> for jungle_server::ServerBuilder {
    fn from(opt: Opt) -> Self {
        let mut builder = jungle_server::ServerBuilder::new()
            .keylog(opt.keylog)
            .stateless_retry(opt.stateless_retry)
            .listen(opt.listen);

        if let Some(key) = opt.key {
            builder = builder.key(key);
        }
        if let Some(cert) = opt.cert {
            builder = builder.cert(cert);
        }
        if let Some(block) = opt.block {
            builder = builder.block(block);
        }
        if let Some(connection_limit) = opt.connection_limit {
            builder = builder.connection_limit(connection_limit);
        }
        #[cfg(feature = "postgres")]
        if let Some(connection_string) = opt.postgres_connection_string {
            builder = builder.postgres_connection_string(connection_string);
        }
        #[cfg(feature = "redb")]
        if let Some(path) = opt.redb_path {
            builder = builder.redb_path(path);
        }

        builder
    }
}

fn main() {
    if jungle_server::init_tracing().is_err() {
        eprintln!("ERROR: failed to initialize tracing subscriber");
        std::process::exit(1);
    }

    let builder = jungle_server::ServerBuilder::from(Opt::parse());
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let code = if let Err(e) = rt.block_on(builder.run()) {
        eprintln!("ERROR: {e}");
        1
    } else {
        0
    };
    std::process::exit(code);
}
