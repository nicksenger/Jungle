use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use jungle_types::{BackendError, WireIn, WireOut};
use std::{future::Future, pin::Pin, sync::Arc};
use tracing::debug;

use crate::{JungleServer, Result, WireRx, WireTx};

type RequestHandlerFuture =
    Pin<Box<dyn Future<Output = std::result::Result<WireOut, BackendError>> + Send + 'static>>;
type RequestHandler = Arc<dyn Fn(Option<WireIn>) -> RequestHandlerFuture + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockServer {
    on_request: RequestHandler,
}

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
impl JungleServer for MockServer {
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request = rx.next().await;
        debug!(has_request = request.is_some(), "received request");

        let response = (self.on_request)(request).await;
        tx.send(response).await?;
        tx.close().await?;
        debug!("complete");
        Ok(())
    }
}

#[derive(Default)]
pub struct MockServerBuilder {
    on_request: Option<RequestHandler>,
}

impl MockServerBuilder {
    pub fn on_request<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Option<WireIn>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<WireOut, BackendError>> + Send + 'static,
    {
        self.on_request = Some(Arc::new(move |request| Box::pin(f(request))));
        self
    }

    pub fn build(self) -> MockServer {
        let default_handler: RequestHandler = Arc::new(|request| {
            Box::pin(async move {
                if request.is_some() {
                    Ok(WireOut::Ack)
                } else {
                    Ok(WireOut::NoAvailableSteps)
                }
            })
        });

        MockServer {
            on_request: self.on_request.unwrap_or(default_handler),
        }
    }
}
