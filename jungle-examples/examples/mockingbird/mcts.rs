use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

pub trait SearchTree {
    type Error;
    type Data: Serialize + DeserializeOwned;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>>;

    fn submit(
        &self,
        data: Self::Data,
        score: f32,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}
