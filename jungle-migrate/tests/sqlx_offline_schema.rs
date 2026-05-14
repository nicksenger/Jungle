#![cfg(feature = "postgres")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[tokio::test]
#[ignore = "manual helper: regenerates SQLx offline cache into jungle-persist/.sqlx"]
async fn regenerate_sqlx_offline_schema_under_jungle_persist() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should be parent of jungle-migrate");
    assert!(
        workspace_root.join("Cargo.toml").is_file()
            && workspace_root
                .join("jungle-migrate")
                .join("Cargo.toml")
                .is_file()
            && workspace_root
                .join("jungle-persist")
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

    // 1) Fresh container is up, now run jungle-migrate migrations against it.
    let pool = sqlx::PgPool::connect(&connection_string)
        .await
        .expect("postgres pool should initialize for sqlx prepare");
    jungle_migrate::migrate_postgres_v0(&pool)
        .await
        .expect("sqlx prepare schema migrations should initialize");
    let schema_version =
        sqlx::query_scalar::<_, i32>("SELECT version FROM jungle_schema_metadata WHERE id = 1")
            .fetch_optional(&pool)
            .await
            .expect("schema metadata query should succeed after migration");
    assert_eq!(
        schema_version,
        Some(0),
        "migration should initialize schema version row"
    );
    pool.close().await;

    // 2) Compile jungle-persist against the migrated container with SQLX_OFFLINE_DIR set.
    let source_sqlx_dir = workspace_root.join("target").join("sqlx");
    if source_sqlx_dir.exists() {
        fs::remove_dir_all(&source_sqlx_dir)
            .expect("existing target/sqlx directory should be removable");
    }
    fs::create_dir_all(&source_sqlx_dir).expect("target/sqlx directory should be creatable");

    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .env("SQLX_OFFLINE", "false")
        .env("SQLX_OFFLINE_DIR", &source_sqlx_dir)
        .env("DATABASE_URL", &connection_string)
        .args(["check", "-p", "jungle-persist", "--features", "postgres"])
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

    // 3) Verify the offline schema was generated under jungle-persist/.sqlx.
    let copied_files =
        list_files_recursive(&target_sqlx_dir).expect("jungle-persist/.sqlx should be readable");
    assert!(
        !copied_files.is_empty(),
        "expected generated SQLx offline schema under jungle-persist/.sqlx, but found none"
    );
    assert_eq!(
        copied_files.len(),
        generated_files.len(),
        "copied SQLx cache file count should match generated file count"
    );
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
