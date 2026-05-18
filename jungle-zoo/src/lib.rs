//! Zoo crate focused on the gorilla ecosystem and probe flows.

use jungle_sdk::prelude::*;

pub mod animals;
pub mod effects;
pub mod probe;
pub mod state;

#[derive(Animals)]
pub struct ZooAnimals(animals::gorilla::Gorilla);

pub struct Zoo;
impl Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = ZooAnimals;
}
