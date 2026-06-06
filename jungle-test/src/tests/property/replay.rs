use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::future::pending;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

pub struct ReplayRainforest(Arc<Mutex<ReplayInner>>);

struct ReplayInner {
    query: Vec<bool>,
    end: oneshot::Sender<()>,
}

pub struct ReplayRainforestAnimals;
impl Animals for ReplayRainforestAnimals {
    type List = jungle_sdk::typosaurus::collections::list::Empty;
}

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest";
    type Animals = ReplayRainforestAnimals;
}

impl ReplayRainforest {
    async fn next(&self) -> bool {
        let mut inner = self.0.lock().await;
        match inner.query.pop() {
            Some(value) => value,
            None => {
                let (replacement_end, _replacement_rx) = oneshot::channel();
                let end = std::mem::replace(&mut inner.end, replacement_end);
                drop(inner);
                let _ = end.send(());
                pending::<bool>().await
            }
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayState {
    color: bool,
    history: String,
}

trait ReplayTockRuntime {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send;
}

impl ReplayTockRuntime for () {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        std::future::ready(false)
    }
}

impl ReplayTockRuntime for ReplayRainforest {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        self.next()
    }
}

pub struct Tock;

#[jungle::effect(id = 1001)]
impl<J> Effect<J> for Tock
where
    J: ReplayTockRuntime + Sync,
{
    type In = ();
    type Out = bool;
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(jungle.run_tock().await) }
    }
}

pub struct Tick;

#[jungle::action]
impl Action for Tick {
    type Effect = Tock;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let tocked = output.map_err(|_err| Failure::from("tock should succeed"))?;
            if tocked {
                state.color = true;
                state.history.push('1');
            } else {
                state.color = false;
                state.history.push('0');
            }
        };
        Ok(__absorb_out_1)
    }
}
