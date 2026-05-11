use super::support::define_action;
use crate::state::{Carnivore, Herbivore, Omnivore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DietDependency {
    pub forage_gain: u32,
    pub hunt_gain: u32,
}

impl Default for DietDependency {
    fn default() -> Self {
        Self {
            forage_gain: 8,
            hunt_gain: 20,
        }
    }
}

impl<T> From<&T> for DietDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Forage,
    id = 50,
    dependency = DietDependency,
    in = Herbivore,
    out = u32,
    err = String,
    act = |dependency, herbivore| {
        let _ = herbivore;
        std::future::ready(Ok(dependency.forage_gain))
    }
);

define_action!(
    Graze,
    id = 51,
    dependency = DietDependency,
    in = Herbivore,
    out = String,
    err = String,
    act = |_dependency, herbivore| {
        std::future::ready(Ok(format!(
            "grazed on {}",
            herbivore.favorite_plant
        )))
    }
);

define_action!(
    Hunt,
    id = 52,
    dependency = DietDependency,
    in = Carnivore,
    out = u32,
    err = String,
    act = |dependency, carnivore| {
        let _ = carnivore;
        std::future::ready(Ok(dependency.hunt_gain))
    }
);

define_action!(
    UseTool,
    id = 53,
    dependency = DietDependency,
    in = Omnivore,
    out = String,
    err = String,
    act = |_dependency, omnivore| {
        std::future::ready(Ok(format!(
            "used a tool while foraging for {}",
            omnivore.favorite_plant
        )))
    }
);
