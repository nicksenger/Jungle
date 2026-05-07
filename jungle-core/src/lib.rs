mod executor;
mod meta;
mod runner;

pub use executor::JungleExecutor;
pub use runner::JungleRunner;

pub trait Jungle {
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>>;
}
