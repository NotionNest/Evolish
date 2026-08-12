use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    application::translation::result_validator::validate_adaptive_result,
    infrastructure::{
        ai::{
            contract_fixtures::{
                AiTranslationProvider, ProviderError, ProviderErrorKind, ProviderTranslationRequest,
            },
            http_client::SecureHttpClient,
            registry::{AiProviderRegistry, ProviderDescriptor},
        },
        credentials::system_store::{CredentialMetadata, CredentialPort},
    },
};

/// Explicitly supported OpenAI-compatible Chat Completions subset with strict JSON Schema output.
pub struct OpenAiCompatibleAdapter {
    endpoint: Url,
    model: String,
    credentials: Arc<dyn CredentialPort>,
    credential_metadata: CredentialMetadata,
    client: SecureHttpClient,
}

impl OpenAiCompatibleAdapter {
    /// Creates an adapter for the documented compatible Chat Completions subset.
    ///
    /// # Errors
    ///
    /// Returns a classified error when the endpoint or client cannot be initialized.
    pub fn new(
        endpoint: &str,
        model: &str,
        credentials: Arc<dyn CredentialPort>,
        credential_metadata: CredentialMetadata,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            endpoint: Url::parse(endpoint)
                .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?,
            model: model.to_owned(),
            credentials,
            credential_metadata,
            client: SecureHttpClient::new()
                .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?,
        })
    }

    #[must_use]
    ///
    /// # Panics
    ///
    /// Panics only when the product-owned compatible descriptor is removed.
    pub fn descriptor_from_registry() -> &'static ProviderDescriptor {
        AiProviderRegistry::new()
            .descriptor(crate::domain::provider::ProviderProtocol::OpenAiCompatible)
            .expect("OpenAI-compatible descriptor")
    }

    /// Executes a real structured-output capability check for the compatible protocol subset.
    ///
    /// # Errors
    ///
    /// Returns the same classified error as a translation request when capability is absent.
    pub async fn test_connection(
        &self,
        request: ProviderTranslationRequest,
        cancellation: CancellationToken,
    ) -> Result<(), ProviderError> {
        self.translate(request, cancellation).await.map(|_| ())
    }
}

#[async_trait]
impl AiTranslationProvider for OpenAiCompatibleAdapter {
    fn descriptor(&self) -> &ProviderDescriptor {
        Self::descriptor_from_registry()
    }

    async fn translate(
        &self,
        request: ProviderTranslationRequest,
        cancellation: CancellationToken,
    ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError> {
        if cancellation.is_cancelled() {
            return Err(ProviderError::new(ProviderErrorKind::Cancelled));
        }
        let secret = self
            .credentials
            .get(&self.credential_metadata)
            .map_err(|_| ProviderError::new(ProviderErrorKind::AuthenticationFailed))?;
        let url = self
            .endpoint
            .join("chat/completions")
            .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?;
        let payload = serde_json::json!({"model": self.model, "messages": [
            {"role": "system", "content": request.prompt().system_contract()},
            {"role": "developer", "content": request.prompt().mode_instruction()},
            {"role": "user", "content": request.prompt().user_content()}
        ], "response_format": {"type": "json_schema", "json_schema": {"name": "adaptive_translation", "strict": true, "schema": {"type": "object"}}}});
        let send = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", secret.expose()))
            .header(CONTENT_TYPE, "application/json")
            .timeout(request.timeout())
            .json(&payload)
            .send();
        let response = tokio::select! {
            result = send => result.map_err(|error| if error.is_timeout() { ProviderError::new(ProviderErrorKind::TimedOut) } else { ProviderError::new(ProviderErrorKind::Network) })?,
            () = cancellation.cancelled() => return Err(ProviderError::new(ProviderErrorKind::Cancelled)),
        };
        if !response.status().is_success() {
            return Err(ProviderError::new(map_status(response.status())));
        }
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseMalformed))?;
        let text = value
            .pointer("/choices/0/message/content")
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .ok_or(ProviderError::new(ProviderErrorKind::ResponseEmpty))?;
        let result = serde_json::from_str(text)
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        validate_adaptive_result(&result, request.intent())
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        Ok(result)
    }
}

fn map_status(status: reqwest::StatusCode) -> ProviderErrorKind {
    match status {
        reqwest::StatusCode::UNAUTHORIZED => ProviderErrorKind::AuthenticationFailed,
        reqwest::StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimited,
        reqwest::StatusCode::PAYMENT_REQUIRED => ProviderErrorKind::QuotaExceeded,
        reqwest::StatusCode::NOT_FOUND => ProviderErrorKind::ModelUnavailable,
        reqwest::StatusCode::FORBIDDEN => ProviderErrorKind::SafetyRefusal,
        _ => ProviderErrorKind::Network,
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::{
        ai::contract_fixtures::{AiTranslationProvider, provider_request},
        credentials::system_store::{
            CredentialMetadata, CredentialPort, CredentialSecret, CredentialStoreError,
        },
    };
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
    struct StaticCredentialStore;
    impl CredentialPort for StaticCredentialStore {
        fn set(
            &self,
            _: &CredentialMetadata,
            _: CredentialSecret,
        ) -> Result<(), CredentialStoreError> {
            Ok(())
        }
        fn get(&self, _: &CredentialMetadata) -> Result<CredentialSecret, CredentialStoreError> {
            Ok(CredentialSecret::new("evolish-secret-canary"))
        }
        fn delete(&self, _: &CredentialMetadata) -> Result<(), CredentialStoreError> {
            Ok(())
        }
    }
    #[tokio::test]
    async fn compatible_adapter_uses_chat_completions_structured_subset() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST")).and(matchers::path("/v1/chat/completions")).and(matchers::header("authorization", "Bearer evolish-secret-canary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices": [{"message": {"content": "{\"kind\":\"sentence\",\"schemaVersion\":1,\"primaryTranslation\":\"你好\"}"}}]}))).mount(&server).await;
        let adapter = super::OpenAiCompatibleAdapter::new(
            &format!("{}/v1/", server.uri()),
            "compat-model",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
        )
        .expect("adapter");
        adapter
            .test_connection(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("structured capability test");
    }

    #[tokio::test]
    async fn compatible_adapter_rejects_empty_completed_content() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"choices": [{"message": {"content": ""}}]})),
            )
            .mount(&server)
            .await;
        let adapter = super::OpenAiCompatibleAdapter::new(
            &format!("{}/v1/", server.uri()),
            "compat-model",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
        )
        .expect("adapter");

        let error = adapter
            .translate(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect_err("empty result");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::ResponseEmpty
        );
    }
}
