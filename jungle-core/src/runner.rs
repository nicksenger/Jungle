use jungle_client::JungleClient;
use jungle_types::{BuildFlowWithContext, ContextExecutor, Creature, DynFlow, ExecutorError};
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
    pub async fn spawn<A, C>(
        &self,
        state: A::State,
        flow_id: Uuid,
        client: &C,
    ) -> Result<A::State, ExecutorError>
    where
        A: Creature,
        C: JungleClient + ?Sized,
        A::Instinct:
            BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = ContextExecutor::<T, A>::new(&self.jungle, state);
        while !executor.is_complete() {
            let request = executor.next_executable_request(())?;
            client
                .action_input(flow_id, request.request_bytes().to_vec())
                .await?;
            let completion = request.run().await?;
            match &completion {
                Ok(output) => client.action_success_output(flow_id, output.clone()).await?,
                Err(error) => client.action_failure_output(flow_id, error.clone()).await?,
            }
            let _emitted = executor.complete_serialized(completion)?;
        }
        Ok(executor.into_state())
    }
}
