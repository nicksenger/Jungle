mod executor;
mod meta;

pub use executor::JungleExecutor;

pub trait Jungle {
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>>;
}
