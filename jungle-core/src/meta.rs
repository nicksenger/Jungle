use jungle_types::{Action, Actions, Animal, Animals, Ecosystem, Instinct};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Animals: Animals,
{
    type Animals = <T::Animals as Animals>::List;

    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
