use sqlx::{Row, SqlitePool};

use crate::domain::{
    language::Language,
    provider::{ProviderProfile, ProviderProtocol},
};

/// Immutable settings snapshot read consistently for a new translation session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceProfileSnapshot {
    primary_provider_id: Option<String>,
    enabled_provider_ids: Vec<String>,
    version: u32,
}

impl WorkspaceProfileSnapshot {
    #[must_use]
    pub const fn is_configured(&self) -> bool {
        self.primary_provider_id.is_some()
    }

    #[must_use]
    pub fn primary_provider_id(&self) -> Option<String> {
        self.primary_provider_id.clone()
    }

    #[must_use]
    pub fn enabled_provider_ids(&self) -> &[String] {
        &self.enabled_provider_ids
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }
}

/// Transactional repository for the closed main/mini workspace configuration.
pub struct WorkspaceProfileRepository {
    pool: SqlitePool,
}

impl WorkspaceProfileRepository {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Reads the main workspace's immutable configuration snapshot.
    ///
    /// # Errors
    ///
    /// Returns a typed error if storage is unavailable or corrupted.
    pub async fn read_main(
        &self,
    ) -> Result<WorkspaceProfileSnapshot, WorkspaceProfileRepositoryError> {
        let row = sqlx::query("SELECT primary_provider_profile_id, enabled_provider_order_json, version FROM translation_workspace_profiles WHERE window_role = 'main'")
            .fetch_one(&self.pool).await.map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        let enabled_provider_ids =
            serde_json::from_str(&row.get::<String, _>("enabled_provider_order_json"))
                .map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        Ok(WorkspaceProfileSnapshot {
            primary_provider_id: row.get("primary_provider_profile_id"),
            enabled_provider_ids,
            version: u32::try_from(row.get::<i64, _>("version"))
                .map_err(|_| WorkspaceProfileRepositoryError::Storage)?,
        })
    }

    /// Inserts the first provider and chooses it as primary in one `SQLite` transaction.
    ///
    /// # Errors
    ///
    /// Returns an error without creating a provider row when workspace validation fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn configure_first_provider(
        &self,
        profile: &ProviderProfile,
        primary_language: Language,
        secondary_language: Language,
        default_mode_id: &str,
        expected_version: u32,
    ) -> Result<WorkspaceProfileSnapshot, WorkspaceProfileRepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        let parameters_json = serde_json::json!({
            "timeout_seconds": profile.parameters().timeout().as_secs(),
            "options": profile.parameters().options(),
        })
        .to_string();
        sqlx::query("INSERT INTO provider_profiles (id, adapter_kind, display_name, endpoint, model, parameters_json, secret_ref, enabled, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))")
            .bind(profile.id().to_string()).bind(protocol_key(profile.protocol())).bind(profile.display_name())
            .bind(profile.endpoint().map(|endpoint| endpoint.url().as_str())).bind(profile.model()).bind(parameters_json)
            .bind(profile.secret_ref()).bind(i64::from(profile.enabled())).bind(i64::from(profile.sort_order()))
            .execute(&mut *transaction).await.map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        let enabled = serde_json::to_string(&[profile.id().to_string()])
            .map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        let result = sqlx::query("UPDATE translation_workspace_profiles SET primary_provider_profile_id = ?, enabled_provider_order_json = ?, primary_target_language = ?, secondary_target_language = ?, default_mode_id = ?, version = version + 1, updated_at = datetime('now') WHERE window_role = 'main' AND version = ?")
            .bind(profile.id().to_string()).bind(enabled).bind(primary_language.bcp47()).bind(secondary_language.bcp47()).bind(default_mode_id).bind(expected_version)
            .execute(&mut *transaction).await.map_err(|_| WorkspaceProfileRepositoryError::InvalidReference)?;
        if result.rows_affected() != 1 {
            return Err(WorkspaceProfileRepositoryError::OptimisticConflict);
        }
        transaction
            .commit()
            .await
            .map_err(|_| WorkspaceProfileRepositoryError::Storage)?;
        self.read_main().await
    }
}

fn protocol_key(protocol: ProviderProtocol) -> &'static str {
    match protocol {
        ProviderProtocol::OpenAi => "openai",
        ProviderProtocol::Anthropic => "anthropic",
        ProviderProtocol::Gemini => "gemini",
        ProviderProtocol::OpenAiCompatible => "openai_compatible",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceProfileRepositoryError {
    Storage,
    InvalidReference,
    OptimisticConflict,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        domain::{
            language::Language,
            provider::{ProviderParameters, ProviderProfile, ProviderProtocol},
            translation::ProviderProfileId,
        },
        infrastructure::storage::database::Database,
    };

    #[tokio::test]
    async fn empty_main_workspace_is_readable_then_first_provider_is_configured_atomically() {
        let database_file = tempfile::NamedTempFile::new().expect("temporary database file");
        let database = Database::open(database_file.path())
            .await
            .expect("migrated database");
        let repository = super::WorkspaceProfileRepository::new(database.pool().clone());

        let initial = repository
            .read_main()
            .await
            .expect("unconfigured workspace");
        assert!(!initial.is_configured());

        let profile = provider("First service");
        let configured = repository
            .configure_first_provider(
                &profile,
                Language::English,
                Language::ChineseSimplified,
                "018f0a10-0000-7000-8000-000000000001",
                initial.version(),
            )
            .await
            .expect("atomic first configuration");
        assert!(configured.is_configured());
        assert_eq!(
            configured.primary_provider_id(),
            Some(profile.id().to_string())
        );
        assert_eq!(
            configured.enabled_provider_ids(),
            &[profile.id().to_string()]
        );
        assert_eq!(configured.version(), initial.version() + 1);
    }

    #[tokio::test]
    async fn invalid_workspace_reference_rolls_back_first_provider_creation() {
        let database_file = tempfile::NamedTempFile::new().expect("temporary database file");
        let database = Database::open(database_file.path())
            .await
            .expect("migrated database");
        let repository = super::WorkspaceProfileRepository::new(database.pool().clone());
        let profile = provider("Rejected service");

        let result = repository
            .configure_first_provider(
                &profile,
                Language::English,
                Language::ChineseSimplified,
                "missing-mode",
                1,
            )
            .await;
        assert_eq!(
            result,
            Err(super::WorkspaceProfileRepositoryError::InvalidReference)
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM provider_profiles")
            .fetch_one(database.pool())
            .await
            .expect("profile count");
        assert_eq!(count, 0);
    }

    fn provider(name: &str) -> ProviderProfile {
        ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAiCompatible,
            name,
            Some("https://example.com/v1"),
            "test-model",
            ProviderParameters::new(std::time::Duration::from_secs(30), json!({}))
                .expect("parameters"),
            Some("credential-reference"),
            true,
            0,
        )
        .expect("profile")
    }
}
