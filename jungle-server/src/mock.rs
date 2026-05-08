use std::sync::Arc;

use async_trait::async_trait;

use crate::{Backend, Result, ServerError};

type RequestCallback = Arc<dyn Fn(Vec<u8>) -> Result<Vec<u8>> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MockServer {
    on_request: RequestCallback,
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
impl Backend for MockServer {
    async fn handle_backend_request(&self, request: &[u8]) -> Result<Vec<u8>> {
        (self.on_request)(request.to_vec())
    }
}

#[derive(Default)]
pub struct MockServerBuilder {
    on_request: Option<RequestCallback>,
}

impl MockServerBuilder {
    pub fn on_request<F>(mut self, f: F) -> Self
    where
        F: Fn(Vec<u8>) -> Result<Vec<u8>> + Send + Sync + 'static,
    {
        self.on_request = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> MockServer {
        let default_handler: RequestCallback =
            Arc::new(|_| Ok(b"jungle-server stub response\n".to_vec()));
        MockServer {
            on_request: self.on_request.unwrap_or(default_handler),
        }
    }

    pub fn fail_with(mut self, message: impl Into<String>) -> Self {
        let message = message.into();
        self.on_request = Some(Arc::new(move |_| {
            Err(ServerError::Backend(message.clone()))
        }));
        self
    }
}
