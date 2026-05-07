use futures::channel::oneshot;
use futures::SinkExt;
use jungle_client::RunnerChannelTx;
use jungle_types::{BuildFlowWithContext, ContextExecutor, Creature, DynFlow, ExecutorError, RunnerOut};
use uuid::Uuid;

pub struct JungleRunner<T> {
    jungle: T,
}

impl<T> JungleRunner<T> {
    pub fn new(jungle: T) -> Self {
        Self { jungle }
    }

    pub fn jungle(&self) -> &T {
        &self.jungle
    }
}

impl<T> JungleRunner<T>
where
    T: 'static,
{
    pub async fn spawn<A>(
        &self,
        state: A::State,
        flow_id: Uuid,
        mut tx: RunnerChannelTx,
    ) -> Result<A::State, ExecutorError>
    where
        A: Creature,
        A::Instinct:
            BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = ContextExecutor::<T, A>::new(&self.jungle, state);
        while !executor.is_complete() {
            let request = executor.next_executable_request(())?;
            let (done_tx, done_rx) = oneshot::channel();
            tx.send((
                RunnerOut::ActionInput {
                    data: request.request_bytes().to_vec(),
                    uuid: flow_id,
                },
                done_tx,
            ))
            .await
            .map_err(|_| ExecutorError::ClientTransportClosed)?;
            done_rx
                .await
                .map_err(|_| ExecutorError::ClientTransportAckDropped)??;
            let completion = request.run().await?;
            let (done_tx, done_rx) = oneshot::channel();
            match &completion {
                Ok(output) => {
                    tx.send((
                        RunnerOut::ActionSuccessOutput {
                            data: output.clone(),
                            uuid: flow_id,
                        },
                        done_tx,
                    ))
                    .await
                    .map_err(|_| ExecutorError::ClientTransportClosed)?
                }
                Err(error) => {
                    tx.send((
                        RunnerOut::ActionFailureOutput {
                            data: error.clone(),
                            uuid: flow_id,
                        },
                        done_tx,
                    ))
                    .await
                    .map_err(|_| ExecutorError::ClientTransportClosed)?
                }
            }
            done_rx
                .await
                .map_err(|_| ExecutorError::ClientTransportAckDropped)??;
            let _emitted = executor.complete_serialized(completion)?;
        }
        Ok(executor.into_state())
    }
}
