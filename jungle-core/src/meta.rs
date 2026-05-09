use jungle_types::{
    AnimaActionDependenciesCompatible, AnimaStatesCompatible, Animas, Ecosystem,
};

use crate::Jungle;

impl<T> Jungle for T
where
    T: Ecosystem,
    <T as Ecosystem>::Animas: Animas,
    for<'a> <T as Ecosystem>::Animas: AnimaStatesCompatible<&'a T>,
    for<'a> <T as Ecosystem>::Animas: AnimaActionDependenciesCompatible<&'a T>,
{
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>> {
        drop(self);
        std::future::ready(Ok(()))
    }
}
