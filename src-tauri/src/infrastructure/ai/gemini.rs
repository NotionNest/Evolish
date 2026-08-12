use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderName};
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

/// Native Gemini `generateContent` adapter using response-schema structured output.
pub struct GeminiAdapter {
    endpoint: Url,
    model: String,
    credentials: Arc<dyn CredentialPort>,
    credential_metadata: CredentialMetadata,
    client: SecureHttpClient,
}

impl GeminiAdapter {
    /// Creates a native adapter for a Gemini `v1beta` API endpoint.
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

    /// Executes a real structured-output capability check.
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
impl AiTranslationProvider for GeminiAdapter {
    fn descriptor(&self) -> &ProviderDescriptor {
        AiProviderRegistry::new()
            .descriptor(crate::domain::provider::ProviderProtocol::Gemini)
            .expect("Gemini descriptor")
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
            .join(&format!("models/{}:generateContent", self.model))
            .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?;
        let payload = serde_json::json!({
            "systemInstruction": {"parts": [{"text": format!("{}\n\n{}", request.prompt().system_contract(), request.prompt().mode_instruction())}]},
            "contents": [{"role": "user", "parts": [{"text": request.prompt().user_content()}]}],
            "generationConfig": {"responseMimeType": "application/json", "responseSchema": {"type": "OBJECT"}}
        });
        let send = self
            .client
            .post(url)
            .header(HeaderName::from_static("x-goog-api-key"), secret.expose())
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
        if is_safety_refusal(&value) {
            return Err(ProviderError::new(ProviderErrorKind::SafetyRefusal));
        }
        let text = value
            .pointer("/candidates/0/content/parts/0/text")
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

fn is_safety_refusal(value: &serde_json::Value) -> bool {
    value
        .pointer("/promptFeedback/blockReason")
        .is_some_and(|reason| !reason.is_null())
        || value
            .pointer("/candidates/0/finishReason")
            .and_then(serde_json::Value::as_str)
            == Some("SAFETY")
}

fn map_status(status: reqwest::StatusCode) -> ProviderErrorKind {
    match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            ProviderErrorKind::AuthenticationFailed
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimited,
        reqwest::StatusCode::PAYMENT_REQUIRED => ProviderErrorKind::QuotaExceeded,
        reqwest::StatusCode::NOT_FOUND => ProviderErrorKind::ModelUnavailable,
        _ => ProviderErrorKind::Network,
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::{
        ai::{
            contract_fixtures::{AiTranslationProvider, provider_request},
            registry::AiProviderRegistry,
        },
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
    fn adapter(endpoint: &str) -> super::GeminiAdapter {
        super::GeminiAdapter::new(
            endpoint,
            "gemini-test",
            Arc::new(StaticCredentialStore),
            CredentialMetadata::new("test-key"),
            &AiProviderRegistry::new(),
        )
        .expect("adapter")
    }

    #[tokio::test]
    async fn native_gemini_uses_response_schema_and_parses_complete_candidate() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST")).and(matchers::path("/v1beta/models/gemini-test:generateContent")).and(matchers::header("x-goog-api-key", "evolish-secret-canary"))
            .and(matchers::body_json(serde_json::json!({"systemInstruction":{"parts":[{"text":"source language: en; target language: zh-CN; intent: Sentence; schema version: 1; return only a complete validated result.\n\nTranslate accurately and clearly."}]},"contents":[{"role":"user","parts":[{"text":"hello"}]}],"generationConfig":{"responseMimeType":"application/json","responseSchema":{"type":"OBJECT"}}})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"candidates":[{"content":{"parts":[{"text":"{\"kind\":\"sentence\",\"schemaVersion\":1,\"primaryTranslation\":\"你好\"}"}]}}]}))).mount(&server).await;
        adapter(&format!("{}/v1beta/", server.uri()))
            .test_connection(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("structured capability test");
    }
    #[tokio::test]
    async fn cancelled_native_gemini_request_returns_without_starting_network_work() {
        let cancellation = tokio_util::sync::CancellationToken::new();
        cancellation.cancel();
        let error = adapter("http://127.0.0.1:9")
            .translate(provider_request(), cancellation)
            .await
            .expect_err("cancelled request");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled
        );
    }
    #[tokio::test]
    async fn native_gemini_maps_native_safety_refusal() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"candidates":[{"finishReason":"SAFETY"}]})),
            )
            .mount(&server)
            .await;
        let error = adapter(&format!("{}/v1beta/", server.uri()))
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
    #[tokio::test]
    async fn native_gemini_rejects_malformed_structured_content() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"candidates":[{"content":{"parts":[{"text":"not-json"}]}}]}),
            ))
            .mount(&server)
            .await;
        let error = adapter(&format!("{}/v1beta/", server.uri()))
            .translate(
                provider_request(),
                tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect_err("malformed result");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::ResponseSchemaInvalid
        );
    }
}
