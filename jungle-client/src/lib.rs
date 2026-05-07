//! Client transport contracts for the Jungle workspace.

use futures::Stream;
use jungle_types::{ClientIn, ClientOut};
use std::pin::Pin;

pub trait JungleClient {
    fn transport<In>(
        &self,
        input: In,
    ) -> Pin<Box<dyn Stream<Item = ClientOut> + Send + 'static>>
    where
        In: Stream<Item = ClientIn> + Send + 'static;
}
