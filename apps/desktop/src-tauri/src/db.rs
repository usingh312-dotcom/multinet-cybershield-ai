use serde::Serialize;
use sqlx::migrate;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use thiserror::Error;

const DB_FILE: &str = "multinet-cybershield-ai.db";

static MIGRATOR: migrate::Migrator = migrate!("../../../database/sqlite/migrations");

#[derive(Debug, Error)]
pub enum DbError {
    #[error("failed to create application data directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] migrate::MigrateError),
}

pub struct DatabaseState {
    pub pool: SqlitePool,
    pub db_file: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct DbStatus {
    pub db_file: String,
    pub migrations_applied: i64,
    pub latest_version: Option<i64>,
    pub foreign_keys_enabled: bool,
}

pub async fn initialize_database(data_dir: &Path) -> Result<DatabaseState, DbError> {
    std::fs::create_dir_all(data_dir).map_err(|source| DbError::CreateDir {
        path: data_dir.to_path_buf(),
        source,
    })?;

    let db_file = data_dir.join(DB_FILE);
    let connect_options = SqliteConnectOptions::new()
        .filename(&db_file)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(DatabaseState {
        pool,
        db_file,
    })
}

pub async fn database_status(state: &DatabaseState) -> Result<DbStatus, DbError> {
    let row = sqlx::query("SELECT COUNT(*) AS applied, MAX(version) AS latest FROM _sqlx_migrations")
        .fetch_one(&state.pool)
        .await?;
    let foreign_keys = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(&state.pool)
        .await?;

    Ok(DbStatus {
        db_file: state.db_file.to_string_lossy().into_owned(),
        migrations_applied: row.try_get("applied")?,
        latest_version: row.try_get("latest")?,
        foreign_keys_enabled: foreign_keys.try_get::<i64, usize>(0)? == 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initializes_and_applies_foundation_migration() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        let status = database_status(&state).await.unwrap();

        assert_eq!(status.migrations_applied, 1);
        assert_eq!(status.latest_version, Some(1));
        assert!(status.foreign_keys_enabled);
        assert!(state.db_file.ends_with(DB_FILE));

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn enforces_foreign_keys() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();

        let orphan = sqlx::query(
            "INSERT INTO connection_metrics (interface_id, measured_at, source) \
             VALUES ('no-such-interface', '2026-01-01T00:00:00Z', 'test')",
        )
        .execute(&state.pool)
        .await;

        assert!(orphan.is_err(), "orphan metric insert must violate the foreign key");

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn never_touches_prewritten_schema_and_is_idempotent() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        initialize_database(&data_dir).await.unwrap();

        let tables = sqlx::query("SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(tables.try_get::<i64, _>("n").unwrap(), 15);

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
