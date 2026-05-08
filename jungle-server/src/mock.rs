use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use jungle_types::WireOut;
use tracing::info;

use crate::{Backend, Result, WireRx, WireTx};

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
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request = rx.next().await;
        info!(has_request = request.is_some(), "received request");

        let response = if request.is_some() {
            WireOut::Ack
        } else {
            WireOut::NoWorkAvailable
        };
        tx.send(response).await?;
        tx.close().await?;
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
