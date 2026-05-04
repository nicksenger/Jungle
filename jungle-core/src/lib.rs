mod meta;

pub trait Jungle {
    type Animals;

    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>>;
}
