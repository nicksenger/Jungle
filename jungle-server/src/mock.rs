use std::sync::Arc;

use async_trait::async_trait;

use crate::{Result, ServerError};

#[derive(Clone)]
pub struct MockServer {}

impl Default for MockServer {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl MockServer {
    pub fn builder() -> MockServerBuilder {
        MockServerBuilder::default()
    }
}

#[async_trait]
impl Backend for MockServer {
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

    async fn handle_connection(self: Arc<Self>, conn: quinn::Incoming) -> Result<()> {
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

#[derive(Default)]
pub struct MockServerBuilder {}

impl MockServerBuilder {
    pub fn build(self) -> MockServer {
        MockServer {}
    }
}
