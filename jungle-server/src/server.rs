use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use jungle_types::WireOut;
use tracing::info;

use crate::{JungleServer, Result, WireRx, WireTx};

#[derive(Debug, Default, Clone, Copy)]
pub struct Server;

#[async_trait]
impl JungleServer for Server {
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request = rx.next().await;
        info!(has_request = request.is_some(), "received request");

        let response = if request.is_some() {
            WireOut::Ack
        } else {
            WireOut::NoWorkAvailable
        };
        tx.send(Ok(response)).await?;
        tx.close().await?;
        info!("complete");
        Ok(())
    }
}
