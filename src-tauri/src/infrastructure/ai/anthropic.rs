use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue};
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

const ANTHROPIC_VERSION: &str = "2023-06-01";
const ADAPTIVE_TRANSLATION_TOOL: &str = "adaptive_translation";

/// Native Anthropic Messages API adapter using forced tool-use structured output.
pub struct AnthropicAdapter {
    endpoint: Url,
    model: String,
    credentials: Arc<dyn CredentialPort>,
    credential_metadata: CredentialMetadata,
    client: SecureHttpClient,
}

impl AnthropicAdapter {
    /// Creates a native adapter for an Anthropic Messages API endpoint.
    ///
    /// # Errors
    ///
    /// Returns a classified error when endpoint or HTTP client setup is invalid.
    pub fn new(
        endpoint: &str,
        model: &str,
        credentials: Arc<dyn CredentialPort>,
        credential_metadata: CredentialMetadata,
        _registry: &AiProviderRegistry,
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

    /// Executes a real structured-output capability check instead of accepting a bare HTTP success.
    ///
    /// # Errors
    ///
    /// Returns the same classified error as a translation request when the model lacks required capability.
    pub async fn test_connection(
        &self,
        request: ProviderTranslationRequest,
        cancellation: CancellationToken,
    ) -> Result<(), ProviderError> {
        self.translate(request, cancellation).await.map(|_| ())
    }
}

#[async_trait]
impl AiTranslationProvider for AnthropicAdapter {
    fn descriptor(&self) -> &ProviderDescriptor {
        AiProviderRegistry::new()
            .descriptor(crate::domain::provider::ProviderProtocol::Anthropic)
            .expect("Anthropic descriptor")
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
            .join("messages")
            .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?;
        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": 2048,
            "system": format!("{}\n\n{}", request.prompt().system_contract(), request.prompt().mode_instruction()),
            "messages": [{"role": "user", "content": request.prompt().user_content()}],
            "tools": [{
                "name": ADAPTIVE_TRANSLATION_TOOL,
                "description": "Return the complete validated adaptive translation result.",
                "input_schema": {"type": "object"}
            }],
            "tool_choice": {"type": "tool", "name": ADAPTIVE_TRANSLATION_TOOL}
        });
        let send = self
            .client
            .post(url)
            .header(HeaderName::from_static("x-api-key"), secret.expose())
            .header(
                HeaderName::from_static("anthropic-version"),
                HeaderValue::from_static(ANTHROPIC_VERSION),
            )
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
        if value.get("stop_reason").and_then(serde_json::Value::as_str) == Some("refusal") {
            return Err(ProviderError::new(ProviderErrorKind::SafetyRefusal));
        }
        let result =
            tool_result(&value).ok_or(ProviderError::new(ProviderErrorKind::ResponseEmpty))?;
        let result = serde_json::from_value(result.clone())
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        validate_adaptive_result(&result, request.intent())
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        Ok(result)
    }
}

fn tool_result(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value
        .get("content")?
        .as_array()?
        .iter()
        .find_map(|content| {
            (content.get("type")?.as_str() == Some("tool_use")
                && content.get("name")?.as_str() == Some(ADAPTIVE_TRANSLATION_TOOL))
            .then(|| content.get("input"))
            .flatten()
        })
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
    use std::sync::Arc;

    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    use crate::infrastructure::{
        ai::{
            contract_fixtures::{AiTranslationProvider, provider_request},
            registry::AiProviderRegistry,
        },
        credentials::system_store::{
            CredentialMetadata, CredentialPort, CredentialSecret, CredentialStoreError,
        },
    };

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
    async fn native_anthropic_forces_structured_tool_use_and_parses_its_input() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/v1/messages"))
            .and(matchers::header("x-api-key", "evolish-secret-canary"))
            .and(matchers::header("anthropic-version", super::ANTHROPIC_VERSION))
            .and(matchers::body_json(serde_json::json!({
                "model": "claude-test", "max_tokens": 2048,
                "system": "source language: en; target language: zh-CN; intent: Sentence; schema version: 1; return only a complete validated result.\n\nTranslate accurately and clearly.",
                "messages": [{"role": "user", "content": "hello"}],
                "tools": [{"name": "adaptive_translation", "description": "Return the complete validated adaptive translation result.", "input_schema": {"type": "object"}}],
                "tool_choice": {"type": "tool", "name": "adaptive_translation"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{"type": "tool_use", "name": "adaptive_translation", "input": {"kind": "sentence", "schemaVersion": 1, "primaryTranslation": "你好"}}],
                "stop_reason": "tool_use"
            })))
            .mount(&server).await;
        let adapter = super::AnthropicAdapter::new(
            &format!("{}/v1/", server.uri()),
            "claude-test",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
            &AiProviderRegistry::new(),
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

    #[test]
    fn native_anthropic_reads_only_the_named_tool_input() {
        let response = serde_json::json!({"content": [
            {"type": "text", "text": "ignore"},
            {"type": "tool_use", "name": "other", "input": {"ignore": true}},
            {"type": "tool_use", "name": "adaptive_translation", "input": {"complete": true}}
        ]});
        assert_eq!(
            super::tool_result(&response),
            Some(&serde_json::json!({"complete": true}))
        );
    }

    #[tokio::test]
    async fn cancelled_native_anthropic_request_returns_without_starting_network_work() {
        let adapter = super::AnthropicAdapter::new(
            "http://127.0.0.1:9",
            "claude-test",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
            &AiProviderRegistry::new(),
        )
        .expect("adapter");
        let cancellation = tokio_util::sync::CancellationToken::new();
        cancellation.cancel();
        let error = adapter
            .translate(provider_request(), cancellation)
            .await
            .expect_err("cancelled request");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled
        );
    }

    #[tokio::test]
    async fn native_anthropic_rejects_invalid_structured_output() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{"type": "tool_use", "name": "adaptive_translation", "input": {"kind": "sentence"}}],
                "stop_reason": "tool_use"
            })))
            .mount(&server).await;
        let adapter = super::AnthropicAdapter::new(
            &format!("{}/v1/", server.uri()),
            "claude-test",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
            &AiProviderRegistry::new(),
        )
        .expect("adapter");
        let error = adapter
            .translate(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect_err("schema invalid");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::ResponseSchemaInvalid
        );
    }

    #[tokio::test]
    async fn native_anthropic_maps_a_native_refusal_to_a_safety_error() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{"type": "text", "text": "I cannot help with that."}],
                "stop_reason": "refusal"
            })))
            .mount(&server)
            .await;
        let adapter = super::AnthropicAdapter::new(
            &format!("{}/v1/", server.uri()),
            "claude-test",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
            &AiProviderRegistry::new(),
        )
        .expect("adapter");
        let error = adapter
            .translate(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect_err("safety refusal");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::SafetyRefusal
        );
    }

    #[test]
    fn native_anthropic_classifies_provider_http_failures_without_response_details() {
        assert_eq!(
            super::map_status(reqwest::StatusCode::UNAUTHORIZED),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::AuthenticationFailed
        );
        assert_eq!(
            super::map_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::RateLimited
        );
        assert_eq!(
            super::map_status(reqwest::StatusCode::PAYMENT_REQUIRED),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::QuotaExceeded
        );
    }
}
