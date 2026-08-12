#[cfg(test)]
mod tests {
    use super::super::migrations;

    #[tokio::test]
    async fn bootstrap_applies_migrations_with_foreign_keys_and_wal_idempotently() {
        let database = tempfile::NamedTempFile::new().expect("temporary database file");
        let first = super::Database::open(database.path())
            .await
            .expect("first bootstrap");
        assert_eq!(first.foreign_keys_enabled().await.expect("foreign keys"), 1);
        assert_eq!(first.journal_mode().await.expect("journal mode"), "wal");
        assert_eq!(
            migrations::applied_version_count(first.pool())
                .await
                .expect("migration count"),
            2
        );

        let second = super::Database::open(database.path())
            .await
            .expect("idempotent bootstrap");
        assert_eq!(
            migrations::applied_version_count(second.pool())
                .await
                .expect("migration count"),
            2
        );
    }

    #[tokio::test]
    async fn migration_checksum_mismatch_prevents_an_unsafe_restart() {
        let database = tempfile::NamedTempFile::new().expect("temporary database file");
        let first = super::Database::open(database.path())
            .await
            .expect("first bootstrap");
        sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 2")
            .execute(first.pool())
            .await
            .expect("corrupt migration metadata for test");
        drop(first);

        let restart = super::Database::open(database.path()).await;
        assert!(matches!(restart, Err(super::DatabaseError::Migrate(_))));
    }

    #[tokio::test]
    async fn sqlite_stores_a_secret_reference_but_never_the_secret_canary() {
        let database = tempfile::NamedTempFile::new().expect("temporary database file");
        let store = super::Database::open(database.path())
            .await
            .expect("database bootstrap");
        let secret_canary = "evolish-secret-canary";

        sqlx::query(
            "INSERT INTO provider_profiles \
             (id, adapter_kind, display_name, endpoint, model, parameters_json, secret_ref, enabled, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("profile-id")
        .bind("openai-compatible")
        .bind("Test profile")
        .bind("https://example.invalid/v1")
        .bind("test-model")
        .bind("{}")
        .bind("provider-secret-reference")
        .bind(1_i64)
        .bind(99_i64)
        .bind("2026-08-12T00:00:00Z")
        .bind("2026-08-12T00:00:00Z")
        .execute(store.pool())
        .await
        .expect("provider profile with reference");

        let secret_refs: Vec<String> = sqlx::query_scalar(
            "SELECT secret_ref FROM provider_profiles WHERE secret_ref IS NOT NULL",
        )
        .fetch_all(store.pool())
        .await
        .expect("stored secret references");
        assert_eq!(secret_refs, ["provider-secret-reference"]);
        assert!(
            !secret_refs
                .iter()
                .any(|reference| reference == secret_canary)
        );

        store.pool().close().await;
        let mut sqlite_bytes = std::fs::read(database.path()).expect("read SQLite database");
        let wal_path = format!("{}-wal", database.path().display());
        if let Ok(wal_bytes) = std::fs::read(wal_path) {
            sqlite_bytes.extend(wal_bytes);
        }
        assert!(
            !sqlite_bytes
                .windows(secret_canary.len())
                .any(|bytes| bytes == secret_canary.as_bytes())
        );
    }
}
use std::{path::Path, str::FromStr};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use super::migrations;

/// The Rust-owned `SQLite` connection pool for all persisted Evolish data.
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens a persistent database, enables its safety pragmas, and applies migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, configured, or safely migrated.
    pub async fn open(path: &Path) -> Result<Self, DatabaseError> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .map_err(DatabaseError::Open)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(DatabaseError::Open)?;
        migrations::run(&pool)
            .await
            .map_err(DatabaseError::Migrate)?;
        Ok(Self { pool })
    }
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Returns the active foreign-key enforcement flag.
    ///
    /// # Errors
    ///
    /// Returns an `SQLx` error if the pragma cannot be queried.
    pub async fn foreign_keys_enabled(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&self.pool)
            .await
    }
    /// Returns the active journal mode.
    ///
    /// # Errors
    ///
    /// Returns an `SQLx` error if the pragma cannot be queried.
    pub async fn journal_mode(&self) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    Open(sqlx::Error),
    Migrate(sqlx::migrate::MigrateError),
}

impl core::fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Open(error) => write!(formatter, "could not open Evolish database: {error}"),
            Self::Migrate(error) => {
                write!(formatter, "could not migrate Evolish database: {error}")
            }
        }
    }
}

impl std::error::Error for DatabaseError {}
