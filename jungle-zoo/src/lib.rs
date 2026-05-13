//! Zoo crate with structural state, actions, and action adapters.

pub mod actions;
pub mod adapt;
pub mod animals;
pub mod probe;
pub mod state;
pub mod testing;

#[derive(jungle_sdk::Animals)]
pub struct ZooAnimals(animals::gorilla::Gorilla);

pub struct Zoo;
impl jungle_sdk::types::Ecosystem for Zoo {
    const NAME: &'static str = "zoo";
    type Animals = ZooAnimals;
}

impl From<&Zoo> for () {
    fn from(_value: &Zoo) -> Self {}
}
