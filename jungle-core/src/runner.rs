use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use jungle_types::{BuildFlowWithContext, ClientIn, ContextExecutor, Creature, DynFlow, ExecutorError};
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
        mut tx: mpsc::Sender<(ClientIn, oneshot::Sender<()>)>,
    ) -> Result<A::State, ExecutorError>
    where
        A: Creature,
        A::Instinct:
            BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = ContextExecutor::<T, A>::new(&self.jungle, state);
        while !executor.is_complete() {
            let request = executor.next_executable_request(())?;
            let completion = request.run().await?;
            let data = match &completion {
                Ok(output) => output.clone(),
                Err(error) => error.clone(),
            };
            let (notify, notified) = oneshot::channel();
            tx.send((ClientIn::ActionOutput { data, uuid: flow_id }, notify))
                .await
                .map_err(|_| ExecutorError::ClientTransportClosed)?;
            notified
                .await
                .map_err(|_| ExecutorError::ClientTransportAckDropped)?;
            let _emitted = executor.complete_serialized(completion)?;
        }
        Ok(executor.into_state())
    }
}
