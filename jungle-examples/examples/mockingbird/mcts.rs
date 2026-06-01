use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

pub trait SearchTree<Tag = ()> {
    type Error;
    type Data: Serialize + DeserializeOwned;

    fn select(&self, tag: Tag) -> impl Future<Output = Result<Self::Data, Self::Error>>;

    fn submit(
        &self,
        tag: Tag,
        data: Self::Data,
        score: f32,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}
