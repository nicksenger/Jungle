use jungle_sdk::server::ServerBuilder;
use redb::{Database, ReadableDatabase, TableDefinition};
use sqlx::PgPool;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

const REDB_SCHEMA_METADATA_TABLE: TableDefinition<u8, u32> =
    TableDefinition::new("jungle_schema_metadata");
const REDB_JOURNEYS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("journeys");
const REDB_EVENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("events");
const REDB_STEPS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("work_items");

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
async fn redb_server_startup_runs_migrations() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");

    let listen_addr = super::reserve_local_addr();
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

    let (schema_version, journeys_exists, events_exists, work_items_exists) =
        redb_migration_state(&db_path)
            .unwrap_or_else(|err| panic!("failed to read redb file state after startup: {err}"));
    assert_eq!(schema_version, Some(0));
    assert!(journeys_exists);
    assert!(events_exists);
    assert!(work_items_exists);
}

#[tokio::test]
#[ignore = "manual helper: regenerates SQLx offline cache into jungle-persist/.sqlx"]
async fn regenerate_sqlx_offline_schema_under_jungle_persist() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should be parent of jungle-test");
    assert!(
        workspace_root.join("Cargo.toml").is_file()
            && workspace_root
                .join("jungle-persist")
                .join("Cargo.toml")
                .is_file()
            && workspace_root
                .join("jungle-test")
                .join("Cargo.toml")
                .is_file(),
        "workspace root layout was not detected"
    );

    let postgres = Postgres::default()
        .start()
        .await
        .expect("postgres testcontainer should start");
    let pg_port = postgres
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres mapped port should be available");
    let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");
    ensure_sqlx_prepare_schema(&connection_string)
        .await
        .expect("sqlx prepare schema should initialize");

    let source_sqlx_dir = workspace_root.join("target").join("sqlx");
    if source_sqlx_dir.exists() {
        fs::remove_dir_all(&source_sqlx_dir)
            .expect("existing target/sqlx directory should be removable");
    }
    fs::create_dir_all(&source_sqlx_dir).expect("target/sqlx directory should be creatable");

    let probe_dir = workspace_root.join("target").join("sqlx-prepare-probe");
    if probe_dir.exists() {
        fs::remove_dir_all(&probe_dir).expect("existing sqlx probe directory should be removable");
    }
    fs::create_dir_all(probe_dir.join("src")).expect("sqlx probe src dir should be creatable");
    fs::write(
        probe_dir.join("Cargo.toml"),
        r#"[package]
name = "sqlx-prepare-probe"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls", "macros"] }
"#,
    )
    .expect("sqlx probe manifest should be writable");
    fs::write(
        probe_dir.join("src").join("main.rs"),
        r#"fn main() {
    let _ = sqlx::query!("SELECT version FROM jungle_schema_metadata WHERE id = 1");
}
"#,
    )
    .expect("sqlx probe source should be writable");

    let status = Command::new("cargo")
        .current_dir(&probe_dir)
        .env("SQLX_OFFLINE", "false")
        .env("SQLX_OFFLINE_DIR", &source_sqlx_dir)
        .env("DATABASE_URL", &connection_string)
        .args(["check"])
        .status()
        .expect("cargo check should execute");
    assert!(status.success(), "cargo check failed with status: {status}");

    let generated_files =
        list_files_recursive(&source_sqlx_dir).expect("target/sqlx should be readable");
    assert!(
        !generated_files.is_empty(),
        "expected sqlx cache files under target/sqlx, but found none"
    );

    assert!(
        source_sqlx_dir.is_dir(),
        "expected sqlx output at target/sqlx after cargo check with SQLX_OFFLINE_DIR"
    );

    let target_sqlx_dir = workspace_root.join("jungle-persist").join(".sqlx");
    if target_sqlx_dir.exists() {
        fs::remove_dir_all(&target_sqlx_dir)
            .expect("existing jungle-persist/.sqlx directory should be removable");
    }
    fs::create_dir_all(&target_sqlx_dir)
        .expect("jungle-persist/.sqlx directory should be creatable");
    copy_dir_all(&source_sqlx_dir, &target_sqlx_dir)
        .expect("target/sqlx should copy to jungle-persist/.sqlx");
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

fn redb_migration_state(db_path: &Path) -> Result<(Option<u32>, bool, bool, bool), String> {
    let db = Database::open(db_path).map_err(|err| err.to_string())?;
    let read_txn = db.begin_read().map_err(|err| err.to_string())?;

    let metadata = read_txn
        .open_table(REDB_SCHEMA_METADATA_TABLE)
        .map_err(|err| err.to_string())?;
    let schema_version = metadata
        .get(1)
        .map_err(|err| err.to_string())?
        .map(|v| v.value());

    let journeys_exists = read_txn.open_table(REDB_JOURNEYS_TABLE).is_ok();
    let events_exists = read_txn.open_table(REDB_EVENTS_TABLE).is_ok();
    let work_items_exists = read_txn.open_table(REDB_STEPS_TABLE).is_ok();

    Ok((
        schema_version,
        journeys_exists,
        events_exists,
        work_items_exists,
    ))
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn list_files_recursive(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        if file_type.is_dir() {
            files.extend(list_files_recursive(&entry_path)?);
        } else {
            files.push(entry_path);
        }
    }
    Ok(files)
}

async fn ensure_sqlx_prepare_schema(connection_string: &str) -> Result<(), sqlx::Error> {
    let pool = PgPool::connect(connection_string).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jungle_schema_metadata (
            id SMALLINT PRIMARY KEY,
            version INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS journeys (
            id UUID PRIMARY KEY,
            ordinal INTEGER NOT NULL,
            status SMALLINT NOT NULL,
            seed BYTEA NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            journey_id UUID NOT NULL REFERENCES journeys(id) ON DELETE CASCADE,
            sequence_id BIGINT NOT NULL,
            kind SMALLINT NOT NULL,
            data BYTEA NOT NULL,
            PRIMARY KEY (journey_id, sequence_id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS work_items (
            id UUID PRIMARY KEY,
            journey_id UUID NOT NULL REFERENCES journeys(id) ON DELETE CASCADE,
            kind SMALLINT NOT NULL,
            status SMALLINT NOT NULL,
            expiry TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}
