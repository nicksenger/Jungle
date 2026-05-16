use jungle_types::{
    AnimalEffectsCompatible, AnimalStatesCompatible, AnimalVersionIdentitiesUnique, Animals,
    Ecosystem,
};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Animals: Animals,
    <T as Ecosystem>::Animals: AnimalVersionIdentitiesUnique,
    for<'a> <T as Ecosystem>::Animals: AnimalStatesCompatible<&'a T>,
    for<'a> <T as Ecosystem>::Animals: AnimalEffectsCompatible<&'a T>,
{
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
