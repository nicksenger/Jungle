use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
#[cfg(any(feature = "postgres", feature = "redb"))]
use jungle_persist::JungleStore;
use jungle_types::{BackendError, WireIn, WireOut};
#[cfg(any(feature = "postgres", feature = "redb"))]
use std::sync::Arc;
use tracing::info;

use crate::{JungleServer, Result, WireRx, WireTx};

#[derive(Clone)]
pub struct Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    store: Arc<dyn JungleStore>,
}

impl Server {
    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    #[cfg(any(feature = "postgres", feature = "redb"))]
    pub fn from_store(store: Arc<dyn JungleStore>) -> Self {
        Self { store }
    }
}

#[cfg(any(feature = "postgres", feature = "redb"))]
#[derive(Default, Clone)]
pub struct ServerBuilder {
    #[cfg(feature = "postgres")]
    postgres: Option<jungle_persist::pg::PgStoreBuilder>,
    #[cfg(feature = "redb")]
    redb: Option<jungle_persist::redb::RedbStoreBuilder>,
}

#[cfg(any(feature = "postgres", feature = "redb"))]
impl ServerBuilder {
    #[cfg(feature = "postgres")]
    pub fn postgres(mut self, builder: jungle_persist::pg::PgStoreBuilder) -> Self {
        self.postgres = Some(builder);
        self
    }

    #[cfg(feature = "postgres")]
    pub fn postgres_connection_string(self, value: impl Into<String>) -> Self {
        self.postgres(jungle_persist::pg::PgStore::builder().connection_string(value))
    }

    #[cfg(feature = "redb")]
    pub fn redb(mut self, builder: jungle_persist::redb::RedbStoreBuilder) -> Self {
        self.redb = Some(builder);
        self
    }

    #[cfg(feature = "redb")]
    pub fn redb_path(self, value: impl Into<std::path::PathBuf>) -> Self {
        self.redb(jungle_persist::redb::RedbStore::builder().path(value))
    }

    pub async fn build(self) -> jungle_persist::Result<Server> {
        #[cfg(all(feature = "postgres", feature = "redb"))]
        {
            if let Some(builder) = self.postgres {
                let store = builder.build().await?;
                store.migrate().await?;
                return Ok(Server::from_store(Arc::new(store)));
            }

            if let Some(builder) = self.redb {
                let store = builder.build()?;
                store.migrate().await?;
                return Ok(Server::from_store(Arc::new(store)));
            }

            info!("both `postgres` and `redb` features are enabled; defaulting to postgres");
            let store = jungle_persist::pg::PgStore::builder().build().await?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }

        #[cfg(all(feature = "postgres", not(feature = "redb")))]
        {
            let store = self
                .postgres
                .unwrap_or_else(jungle_persist::pg::PgStore::builder)
                .build()
                .await?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }

        #[cfg(all(feature = "redb", not(feature = "postgres")))]
        {
            let store = self
                .redb
                .unwrap_or_else(jungle_persist::redb::RedbStore::builder)
                .build()?;
            store.migrate().await?;
            return Ok(Server::from_store(Arc::new(store)));
        }
    }
}

#[async_trait]
impl JungleServer for Server {
    async fn handle_request(&self, (mut tx, mut rx): (WireTx, WireRx)) -> Result<()> {
        let request = rx.next().await;
        info!(has_request = request.is_some(), "received request");

        let response = match request {
            Some(WireIn::CreateFlow { ordinal, seed }) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let flow_id = self.store.create_flow(ordinal, seed).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::FlowCreated(flow_id)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = (ordinal, seed);
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "create_flow is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::FlowStatus(flow_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let status = self.store.flow_status(flow_id).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::FlowStatus(status)
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = flow_id;
                    return Err(crate::ServerError::Backend(BackendError::Message(
                        "flow_status is unavailable without a persistence backend".to_string(),
                    )));
                }
            }
            Some(WireIn::FlowComplete(flow_id)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    self.store.flow_complete(flow_id).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = flow_id;
                    WireOut::Ack
                }
            }
            Some(WireIn::PollWork) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    match self.store.claim_work().await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })? {
                        Some(work) => WireOut::PendingWork(work),
                        None => WireOut::NoWorkAvailable,
                    }
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    WireOut::NoWorkAvailable
                }
            }
            Some(WireIn::HistoryEvent(history)) => {
                #[cfg(any(feature = "postgres", feature = "redb"))]
                {
                    let flow_id = match &history {
                        jungle_types::RunnerOut::ActionInput { uuid, .. }
                        | jungle_types::RunnerOut::ActionSuccessOutput { uuid, .. }
                        | jungle_types::RunnerOut::ActionFailureOutput { uuid, .. } => *uuid,
                    };
                    self.store
                        .flow_alive_if_created(flow_id)
                        .await
                        .map_err(|err| {
                            crate::ServerError::Backend(BackendError::Message(err.to_string()))
                        })?;
                    self.store.append_history(history).await.map_err(|err| {
                        crate::ServerError::Backend(BackendError::Message(err.to_string()))
                    })?;
                    WireOut::Ack
                }
                #[cfg(not(any(feature = "postgres", feature = "redb")))]
                {
                    let _ = history;
                    WireOut::Ack
                }
            }
            None => WireOut::NoWorkAvailable,
        };
        tx.send(Ok(response)).await?;
        tx.close().await?;
        info!("complete");
        Ok(())
    }
}
