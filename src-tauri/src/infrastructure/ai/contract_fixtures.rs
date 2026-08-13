use std::{error::Error, fmt, time::Duration};

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    application::translation::prompt_compiler::CompiledPrompt,
    domain::{
        language::LanguageDirection, query_intent::QueryIntent,
        translation::AdaptiveTranslationResult,
    },
    infrastructure::ai::registry::ProviderDescriptor,
};

/// Complete, non-secret data needed by a provider adapter for one translation attempt.
#[derive(Clone)]
pub struct ProviderTranslationRequest {
    direction: LanguageDirection,
    intent: QueryIntent,
    prompt: CompiledPrompt,
    schema_version: u16,
    timeout: Duration,
    parameters: Value,
}

impl ProviderTranslationRequest {
    /// Creates a request that contains no credential material.
    ///
    /// # Errors
    ///
    /// Returns an error when the schema version, timeout, or non-secret parameters are invalid.
    pub fn new(
        direction: LanguageDirection,
        intent: QueryIntent,
        prompt: CompiledPrompt,
        schema_version: u16,
        timeout: Duration,
        parameters: Value,
    ) -> Result<Self, ProviderRequestError> {
        if schema_version == 0 {
            return Err(ProviderRequestError::InvalidSchemaVersion);
        }
        if timeout.is_zero() {
            return Err(ProviderRequestError::InvalidTimeout);
        }
        if !parameters.is_object() {
            return Err(ProviderRequestError::ParametersMustBeObject);
        }
        if contains_secret_key(&parameters) {
            return Err(ProviderRequestError::SecretInParameters);
        }
        Ok(Self {
            direction,
            intent,
            prompt,
            schema_version,
            timeout,
            parameters,
        })
    }

    #[must_use]
    pub const fn direction(&self) -> LanguageDirection {
        self.direction
    }

    #[must_use]
    pub const fn intent(&self) -> QueryIntent {
        self.intent
    }

    #[must_use]
    pub fn prompt(&self) -> &CompiledPrompt {
        &self.prompt
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[must_use]
    pub const fn parameters(&self) -> &Value {
        &self.parameters
    }
}

fn contains_secret_key(value: &Value) -> bool {
    match value {
        Value::Object(entries) => entries.iter().any(|(key, value)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "api_key" | "authorization" | "secret" | "token"
            ) || contains_secret_key(value)
        }),
        Value::Array(entries) => entries.iter().any(contains_secret_key),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

impl fmt::Debug for ProviderTranslationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderTranslationRequest")
            .field("source_language", &self.direction.source().bcp47())
            .field("target_language", &self.direction.target().bcp47())
            .field("intent", &self.intent)
            .field("schema_version", &self.schema_version)
            .field("timeout", &self.timeout)
            .field("parameters", &self.parameters)
            .field(
                "user_content_length",
                &self.prompt.user_content().chars().count(),
            )
            .finish()
    }
}

/// Safe validation failures for a provider request before network I/O begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRequestError {
    InvalidSchemaVersion,
    InvalidTimeout,
    ParametersMustBeObject,
    SecretInParameters,
}

/// Classified provider failures that callers can handle without raw response details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderErrorKind {
    AuthenticationFailed,
    RateLimited,
    QuotaExceeded,
    ModelUnavailable,
    SafetyRefusal,
    ResponseEmpty,
    ResponseMalformed,
    ResponseSchemaInvalid,
    ResponseTooLarge,
    TimedOut,
    Cancelled,
    Network,
}

impl ProviderErrorKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AuthenticationFailed => "translation.provider.authentication_failed",
            Self::RateLimited => "translation.provider.rate_limited",
            Self::QuotaExceeded => "translation.provider.quota_exceeded",
            Self::ModelUnavailable => "translation.provider.model_unavailable",
            Self::SafetyRefusal => "translation.provider.safety_refusal",
            Self::ResponseEmpty => "translation.provider.response_empty",
            Self::ResponseMalformed => "translation.provider.response_malformed",
            Self::ResponseSchemaInvalid => "translation.provider.response_schema_invalid",
            Self::ResponseTooLarge => "translation.provider.response_too_large",
            Self::TimedOut => "translation.provider.timed_out",
            Self::Cancelled => "translation.provider.cancelled",
            Self::Network => "translation.provider.network",
        }
    }
}

/// A redacted provider failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderError {
    kind: ProviderErrorKind,
}

impl ProviderError {
    #[must_use]
    pub const fn new(kind: ProviderErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> ProviderErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn code(self) -> &'static str {
        self.kind.code()
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl Error for ProviderError {}

/// A compiled-in provider adapter that returns only fully validated result data.
#[async_trait]
pub trait AiTranslationProvider: Send + Sync {
    /// Returns the adapter's immutable protocol descriptor.
    fn descriptor(&self) -> &ProviderDescriptor;

    /// Translates one request, observing cancellation and returning a classified failure.
    async fn translate(
        &self,
        request: ProviderTranslationRequest,
        cancellation: CancellationToken,
    ) -> Result<AdaptiveTranslationResult, ProviderError>;
}

#[cfg(test)]
use std::collections::VecDeque;

#[cfg(test)]
use crate::{
    application::translation::{
        prompt_compiler::PromptCompiler, result_validator::validate_adaptive_result,
    },
    domain::{
        language::Language, translation::SentenceResultV1, translation_mode::BuiltInTranslationMode,
    },
    infrastructure::ai::registry::AiProviderRegistry,
};

/// Builds a safe request fixture used by every provider adapter contract test.
#[cfg(test)]
#[must_use]
///
/// # Panics
///
/// Panics only when a hard-coded contract fixture becomes invalid.
pub fn provider_request() -> ProviderTranslationRequest {
    let direction = LanguageDirection::new(Language::English, Language::ChineseSimplified)
        .expect("fixture has different languages");
    let prompt = PromptCompiler::compile(
        &BuiltInTranslationMode::Standard.snapshot(),
        direction,
        QueryIntent::Sentence,
        1,
        "hello",
    );
    ProviderTranslationRequest::new(
        direction,
        QueryIntent::Sentence,
        prompt,
        1,
        Duration::from_secs(30),
        serde_json::json!({"temperature": 0.2}),
    )
    .expect("valid fixture request")
}

#[cfg(test)]
pub struct FixtureProvider {
    outcomes: std::sync::Mutex<VecDeque<Result<AdaptiveTranslationResult, ProviderError>>>,
}

#[cfg(test)]
#[must_use]
pub fn fixture_provider() -> FixtureProvider {
    let valid_result = AdaptiveTranslationResult::Sentence(SentenceResultV1 {
        schema_version: 1,
        primary_translation: "你好".to_owned(),
        wording_notes: Vec::new(),
        grammar_notes: Vec::new(),
    });
    let errors = [
        ProviderErrorKind::AuthenticationFailed,
        ProviderErrorKind::RateLimited,
        ProviderErrorKind::QuotaExceeded,
        ProviderErrorKind::ModelUnavailable,
        ProviderErrorKind::SafetyRefusal,
        ProviderErrorKind::ResponseEmpty,
        ProviderErrorKind::ResponseMalformed,
        ProviderErrorKind::ResponseSchemaInvalid,
        ProviderErrorKind::ResponseTooLarge,
        ProviderErrorKind::TimedOut,
    ];
    let mut outcomes = VecDeque::from([Ok(valid_result)]);
    outcomes.extend(errors.into_iter().map(|kind| Err(ProviderError::new(kind))));
    FixtureProvider {
        outcomes: std::sync::Mutex::new(outcomes),
    }
}

#[cfg(test)]
#[async_trait]
impl AiTranslationProvider for FixtureProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        AiProviderRegistry::new()
            .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
            .expect("OpenAI descriptor is compiled in")
    }

    async fn translate(
        &self,
        _request: ProviderTranslationRequest,
        cancellation: CancellationToken,
    ) -> Result<AdaptiveTranslationResult, ProviderError> {
        if cancellation.is_cancelled() {
            return Err(ProviderError::new(ProviderErrorKind::Cancelled));
        }
        self.outcomes
            .lock()
            .expect("fixture outcomes lock")
            .pop_front()
            .expect("fixture outcome")
    }
}

/// Runs the shared conformance checks against a deliberately scripted test double.
#[cfg(test)]
///
/// # Panics
///
/// Panics when the supplied adapter violates a shared result or error contract.
pub async fn assert_provider_contract(provider: impl AiTranslationProvider) {
    let success = provider
        .translate(provider_request(), CancellationToken::new())
        .await
        .expect("fixture valid result");
    validate_adaptive_result(&success, QueryIntent::Sentence).expect("complete valid result");

    for expected in [
        ProviderErrorKind::AuthenticationFailed,
        ProviderErrorKind::RateLimited,
        ProviderErrorKind::QuotaExceeded,
        ProviderErrorKind::ModelUnavailable,
        ProviderErrorKind::SafetyRefusal,
        ProviderErrorKind::ResponseEmpty,
        ProviderErrorKind::ResponseMalformed,
        ProviderErrorKind::ResponseSchemaInvalid,
        ProviderErrorKind::ResponseTooLarge,
        ProviderErrorKind::TimedOut,
    ] {
        let error = provider
            .translate(provider_request(), CancellationToken::new())
            .await
            .expect_err("fixture failure");
        assert_eq!(error.kind(), expected);
        assert_eq!(error.code(), expected.code());
    }

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let error = provider
        .translate(provider_request(), cancellation)
        .await
        .expect_err("cancelled request");
    assert_eq!(error.kind(), ProviderErrorKind::Cancelled);
}
