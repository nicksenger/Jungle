#[cfg(feature = "fjall")]
mod fjall;
#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "fjall")]
pub use fjall::migrate_fjall_v0;
#[cfg(feature = "postgres")]
pub use postgres::migrate_postgres_v0;
