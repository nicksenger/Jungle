//! Zoo crate focused on the gorilla ecosystem and probe flows.

pub mod animals;
pub mod effects;
pub mod probe;
pub mod state;

#[derive(jungle_sdk::Animals)]
pub struct ZooAnimals(animals::gorilla::Gorilla);

pub struct Zoo;
impl jungle_sdk::types::Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = ZooAnimals;
}
