use futures::channel::oneshot;
use futures::SinkExt;
use jungle_client::{RunnerChannelMessage, RunnerChannelResponse, RunnerChannelTx};
use jungle_types::{
    Animal, AnimalObservation, AnimalPerturbation, BuildFlowWithContext, ContextExecutor, DynFlow,
    ExecutorError, ObservationAdapter, PerturbationAdapter, RunnerOut,
};
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
        journey_id: Uuid,
        mut tx: RunnerChannelTx,
    ) -> Result<A::State, ExecutorError>
    where
        A: Animal + AnimalObservation + AnimalPerturbation,
        A::Journey: BuildFlowWithContext<(*const T, DynFlow<A::State>), Output = DynFlow<A::State>>,
    {
        let mut executor = ContextExecutor::<T, A>::new(&self.jungle, state);
        if let Some(appearance) =
            <<A as AnimalObservation>::Adapter as ObservationAdapter<A>>::snapshot(
                executor.state(),
            )?
        {
            send_history(
                &mut tx,
                RunnerOut::Appearance {
                    data: appearance,
                    uuid: journey_id,
                },
            )
            .await?;
        }
        while !executor.is_complete() {
            process_perturbations(&mut executor, journey_id, &mut tx).await?;
            let request = executor.next_executable_request(())?;
            send_history(
                &mut tx,
                RunnerOut::ActionInput {
                    data: request.request_bytes().to_vec(),
                    uuid: journey_id,
                },
            )
            .await?;
            let completion = request.run().await?;
            match &completion {
                Ok(output) => {
                    send_history(
                        &mut tx,
                        RunnerOut::ActionSuccessOutput {
                            data: output.clone(),
                            uuid: journey_id,
                        },
                    )
                    .await?
                }
                Err(error) => {
                    send_history(
                        &mut tx,
                        RunnerOut::ActionFailureOutput {
                            data: error.clone(),
                            uuid: journey_id,
                        },
                    )
                    .await?
                }
            }
            let _emitted = executor.complete_serialized(completion)?;
            if let Some(appearance) = <<A as AnimalObservation>::Adapter as ObservationAdapter<
                A,
            >>::snapshot(executor.state())?
            {
                send_history(
                    &mut tx,
                    RunnerOut::Appearance {
                        data: appearance,
                        uuid: journey_id,
                    },
                )
                .await?;
            }
        }
        Ok(executor.into_state())
    }
}

async fn send_history(tx: &mut RunnerChannelTx, history: RunnerOut) -> Result<(), ExecutorError> {
    let (done_tx, done_rx) = oneshot::channel();
    tx.send((RunnerChannelMessage::History(history), done_tx))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
    match done_rx
        .await
        .map_err(|_| ExecutorError::ClientTransportAckDropped)??
    {
        RunnerChannelResponse::Ack => Ok(()),
        RunnerChannelResponse::ClaimedPerturbation(_) => Err(ExecutorError::ClientTransport(
            "runner expected ack response for history message".to_string(),
        )),
    }
}

async fn process_perturbations<A, Ctx>(
    executor: &mut ContextExecutor<'_, Ctx, A>,
    journey_id: Uuid,
    tx: &mut RunnerChannelTx,
) -> Result<(), ExecutorError>
where
    A: Animal + AnimalPerturbation,
    Ctx: 'static,
    A::Journey: BuildFlowWithContext<(*const Ctx, DynFlow<A::State>), Output = DynFlow<A::State>>,
{
    if !<<A as AnimalPerturbation>::Adapter as PerturbationAdapter<A>>::enabled() {
        return Ok(());
    }

    loop {
        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::ClaimAnimalPerturbation { journey_id },
            done_tx,
        ))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
        let claimed = match done_rx
            .await
            .map_err(|_| ExecutorError::ClientTransportAckDropped)??
        {
            RunnerChannelResponse::ClaimedPerturbation(claimed) => claimed,
            RunnerChannelResponse::Ack => {
                return Err(ExecutorError::ClientTransport(
                    "runner expected claimed perturbation response".to_string(),
                ));
            }
        };

        let Some(claimed) = claimed else {
            break;
        };

        {
            let state = executor.state_mut();
            let applied = <<A as AnimalPerturbation>::Adapter as PerturbationAdapter<A>>::apply(
                state,
                &claimed.data,
            )?;
            if !applied {
                break;
            }
        }

        let (done_tx, done_rx) = oneshot::channel();
        tx.send((
            RunnerChannelMessage::AckAnimalPerturbation {
                journey_id,
                perturbation_id: claimed.id,
            },
            done_tx,
        ))
        .await
        .map_err(|_| ExecutorError::ClientTransportClosed)?;
        match done_rx
            .await
            .map_err(|_| ExecutorError::ClientTransportAckDropped)??
        {
            RunnerChannelResponse::Ack => {}
            RunnerChannelResponse::ClaimedPerturbation(_) => {
                return Err(ExecutorError::ClientTransport(
                    "runner expected ack response for perturbation ack".to_string(),
                ));
            }
        }
    }

    Ok(())
}
