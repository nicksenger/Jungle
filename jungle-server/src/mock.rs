use async_trait::async_trait;
use tracing::info;

use crate::{Backend, Result, ServerError};

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
    async fn handle_request(
        &self,
        (send, recv): (quinn::SendStream, quinn::RecvStream),
    ) -> Result<()> {
        let (mut send, mut recv) = (send, recv);
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
}

#[derive(Default)]
pub struct MockServerBuilder {}

impl MockServerBuilder {
    pub fn build(self) -> MockServer {
        MockServer {}
    }
}
