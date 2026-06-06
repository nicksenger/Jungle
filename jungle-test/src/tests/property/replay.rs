use jungle_sdk::prelude::*;
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

pub struct Tock;

#[jungle::effect(id = 1001)]
impl Effect<ReplayRainforest> for Tock {
    type In = ();
    type Out = bool;
    type Err = ();

    fn effect(
        jungle: &ReplayRainforest,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(jungle.next().await) }
    }
}
