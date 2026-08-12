use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::domain::provider::{ProviderProfile, ProviderProtocol};

/// A profile summary safe for settings and IPC consumers; it never contains a credential value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoredProviderProfile {
    id: String,
    display_name: String,
    protocol: String,
    model: String,
    endpoint: Option<String>,
    enabled: bool,
    sort_order: u32,
    credential_configured: bool,
}

impl StoredProviderProfile {
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub const fn has_secret_reference(&self) -> bool {
        self.credential_configured
    }
}

/// `SQLx` repository for the immutable core provider-profile schema.
pub struct ProviderProfileRepository {
    pool: SqlitePool,
}

impl ProviderProfileRepository {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Saves one validated profile without receiving or persisting a credential value.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderProfileRepositoryError::Storage`] when `SQLite` rejects the write.
    pub async fn save(
        &self,
        profile: &ProviderProfile,
    ) -> Result<(), ProviderProfileRepositoryError> {
        let parameters_json = serde_json::json!({
            "timeout_seconds": profile.parameters().timeout().as_secs(),
            "options": profile.parameters().options(),
        })
        .to_string();
        sqlx::query(
            "INSERT INTO provider_profiles \
             (id, adapter_kind, display_name, endpoint, model, parameters_json, secret_ref, enabled, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind(profile.id().to_string())
        .bind(protocol_key(profile.protocol()))
        .bind(profile.display_name())
        .bind(profile.endpoint().map(|endpoint| endpoint.url().as_str()))
        .bind(profile.model())
        .bind(parameters_json)
        .bind(profile.secret_ref())
        .bind(i64::from(profile.enabled()))
        .bind(i64::from(profile.sort_order()))
        .execute(&self.pool)
        .await
        .map_err(|_| ProviderProfileRepositoryError::Storage)?;
        Ok(())
    }

    /// Returns all safe profile summaries in their persisted user order.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderProfileRepositoryError::Storage`] when `SQLite` cannot be queried.
    pub async fn list(&self) -> Result<Vec<StoredProviderProfile>, ProviderProfileRepositoryError> {
        let rows = sqlx::query(
            "SELECT id, adapter_kind, display_name, endpoint, model, enabled, sort_order, secret_ref \
             FROM provider_profiles ORDER BY sort_order ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| ProviderProfileRepositoryError::Storage)?;
        rows.into_iter()
            .map(|row| {
                Ok(StoredProviderProfile {
                    id: row.get("id"),
                    display_name: row.get("display_name"),
                    protocol: row.get("adapter_kind"),
                    model: row.get("model"),
                    endpoint: row.get("endpoint"),
                    enabled: row.get::<i64, _>("enabled") == 1,
                    sort_order: u32::try_from(row.get::<i64, _>("sort_order"))
                        .map_err(|_| ProviderProfileRepositoryError::Storage)?,
                    credential_configured: row.get::<Option<String>, _>("secret_ref").is_some(),
                })
            })
            .collect()
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
pub enum ProviderProfileRepositoryError {
    Storage,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        domain::{
            provider::{ProviderParameters, ProviderProfile, ProviderProtocol},
            translation::ProviderProfileId,
        },
        infrastructure::storage::database::Database,
    };

    #[tokio::test]
    async fn stores_multiple_profiles_in_order_without_storing_secret_material() {
        let database_file = tempfile::NamedTempFile::new().expect("temporary database file");
        let database = Database::open(database_file.path())
            .await
            .expect("migrated database");
        let repository = super::ProviderProfileRepository::new(database.pool().clone());
        let first = profile("Personal", "https://one.example/v1", 0, "ref-personal");
        let second = profile("Work", "https://two.example/v1", 1, "ref-work");

        repository.save(&first).await.expect("save first profile");
        repository.save(&second).await.expect("save second profile");

        let profiles = repository.list().await.expect("list profiles");
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].display_name(), "Personal");
        assert_eq!(profiles[1].display_name(), "Work");
        assert!(
            profiles
                .iter()
                .all(super::StoredProviderProfile::has_secret_reference)
        );
        let json = serde_json::to_string(&profiles).expect("profiles serialize");
        assert!(!json.contains("evolish-secret-canary"));
        assert!(!json.contains("api_key"));
    }

    fn profile(name: &str, endpoint: &str, sort_order: u32, secret_ref: &str) -> ProviderProfile {
        ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAiCompatible,
            name,
            Some(endpoint),
            "test-model",
            ProviderParameters::new(
                std::time::Duration::from_secs(30),
                json!({"temperature": 0.2}),
            )
            .expect("parameters"),
            Some(secret_ref),
            true,
            sort_order,
        )
        .expect("profile")
    }
}
