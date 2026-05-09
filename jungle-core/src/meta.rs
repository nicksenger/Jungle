use jungle_types::{
    AnimaActionDependenciesCompatible, AnimaStatesCompatible, Animae, Ecosystem,
};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Animae: Animae,
    for<'a> <T as Ecosystem>::Animae: AnimaStatesCompatible<&'a T>,
    for<'a> <T as Ecosystem>::Animae: AnimaActionDependenciesCompatible<&'a T>,
{
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
