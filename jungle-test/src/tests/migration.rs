use fjall::{KeyspaceCreateOptions, Readable, SingleWriterTxDatabase};
use jungle_sdk::server::ServerBuilder;
use sqlx::PgPool;
use std::path::Path;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

const FJALL_KEYSPACES: [&str; 13] = [
    "jungle_schema_metadata",
    "journeys",
    "events",
    "event_timestamps",
    "work_items",
    "timer_tasks",
    "timer_due_index",
    "journey_leases",
    "owner_wakes",
    "animal_appearances",
    "animal_perturbations",
    "animal_generations",
    "journey_event_sequences",
];

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

    let listen_addr = super::reserve_local_addr();
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
            Ok((
                schema_version,
                journeys_exists,
                journeys_status_exists,
                events_exists,
                work_items_exists,
                work_items_status_exists,
            )) => {
                assert_eq!(schema_version, Some(0));
                assert!(journeys_exists);
                assert!(journeys_status_exists);
                assert!(events_exists);
                assert!(work_items_exists);
                assert!(work_items_status_exists);
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
async fn fjall_server_startup_runs_migrations() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.fjall");

    let listen_addr = super::reserve_local_addr();
    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .fjall_path(db_path)
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

    assert!(
        initialized,
        "fjall database directory was not created before timeout"
    );

    let (schema_version, keyspace_states) = fjall_migration_state(&db_path)
        .unwrap_or_else(|err| panic!("failed to read fjall database state after startup: {err}"));
    assert_eq!(schema_version, Some(0));
    for (name, exists) in keyspace_states {
        assert!(
            exists,
            "fjall keyspace should exist after migration: {name}"
        );
    }
}

async fn migration_state(
    connection_string: &str,
) -> Result<(Option<i32>, bool, bool, bool, bool, bool), sqlx::Error> {
    let pool = PgPool::connect(connection_string).await?;

    let schema_version =
        sqlx::query_scalar::<_, i32>("SELECT version FROM jungle_schema_metadata WHERE id = 1")
            .fetch_optional(&pool)
            .await?;
    let journeys_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'journeys')",
    )
    .fetch_one(&pool)
    .await?;
    let journeys_status_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'journeys' AND column_name = 'status')",
    )
    .fetch_one(&pool)
    .await?;
    let events_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'events')",
    )
    .fetch_one(&pool)
    .await?;
    let work_items_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'work_items')",
    )
    .fetch_one(&pool)
    .await?;
    let work_items_status_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'work_items' AND column_name = 'status')",
    )
    .fetch_one(&pool)
    .await?;

    pool.close().await;
    Ok((
        schema_version,
        journeys_exists,
        journeys_status_exists,
        events_exists,
        work_items_exists,
        work_items_status_exists,
    ))
}

fn fjall_migration_state(
    db_path: &Path,
) -> Result<(Option<u32>, Vec<(&'static str, bool)>), String> {
    let db = SingleWriterTxDatabase::builder(db_path)
        .open()
        .map_err(|err| err.to_string())?;
    let metadata = db
        .keyspace("jungle_schema_metadata", KeyspaceCreateOptions::default)
        .map_err(|err| err.to_string())?;
    let schema_version = db
        .read_tx()
        .get(&metadata, [1_u8])
        .map_err(|err| err.to_string())?
        .map(|value| {
            let bytes: [u8; 4] = value
                .as_ref()
                .try_into()
                .map_err(|_| format!("invalid schema version length: {}", value.len()))?;
            Ok::<u32, String>(u32::from_be_bytes(bytes))
        })
        .transpose()?;
    let keyspace_states = FJALL_KEYSPACES
        .into_iter()
        .map(|name| (name, db.keyspace_exists(name)))
        .collect();

    Ok((schema_version, keyspace_states))
}
