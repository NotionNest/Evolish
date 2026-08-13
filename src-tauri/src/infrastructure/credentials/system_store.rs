#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{CredentialMetadata, CredentialPort, CredentialSecret, CredentialStoreError};

    struct RecordingCredentialStore {
        write_count: Mutex<usize>,
        deleted: Mutex<bool>,
        secret: Mutex<Option<CredentialSecret>>,
    }

    impl CredentialPort for RecordingCredentialStore {
        fn set(
            &self,
            _metadata: &CredentialMetadata,
            secret: CredentialSecret,
        ) -> Result<(), CredentialStoreError> {
            *self.write_count.lock().expect("write count lock") += 1;
            *self.secret.lock().expect("secret lock") = Some(secret);
            Ok(())
        }

        fn get(
            &self,
            _metadata: &CredentialMetadata,
        ) -> Result<CredentialSecret, CredentialStoreError> {
            self.secret
                .lock()
                .expect("secret lock")
                .take()
                .ok_or(CredentialStoreError::NotFound)
        }

        fn delete(&self, _metadata: &CredentialMetadata) -> Result<(), CredentialStoreError> {
            *self.deleted.lock().expect("deleted lock") = true;
            Ok(())
        }
    }

    struct UnavailableCredentialStore;

    impl CredentialPort for UnavailableCredentialStore {
        fn set(
            &self,
            _metadata: &CredentialMetadata,
            _secret: CredentialSecret,
        ) -> Result<(), CredentialStoreError> {
            Err(CredentialStoreError::Unavailable)
        }

        fn delete(&self, _metadata: &CredentialMetadata) -> Result<(), CredentialStoreError> {
            Err(CredentialStoreError::Unavailable)
        }

        fn get(
            &self,
            _metadata: &CredentialMetadata,
        ) -> Result<CredentialSecret, CredentialStoreError> {
            Err(CredentialStoreError::Unavailable)
        }
    }

    #[test]
    fn secret_canary_cannot_appear_in_debug_or_serialized_credential_metadata() {
        let secret = super::CredentialSecret::new("evolish-secret-canary");
        let metadata = super::CredentialMetadata::new("provider-secret-reference");
        let debug = format!("{secret:?} {metadata:?}");
        let serialized = serde_json::to_string(&metadata).expect("metadata serializes");
        let captured_log = format!("credential update: {metadata:?}; {secret:?}");

        assert!(!debug.contains("evolish-secret-canary"));
        assert!(!serialized.contains("evolish-secret-canary"));
        assert!(!serialized.contains("api_key"));
        assert!(!captured_log.contains("evolish-secret-canary"));
    }

    #[test]
    fn credential_port_supports_secret_update_and_delete() {
        let store = RecordingCredentialStore {
            write_count: Mutex::new(0),
            deleted: Mutex::new(false),
            secret: Mutex::new(None),
        };
        let metadata = CredentialMetadata::new("provider-secret-reference");

        store
            .set(&metadata, CredentialSecret::new("first-value"))
            .expect("first secret write");
        store
            .set(&metadata, CredentialSecret::new("updated-value"))
            .expect("secret update");
        let updated = store.get(&metadata).expect("read updated secret");
        store.delete(&metadata).expect("secret deletion");

        assert_eq!(*store.write_count.lock().expect("write count lock"), 2);
        assert_eq!(updated.expose(), "updated-value");
        assert!(*store.deleted.lock().expect("deleted lock"));
    }

    #[test]
    fn unavailable_credential_store_has_no_plaintext_fallback() {
        let store = UnavailableCredentialStore;
        let metadata = CredentialMetadata::new("provider-secret-reference");

        assert_eq!(
            store.set(&metadata, CredentialSecret::new("evolish-secret-canary")),
            Err(CredentialStoreError::Unavailable)
        );
        assert_eq!(
            store.delete(&metadata),
            Err(CredentialStoreError::Unavailable)
        );
    }
}
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use keyring_core::{Entry, api::CredentialStoreApi};

/// Secret material whose debug representation, serialization, and persistence are forbidden.
pub struct CredentialSecret(SecretString);
impl CredentialSecret {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretString::from(value.into()))
    }
    /// Exposes secret bytes only to a platform credential adapter during a write.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}
impl core::fmt::Debug for CredentialSecret {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("CredentialSecret(REDACTED)")
    }
}

/// Non-secret reference that may be stored alongside a provider profile.
#[derive(Debug, Serialize)]
pub struct CredentialMetadata {
    secret_ref: String,
}
impl CredentialMetadata {
    #[must_use]
    pub fn new(secret_ref: impl Into<String>) -> Self {
        Self {
            secret_ref: secret_ref.into(),
        }
    }
    #[must_use]
    pub fn secret_ref(&self) -> &str {
        &self.secret_ref
    }
}

/// Platform credential-store boundary. Implementations must never persist a plaintext fallback.
pub trait CredentialPort: Send + Sync {
    /// Stores a secret in the platform credential store.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the platform credential store is unavailable or rejects the write.
    fn set(
        &self,
        metadata: &CredentialMetadata,
        secret: CredentialSecret,
    ) -> Result<(), CredentialStoreError>;
    /// Retrieves a secret only for an in-process provider adapter.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the secret does not exist or cannot be read.
    fn get(&self, metadata: &CredentialMetadata) -> Result<CredentialSecret, CredentialStoreError>;
    /// Deletes a secret from the platform credential store.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the platform credential store is unavailable or rejects deletion.
    fn delete(&self, metadata: &CredentialMetadata) -> Result<(), CredentialStoreError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialStoreError {
    Unavailable,
    NotFound,
    WriteFailed,
    DeleteFailed,
}

/// Native platform credential-store adapter. It has no filesystem or database fallback.
pub struct SystemCredentialStore;

impl CredentialPort for SystemCredentialStore {
    fn set(
        &self,
        metadata: &CredentialMetadata,
        secret: CredentialSecret,
    ) -> Result<(), CredentialStoreError> {
        system_entry(metadata)?
            .set_secret(secret.expose().as_bytes())
            .map_err(|_| CredentialStoreError::WriteFailed)
    }

    fn get(&self, metadata: &CredentialMetadata) -> Result<CredentialSecret, CredentialStoreError> {
        let bytes = system_entry(metadata)?
            .get_secret()
            .map_err(|_| CredentialStoreError::NotFound)?;
        let secret = String::from_utf8(bytes).map_err(|_| CredentialStoreError::NotFound)?;
        Ok(CredentialSecret::new(secret))
    }

    fn delete(&self, metadata: &CredentialMetadata) -> Result<(), CredentialStoreError> {
        system_entry(metadata)?
            .delete_credential()
            .map_err(|_| CredentialStoreError::DeleteFailed)
    }
}

fn system_entry(metadata: &CredentialMetadata) -> Result<Entry, CredentialStoreError> {
    const SERVICE: &str = "io.notionnest.evolish";
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new()
        .map_err(|_| CredentialStoreError::Unavailable)?;
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new()
        .map_err(|_| CredentialStoreError::Unavailable)?;
    #[cfg(target_os = "linux")]
    let store = zbus_secret_service_keyring_store::Store::new()
        .map_err(|_| CredentialStoreError::Unavailable)?;
    store
        .build(SERVICE, metadata.secret_ref(), None)
        .map_err(|_| CredentialStoreError::Unavailable)
}
