use crate::runner::JungleRunner;
use futures::channel::mpsc;
use futures::StreamExt;
use jungle_client::{JungleClient, RunnerChannelTx};
use jungle_types::{
    BuildFlowWithContext, Anima, AnimaSet, Animae, DynFlow, Ecosystem, ExecutorError,
    RunnerOut, StripAnimaHeaders, Work,
};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::sleep;
use typosaurus::collections::list;
use typosaurus::collections::sp::{FlattenNodes, SPFlatten};
use typosaurus::num::Unsigned;
use uuid::Uuid;

pub struct JungleWorker<T> {
    client: Box<dyn JungleClient>,
    runner: JungleRunner<T>,
}

impl<T> JungleWorker<T>
where
    T: Ecosystem + 'static,
    T::Animae: Animae,
    <T::Animae as Animae>::List: FlattenNodes,
    SPFlatten<<T::Animae as Animae>::List>: StripAnimaHeaders,
    AnimaSet<T::Animae>: SpawnByOrdinal<T>,
{
    pub fn new<C>(jungle: T, client: C) -> Self
    where
        C: JungleClient + 'static,
    {
        Self {
            client: Box::new(client),
            runner: JungleRunner::new(jungle),
        }
    }

    pub fn client(&self) -> &dyn JungleClient {
        self.client.as_ref()
    }

    pub fn runner(&self) -> &JungleRunner<T> {
        &self.runner
    }

    pub async fn spawn(&self) -> Result<(), ExecutorError> {
        let (tx, mut rx): (RunnerChannelTx, _) = mpsc::channel(64);
        let client_for_transport = self.client.clone();
        tokio::spawn(async move {
            while let Some((message, done)) = rx.next().await {
                let result = match message {
                    RunnerOut::ActionInput { data, uuid } => {
                        client_for_transport.action_input(uuid, data).await
                    }
                    RunnerOut::ActionSuccessOutput { data, uuid } => {
                        client_for_transport.action_success_output(uuid, data).await
                    }
                    RunnerOut::ActionFailureOutput { data, uuid } => {
                        client_for_transport.action_failure_output(uuid, data).await
                    }
                };
                let _ = done.send(result);
            }
        });

        loop {
            match self.client.poll_work().await? {
                Some(Work::StartFlow {
                    flow_id,
                    ordinal,
                    seed,
                }) => {
                    let launched =
                        <AnimaSet<T::Animae> as SpawnByOrdinal<T>>::spawn_by_ordinal(
                            ordinal,
                            seed,
                            flow_id,
                            &self.runner,
                            tx.clone(),
                        )
                        .await?;
                    if !launched {
                        return Err(ExecutorError::InputDeserialize(format!(
                            "unknown anima ordinal: {ordinal}"
                        )));
                    }
                    self.client.flow_complete(flow_id).await?;
                }
                None => {}
            }
            sleep(Duration::from_millis(200)).await;
        }
    }
}

pub trait SpawnByOrdinal<T>: Send + Sync {
    fn spawn_by_ordinal<'a>(
        ordinal: u32,
        seed: Vec<u8>,
        flow_id: Uuid,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ExecutorError>> + 'a>>;
}

impl<T> SpawnByOrdinal<T> for list::Empty {
    fn spawn_by_ordinal<'a>(
        _ordinal: u32,
        _seed: Vec<u8>,
        _flow_id: Uuid,
        _runner: &'a JungleRunner<T>,
        _tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ExecutorError>> + 'a>> {
        Box::pin(async { Ok(false) })
    }
}

impl<T, Head, Tail, Ordinal> SpawnByOrdinal<T> for list::List<(Head, Tail)>
where
    Head: Anima<Id = jungle_types::Id<Ordinal>> + Send + Sync + 'static,
    Head::Seed: Send + 'static,
    Head::State: Send + 'static,
    Head::Journey:
        BuildFlowWithContext<(*const T, DynFlow<Head::State>), Output = DynFlow<Head::State>>,
    Ordinal: Unsigned,
    Tail: SpawnByOrdinal<T>,
    T: 'static,
{
    fn spawn_by_ordinal<'a>(
        ordinal: u32,
        seed: Vec<u8>,
        flow_id: Uuid,
        runner: &'a JungleRunner<T>,
        tx: RunnerChannelTx,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ExecutorError>> + 'a>> {
        Box::pin(async move {
            if ordinal == <Ordinal as Unsigned>::U32 {
                let seed: Head::Seed = postcard::from_bytes(&seed)
                    .map_err(|err| ExecutorError::InputDeserialize(err.to_string()))?;
                let state: Head::State = seed.into();
                let _ = runner.spawn::<Head>(state, flow_id, tx).await?;
                return Ok(true);
            }

            Tail::spawn_by_ordinal(ordinal, seed, flow_id, runner, tx).await
        })
    }
}
