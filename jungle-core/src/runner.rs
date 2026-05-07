use jungle_types::{BuildFlowWithContext, ContextExecutor, Creature, DynFlow, ExecutorError};

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
    pub async fn spawn<A>(&self, state: A::State) -> Result<A::State, ExecutorError>
    where
        A: Creature,
        A::Instinct:
            BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = ContextExecutor::<T, A>::new(&self.jungle, state);
        while !executor.is_complete() {
            let request = executor.next_executable_request(())?;
            let completion = request.run().await?;
            let _emitted = executor.complete_serialized(completion)?;
        }
        Ok(executor.into_state())
    }
}
