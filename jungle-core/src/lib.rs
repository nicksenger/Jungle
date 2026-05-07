mod executor;
mod meta;
mod runner;
mod worker;

pub use executor::JungleExecutor;
pub use runner::JungleRunner;
pub use worker::JungleWorker;

pub trait Jungle {
    fn manifest(self) -> impl std::future::Future<Output = Result<(), jungle_types::Error>>;
}
