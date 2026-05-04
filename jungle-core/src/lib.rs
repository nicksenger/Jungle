mod meta;

pub trait Jungle {
    type Animals;

    async fn manifest(self) -> Result<(), jungle_types::Error>;
}
