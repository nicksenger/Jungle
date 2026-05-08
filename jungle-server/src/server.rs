use async_trait::async_trait;
use tracing::info;

use crate::{Backend, Result, ServerError};

#[derive(Debug, Default, Clone, Copy)]
pub struct Server;

#[async_trait]
impl Backend for Server {
    async fn handle_request(
        &self,
        backend: Box<dyn Backend>,
        (send, recv): (quinn::SendStream, quinn::RecvStream),
    ) -> Result<()> {
        let (mut send, mut recv) = (send, recv);
        let req = recv
            .read_to_end(64 * 1024)
            .await
            .map_err(ServerError::ReadRequest)?;
        info!(request_len = req.len(), "received request");

        let resp = backend.handle_backend_request(&req).await?;
        send.write_all(&resp)
            .await
            .map_err(ServerError::WriteResponse)?;
        send.finish().map_err(ServerError::FinishResponse)?;
        info!("complete");
        Ok(())
    }
}
