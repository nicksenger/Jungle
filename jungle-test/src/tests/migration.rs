use jungle_sdk::server::ServerBuilder;
use redb::{Database, ReadableDatabase, TableDefinition};
use sqlx::PgPool;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::path::Path;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

const REDB_SCHEMA_METADATA_TABLE: TableDefinition<u8, u32> =
    TableDefinition::new("jungle_schema_metadata");
const REDB_FLOWS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("flows");
const REDB_EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");

#[tokio::test]
async fn postgres_server_startup_runs_migrations() {
    let postgres = Postgres::default()
        .start()
        .await
        .expect("postgres testcontainer should start");
    let pg_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres mapped port should be available");
    let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");

    let listen_addr = reserve_local_addr();
    let server_task = tokio::spawn({
        let connection_string = connection_string.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .postgres_connection_string(connection_string)
                .run()
                .await
        }
    });

    let mut migrated = false;
    let mut last_error = String::new();
    for _ in 0..80 {
        match migration_state(&connection_string).await {
            Ok((schema_version, flows_exists, events_exists)) => {
                assert_eq!(schema_version, Some(0));
                assert!(flows_exists);
                assert!(events_exists);
                migrated = true;
                break;
            }
            Err(err) => {
                last_error = err.to_string();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    server_task.abort();
    let _ = server_task.await;

    assert!(
        migrated,
        "postgres migration did not complete before timeout: {last_error}"
    );
}

#[tokio::test]
async fn redb_server_startup_runs_migrations() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let mut initialized = false;
    for _ in 0..80 {
        if db_path.exists() {
            initialized = true;
            break;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    server_task.abort();
    let _ = server_task.await;

    assert!(initialized, "redb file was not created before timeout");

    let (schema_version, flows_exists, events_exists) = redb_migration_state(&db_path)
        .unwrap_or_else(|err| panic!("failed to read redb file state after startup: {err}"));
    assert_eq!(schema_version, Some(0));
    assert!(flows_exists);
    assert!(events_exists);
}

async fn migration_state(
    connection_string: &str,
) -> Result<(Option<i32>, bool, bool), sqlx::Error> {
    let pool = PgPool::connect(connection_string).await?;

    let schema_version =
        sqlx::query_scalar::<_, i32>("SELECT version FROM jungle_schema_metadata WHERE id = 1")
            .fetch_optional(&pool)
            .await?;
    let flows_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'flows')",
    )
    .fetch_one(&pool)
    .await?;
    let events_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'events')",
    )
    .fetch_one(&pool)
    .await?;

    pool.close().await;
    Ok((schema_version, flows_exists, events_exists))
}

fn redb_migration_state(db_path: &Path) -> Result<(Option<u32>, bool, bool), String> {
    let db = Database::open(db_path).map_err(|err| err.to_string())?;
    let read_txn = db.begin_read().map_err(|err| err.to_string())?;

    let metadata = read_txn
        .open_table(REDB_SCHEMA_METADATA_TABLE)
        .map_err(|err| err.to_string())?;
    let schema_version = metadata
        .get(1)
        .map_err(|err| err.to_string())?
        .map(|v| v.value());

    let flows_exists = read_txn.open_table(REDB_FLOWS_TABLE).is_ok();
    let events_exists = read_txn.open_table(REDB_EVENTS_TABLE).is_ok();

    Ok((schema_version, flows_exists, events_exists))
}

fn reserve_local_addr() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
        .expect("should bind temporary udp socket for test port reservation");
    socket
        .local_addr()
        .expect("temporary udp socket should expose local address")
}
