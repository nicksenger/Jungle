use jungle_types::{AnimalStatesCompatible, Animals, Ecosystem};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Animals: Animals,
    for<'a> <T as Ecosystem>::Animals: AnimalStatesCompatible<&'a T>,
{
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}