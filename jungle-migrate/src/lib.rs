#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "redb")]
mod redb;

#[cfg(feature = "postgres")]
pub use postgres::migrate_postgres_v0;
#[cfg(feature = "redb")]
pub use redb::migrate_redb_v0;
