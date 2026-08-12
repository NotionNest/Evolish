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

/// Native `OpenAI` Responses API adapter using strict structured output.
pub struct OpenAiAdapter {
    endpoint: Url,
    model: String,
    credentials: Arc<dyn CredentialPort>,
    credential_metadata: CredentialMetadata,
    client: SecureHttpClient,
}

impl OpenAiAdapter {
    /// Creates a native adapter for an `OpenAI` Responses API endpoint.
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
impl AiTranslationProvider for OpenAiAdapter {
    fn descriptor(&self) -> &ProviderDescriptor {
        AiProviderRegistry::new()
            .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
            .expect("OpenAI descriptor")
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
            .join("responses")
            .map_err(|_| ProviderError::new(ProviderErrorKind::Network))?;
        let payload = serde_json::json!({
            "model": self.model,
            "input": [
                {"role": "system", "content": request.prompt().system_contract()},
                {"role": "developer", "content": request.prompt().mode_instruction()},
                {"role": "user", "content": request.prompt().user_content()}
            ],
            "text": {"format": {"type": "json_schema", "name": "adaptive_translation", "strict": true, "schema": {"type": "object"}}}
        });
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
        let text = response_output_text(&value)
            .ok_or(ProviderError::new(ProviderErrorKind::ResponseEmpty))?;
        let result = serde_json::from_str(text)
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        validate_adaptive_result(&result, request.intent())
            .map_err(|_| ProviderError::new(ProviderErrorKind::ResponseSchemaInvalid))?;
        Ok(result)
    }
}

fn response_output_text(value: &serde_json::Value) -> Option<&str> {
    value.get("output")?.as_array()?.iter().find_map(|item| {
        (item.get("type")?.as_str() == Some("message")
            && item.get("status")?.as_str() == Some("completed"))
        .then(|| item.get("content")?.as_array())
        .flatten()
        .and_then(|content| {
            content.iter().find_map(|part| {
                (part.get("type")?.as_str() == Some("output_text"))
                    .then(|| part.get("text")?.as_str())
                    .flatten()
                    .filter(|text| !text.trim().is_empty())
            })
        })
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
    use std::{sync::Arc, time::Duration};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::oneshot,
    };
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
    async fn native_openai_uses_responses_structured_output_and_parses_only_complete_result() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/v1/responses"))
            .and(matchers::header("authorization", "Bearer evolish-secret-canary"))
            .and(matchers::body_json(serde_json::json!({
                "model": "gpt-test",
                "input": [{"role": "system", "content": "source language: en; target language: zh-CN; intent: Sentence; schema version: 1; return only a complete validated result."}, {"role": "developer", "content": "Translate accurately and clearly."}, {"role": "user", "content": "hello"}],
                "text": {"format": {"type": "json_schema", "name": "adaptive_translation", "strict": true, "schema": {"type": "object"}}}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"output": [{"type": "message", "status": "completed", "content": [{"type": "output_text", "text": "{\"kind\":\"sentence\",\"schemaVersion\":1,\"primaryTranslation\":\"你好\"}"}]}]})))
            .mount(&server).await;

        let adapter = super::OpenAiAdapter::new(
            &format!("{}/v1/", server.uri()),
            "gpt-test",
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
    fn native_openai_skips_reasoning_items_until_a_complete_message_is_available() {
        let response = serde_json::json!({"output": [
            {"type": "reasoning", "content": []},
            {"type": "message", "status": "completed", "content": [{"type": "output_text", "text": "complete"}]}
        ]});
        assert_eq!(super::response_output_text(&response), Some("complete"));
    }

    #[test]
    fn native_openai_classifies_provider_http_failures_without_response_details() {
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

    #[tokio::test]
    async fn cancelled_native_request_returns_without_starting_network_work() {
        let adapter = super::OpenAiAdapter::new(
            "http://127.0.0.1:9",
            "gpt-test",
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
    async fn native_openai_rejects_schema_invalid_content_after_receiving_the_complete_body() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "output": [{"type": "message", "status": "completed", "content": [{"type": "output_text", "text": "{not-json}"}]}]
            })))
            .mount(&server)
            .await;
        let adapter = super::OpenAiAdapter::new(
            &format!("{}/v1/", server.uri()),
            "gpt-test",
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
            .expect_err("schema-invalid result");
        assert_eq!(
            error.kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::ResponseSchemaInvalid
        );
    }

    #[tokio::test]
    async fn native_openai_does_not_publish_a_chunked_result_before_the_complete_body_arrives() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server bind");
        let address = listener.local_addr().expect("test server address");
        let (first_chunk_sent, first_chunk_receiver) = oneshot::channel();
        let (release_response, release_receiver) = oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await.expect("read request");
            let body = "{\"output\":[{\"type\":\"message\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"{\\\"kind\\\":\\\"sentence\\\",\\\"schemaVersion\\\":1,\\\"primaryTranslation\\\":\\\"你好\\\"}\"}]}]}".as_bytes();
            let split = body.len() / 2;
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n").await.expect("headers");
            socket
                .write_all(format!("{split:X}\r\n").as_bytes())
                .await
                .expect("first length");
            socket.write_all(&body[..split]).await.expect("first body");
            socket.write_all(b"\r\n").await.expect("first terminator");
            first_chunk_sent.send(()).expect("first chunk notification");
            release_receiver.await.expect("release response");
            socket
                .write_all(format!("{:X}\r\n", body.len() - split).as_bytes())
                .await
                .expect("second length");
            socket.write_all(&body[split..]).await.expect("second body");
            socket
                .write_all(b"\r\n0\r\n\r\n")
                .await
                .expect("chunk terminator");
        });
        let adapter = Arc::new(
            super::OpenAiAdapter::new(
                &format!("http://{address}/v1/"),
                "gpt-test",
                Arc::new(StaticCredentialStore),
                CredentialMetadata::new("test-key"),
                &AiProviderRegistry::new(),
            )
            .expect("adapter"),
        );
        let client = Arc::clone(&adapter);
        let mut translation = tokio::spawn(async move {
            client
                .translate(
                    provider_request(),
                    tokio_util::sync::CancellationToken::new(),
                )
                .await
        });

        first_chunk_receiver.await.expect("first chunk received");
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut translation)
                .await
                .is_err()
        );
        release_response.send(()).expect("release response");
        let result = translation
            .await
            .expect("translation task")
            .expect("complete result");
        assert_eq!(
            result.intent(),
            crate::domain::query_intent::QueryIntent::Sentence
        );
    }
}
