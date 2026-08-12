use sqlx::{Row, SqlitePool};

/// Database-facing mode summary, ordered by the persisted user order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredTranslationMode {
    id: String,
    name: String,
}
impl StoredTranslationMode {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationModeRepositoryError {
    BuiltInImmutable,
    ReferencedByWorkspace,
    NotFound,
    OptimisticConflict,
    Storage,
}

pub struct TranslationModeRepository {
    pool: SqlitePool,
}
impl TranslationModeRepository {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    /// Returns all translation modes in their persisted display order.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationModeRepositoryError::Storage`] when `SQLite` fails.
    pub async fn list(&self) -> Result<Vec<StoredTranslationMode>, TranslationModeRepositoryError> {
        let rows = sqlx::query("SELECT id, name FROM translation_modes ORDER BY sort_order ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|_| TranslationModeRepositoryError::Storage)?;
        Ok(rows
            .into_iter()
            .map(|row| StoredTranslationMode {
                id: row.get("id"),
                name: row.get("name"),
            })
            .collect())
    }
    /// Deletes a custom mode only when no workspace uses it as a default.
    ///
    /// # Errors
    ///
    /// Returns a typed error for missing, built-in, referenced, or storage-failed modes.
    pub async fn delete(&self, id: &str) -> Result<(), TranslationModeRepositoryError> {
        let kind = sqlx::query("SELECT kind FROM translation_modes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| TranslationModeRepositoryError::Storage)?
            .ok_or(TranslationModeRepositoryError::NotFound)?
            .get::<String, _>("kind");
        if kind == "built_in" {
            return Err(TranslationModeRepositoryError::BuiltInImmutable);
        }
        let referenced =
            sqlx::query("SELECT 1 FROM translation_workspace_profiles WHERE default_mode_id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| TranslationModeRepositoryError::Storage)?;
        if referenced.is_some() {
            return Err(TranslationModeRepositoryError::ReferencedByWorkspace);
        }
        sqlx::query("DELETE FROM translation_modes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|_| TranslationModeRepositoryError::Storage)?;
        Ok(())
    }

    /// Advances a workspace profile's optimistic-lock version.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationModeRepositoryError::OptimisticConflict`] when the
    /// caller's version is stale.
    pub async fn advance_workspace_version(
        &self,
        id: &str,
        expected_version: u32,
    ) -> Result<u32, TranslationModeRepositoryError> {
        let result = sqlx::query("UPDATE translation_workspace_profiles SET version = version + 1 WHERE id = ? AND version = ?")
            .bind(id).bind(expected_version).execute(&self.pool).await.map_err(|_| TranslationModeRepositoryError::Storage)?;
        if result.rows_affected() != 1 {
            return Err(TranslationModeRepositoryError::OptimisticConflict);
        }
        expected_version
            .checked_add(1)
            .ok_or(TranslationModeRepositoryError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{Row, sqlite::SqlitePoolOptions};

    const CORE_SCHEMA: &str = include_str!("../../../migrations/0001_translation_core.sql");

    #[tokio::test]
    async fn core_schema_accepts_only_main_and_mini_roles_and_seeds_unconfigured_main() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("temporary SQLite database");
        sqlx::raw_sql(CORE_SCHEMA)
            .execute(&pool)
            .await
            .expect("schema");

        let main = sqlx::query("SELECT window_role, primary_provider_profile_id, enabled_provider_order_json FROM translation_workspace_profiles WHERE window_role = 'main'")
            .fetch_one(&pool).await.expect("main workspace seed");
        assert_eq!(main.get::<String, _>("window_role"), "main");
        assert!(
            main.get::<Option<String>, _>("primary_provider_profile_id")
                .is_none()
        );
        assert_eq!(main.get::<String, _>("enabled_provider_order_json"), "[]");

        let invalid = sqlx::query("INSERT INTO translation_workspace_profiles (id, window_role, enabled_provider_order_json, primary_target_language, secondary_target_language, default_mode_id, version, created_at, updated_at) VALUES ('workspace-invalid', 'floating', '[]', 'en', 'zh-CN', '018f0a10-0000-7000-8000-000000000001', 1, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z')")
            .execute(&pool).await;
        assert!(invalid.is_err());
    }

    #[tokio::test]
    async fn repository_preserves_mode_order_and_rejects_deletion_of_builtin_or_referenced_modes() {
        let pool = migrated_pool().await;
        let repository = super::TranslationModeRepository::new(pool.clone());

        let modes = repository.list().await.expect("seeded modes");
        assert_eq!(modes.len(), 5);
        assert_eq!(modes[0].name(), "Standard");
        assert_eq!(
            repository.delete(modes[0].id()).await,
            Err(super::TranslationModeRepositoryError::BuiltInImmutable)
        );

        sqlx::query("INSERT INTO translation_modes (id, kind, name, instruction, version, enabled, sort_order, created_at, updated_at) VALUES ('018f0a10-0000-7000-8000-000000000006', 'custom', 'Personal', 'Prefer simple language.', 1, 1, 5, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z')").execute(&pool).await.expect("custom seed");
        sqlx::query("UPDATE translation_workspace_profiles SET default_mode_id = '018f0a10-0000-7000-8000-000000000006' WHERE window_role = 'main'").execute(&pool).await.expect("reference custom mode");
        assert_eq!(
            repository
                .delete("018f0a10-0000-7000-8000-000000000006")
                .await,
            Err(super::TranslationModeRepositoryError::ReferencedByWorkspace)
        );
    }

    #[tokio::test]
    async fn configured_workspace_requires_enabled_primary_membership_and_an_enabled_default_mode()
    {
        let pool = migrated_pool().await;
        sqlx::query("INSERT INTO provider_profiles (id, adapter_kind, display_name, model, parameters_json, enabled, sort_order, created_at, updated_at) VALUES ('provider-disabled', 'openai', 'Disabled', 'gpt', '{}', 0, 0, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z')").execute(&pool).await.expect("provider seed");
        let disabled_primary = sqlx::query("UPDATE translation_workspace_profiles SET primary_provider_profile_id = 'provider-disabled', enabled_provider_order_json = '[\"provider-disabled\"]' WHERE window_role = 'main'").execute(&pool).await;
        assert!(disabled_primary.is_err());

        sqlx::query("INSERT INTO provider_profiles (id, adapter_kind, display_name, model, parameters_json, enabled, sort_order, created_at, updated_at) VALUES ('provider-enabled', 'openai', 'Enabled', 'gpt', '{}', 1, 1, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z')").execute(&pool).await.expect("provider seed");
        let missing_order_membership = sqlx::query("UPDATE translation_workspace_profiles SET primary_provider_profile_id = 'provider-enabled', enabled_provider_order_json = '[]' WHERE window_role = 'main'").execute(&pool).await;
        assert!(missing_order_membership.is_err());
    }

    #[tokio::test]
    async fn workspace_updates_require_the_expected_optimistic_lock_version() {
        let repository = super::TranslationModeRepository::new(migrated_pool().await);

        assert_eq!(
            repository
                .advance_workspace_version("workspace-main", 0)
                .await,
            Err(super::TranslationModeRepositoryError::OptimisticConflict)
        );
        assert_eq!(
            repository
                .advance_workspace_version("workspace-main", 1)
                .await,
            Ok(2)
        );
        assert_eq!(
            repository
                .advance_workspace_version("workspace-main", 1)
                .await,
            Err(super::TranslationModeRepositoryError::OptimisticConflict)
        );
    }

    async fn migrated_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("temporary SQLite database");
        sqlx::raw_sql(CORE_SCHEMA)
            .execute(&pool)
            .await
            .expect("schema");
        pool
    }
}
