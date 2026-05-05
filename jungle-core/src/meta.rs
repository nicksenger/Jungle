use jungle_types::{CreatureStatesCompatible, Creatures, Ecosystem};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Creatures: Creatures,
    for<'a> <T as Ecosystem>::Creatures: CreatureStatesCompatible<&'a T>,
{
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
