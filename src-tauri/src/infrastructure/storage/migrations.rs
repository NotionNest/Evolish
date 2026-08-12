use sqlx::{SqlitePool, migrate::Migrator};

/// The immutable migration sequence for Evolish's local translation store.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Runs and checksum-validates every migration before the application exposes a window.
///
/// # Errors
///
/// Returns the underlying `SQLx` migration error when the store cannot be safely upgraded.
pub async fn run(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Returns the number of applied migrations for bootstrap and upgrade tests.
///
/// # Errors
///
/// Returns the underlying `SQLx` error if migration metadata cannot be read.
pub async fn applied_version_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}
