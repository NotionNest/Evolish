use std::time::Duration;

use serde_json::Value;

use crate::{
    application::translation::{
        classifier::IntentClassifier,
        language_direction::{
            DirectionResolutionError, LanguageDirectionPreferences, LanguageDirectionResolver,
            TargetLanguageSelection,
        },
        ports::LanguageDetectorPort,
        prompt_compiler::PromptCompiler,
    },
    domain::{
        language::LanguageSelection,
        query_intent::IntentSelection,
        translation::{AdaptiveTranslationResult, TranslationSessionId},
        translation_mode::TranslationModeSnapshot,
    },
    infrastructure::ai::contract_fixtures::{
        AiTranslationProvider, ProviderError, ProviderRequestError, ProviderTranslationRequest,
    },
};

/// User-editable input captured at submission time before any provider request begins.
#[derive(Clone, Debug)]
pub struct TranslationDraft {
    pub text: String,
    pub source: LanguageSelection,
    pub target: TargetLanguageSelection,
    pub intent: IntentSelection,
    pub mode: TranslationModeSnapshot,
    pub timeout: Duration,
    pub parameters: Value,
}

/// Immutable, non-secret request snapshot that can be safely handed to a provider adapter.
#[derive(Clone)]
pub struct PreparedTranslation {
    text: String,
    request: ProviderTranslationRequest,
    configuration: TranslationConfigurationSnapshot,
}

impl PreparedTranslation {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn request(&self) -> &ProviderTranslationRequest {
        &self.request
    }

    #[must_use]
    pub const fn configuration(&self) -> &TranslationConfigurationSnapshot {
        &self.configuration
    }
}

/// Non-secret immutable input/configuration data captured for one attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationConfigurationSnapshot {
    text: String,
    source_language: String,
    target_language: String,
    intent: crate::domain::query_intent::QueryIntent,
    mode_id: String,
    mode_version: u32,
    timeout: Duration,
    parameters: Value,
}

impl TranslationConfigurationSnapshot {
    #[must_use]
    pub fn diff(&self, current: &Self) -> ConfigurationDiff {
        let mut changes = 0;
        if self.text != current.text {
            changes |= TEXT_CHANGED;
        }
        if self.source_language != current.source_language
            || self.target_language != current.target_language
        {
            changes |= DIRECTION_CHANGED;
        }
        if self.intent != current.intent {
            changes |= INTENT_CHANGED;
        }
        if self.mode_id != current.mode_id || self.mode_version != current.mode_version {
            changes |= MODE_CHANGED;
        }
        if self.timeout != current.timeout {
            changes |= TIMEOUT_CHANGED;
        }
        if self.parameters != current.parameters {
            changes |= PARAMETERS_CHANGED;
        }
        ConfigurationDiff { changes }
    }
}

/// Stable, typed comparison of two frozen attempt configurations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigurationDiff {
    changes: u8,
}

impl ConfigurationDiff {
    #[must_use]
    pub const fn text_changed(self) -> bool {
        self.changes & TEXT_CHANGED != 0
    }
    #[must_use]
    pub const fn direction_changed(self) -> bool {
        self.changes & DIRECTION_CHANGED != 0
    }
    #[must_use]
    pub const fn intent_changed(self) -> bool {
        self.changes & INTENT_CHANGED != 0
    }
    #[must_use]
    pub const fn mode_changed(self) -> bool {
        self.changes & MODE_CHANGED != 0
    }
    #[must_use]
    pub const fn timeout_changed(self) -> bool {
        self.changes & TIMEOUT_CHANGED != 0
    }
    #[must_use]
    pub const fn parameters_changed(self) -> bool {
        self.changes & PARAMETERS_CHANGED != 0
    }
}

const TEXT_CHANGED: u8 = 1;
const DIRECTION_CHANGED: u8 = 1 << 1;
const INTENT_CHANGED: u8 = 1 << 2;
const MODE_CHANGED: u8 = 1 << 3;
const TIMEOUT_CHANGED: u8 = 1 << 4;
const PARAMETERS_CHANGED: u8 = 1 << 5;

/// Input validation and local preflight failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrepareError {
    EmptyText,
    ControlOnlyText,
    Direction(DirectionResolutionError),
    Request(ProviderRequestError),
}

/// Failure from the application submission boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitError {
    Prepare(PrepareError),
    Provider(ProviderError),
}

/// Creates complete provider-ready snapshots from local data and user-selected settings.
pub struct TranslationService<D> {
    detector: D,
    preferences: LanguageDirectionPreferences,
    classifier: IntentClassifier,
}

impl<D> TranslationService<D>
where
    D: LanguageDetectorPort,
{
    #[must_use]
    pub fn new(detector: D, preferences: LanguageDirectionPreferences) -> Self {
        Self {
            detector,
            preferences,
            classifier: IntentClassifier,
        }
    }

    /// Validates and freezes a normalized, complete non-secret provider request.
    ///
    /// # Errors
    ///
    /// Returns a local input, direction, or safe provider-request validation error before network I/O.
    pub fn prepare(&self, draft: TranslationDraft) -> Result<PreparedTranslation, PrepareError> {
        let text = normalize_text(&draft.text);
        if text.is_empty() {
            return Err(PrepareError::EmptyText);
        }
        if text.chars().all(char::is_control) {
            return Err(PrepareError::ControlOnlyText);
        }
        let detection = self.detector.detect(&text);
        let direction = LanguageDirectionResolver::resolve(
            self.preferences,
            draft.source,
            draft.target,
            &detection,
        )
        .map_err(PrepareError::Direction)?;
        let intent = self.classifier.classify(&text, draft.intent).intent();
        let prompt = PromptCompiler::compile(&draft.mode, direction, intent, 1, &text);
        let configuration = TranslationConfigurationSnapshot {
            text: text.clone(),
            source_language: direction.source().bcp47().to_owned(),
            target_language: direction.target().bcp47().to_owned(),
            intent,
            mode_id: draft.mode.id().to_string(),
            mode_version: draft.mode.version(),
            timeout: draft.timeout,
            parameters: draft.parameters.clone(),
        };
        let request = ProviderTranslationRequest::new(
            direction,
            intent,
            prompt,
            1,
            draft.timeout,
            draft.parameters,
        )
        .map_err(PrepareError::Request)?;
        Ok(PreparedTranslation {
            text,
            request,
            configuration,
        })
    }

    /// Prepares a snapshot then starts exactly the primary provider through the session supervisor.
    ///
    /// # Errors
    ///
    /// Returns preflight errors before any provider work, otherwise returns the supervisor's provider result.
    pub async fn submit_primary(
        &self,
        supervisor: &crate::application::translation::supervisor::QuerySupervisor,
        window_label: &str,
        primary: std::sync::Arc<dyn AiTranslationProvider>,
        draft: TranslationDraft,
    ) -> Result<(TranslationSessionId, AdaptiveTranslationResult), SubmitError> {
        let prepared = self.prepare(draft).map_err(SubmitError::Prepare)?;
        let configuration = prepared.configuration.clone();
        let result = supervisor
            .submit_primary(window_label, primary, Vec::new(), prepared.request)
            .await
            .map_err(SubmitError::Provider)?;
        supervisor.attach_configuration(result.0, "__primary", configuration);
        Ok(result)
    }

    /// Repeats the primary provider with the current explicit draft instead of replaying an older request.
    ///
    /// # Errors
    ///
    /// Returns preflight errors before any provider work, otherwise returns the supervisor's retry result.
    pub async fn retry_primary(
        &self,
        supervisor: &crate::application::translation::supervisor::QuerySupervisor,
        session_id: TranslationSessionId,
        primary: std::sync::Arc<dyn AiTranslationProvider>,
        draft: TranslationDraft,
    ) -> Result<AdaptiveTranslationResult, SubmitError> {
        let prepared = self.prepare(draft).map_err(SubmitError::Prepare)?;
        let configuration = prepared.configuration.clone();
        let result = supervisor
            .retry_provider(session_id, "__primary", primary, prepared.request)
            .await
            .map_err(SubmitError::Provider)?;
        supervisor.attach_configuration(session_id, "__primary", configuration);
        Ok(result)
    }
}

fn normalize_text(input: &str) -> String {
    input
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        application::translation::{
            language_direction::{LanguageDirectionPreferences, TargetLanguageSelection},
            ports::{DetectionReason, LanguageDetection, LanguageDetectorPort},
        },
        domain::{
            language::{Language, LanguageSelection},
            query_intent::IntentSelection,
            translation_mode::BuiltInTranslationMode,
        },
    };

    struct EnglishDetector;
    impl LanguageDetectorPort for EnglishDetector {
        fn detect(&self, _: &str) -> LanguageDetection {
            LanguageDetection::certain(Language::English, 99, DetectionReason::ScriptRule)
        }
    }

    #[test]
    fn prepare_freezes_normalized_text_direction_intent_and_compiled_prompt() {
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );

        let prepared = service
            .prepare(super::TranslationDraft {
                text: "  hello\r\nworld  ".to_owned(),
                source: LanguageSelection::Automatic,
                target: TargetLanguageSelection::Automatic,
                intent: IntentSelection::Automatic,
                mode: BuiltInTranslationMode::Standard.snapshot(),
                timeout: Duration::from_secs(30),
                parameters: serde_json::json!({"temperature": 0.2}),
            })
            .expect("prepared request");

        assert_eq!(prepared.text(), "hello\nworld");
        assert_eq!(prepared.request().direction().source(), Language::English);
        assert_eq!(
            prepared.request().direction().target(),
            Language::ChineseSimplified
        );
        assert_eq!(prepared.request().prompt().user_content(), "hello\nworld");
    }

    #[test]
    fn prepare_rejects_blank_or_control_only_text_before_provider_selection() {
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );
        let draft = |text: &str| super::TranslationDraft {
            text: text.to_owned(),
            source: LanguageSelection::Automatic,
            target: TargetLanguageSelection::Automatic,
            intent: IntentSelection::Automatic,
            mode: BuiltInTranslationMode::Standard.snapshot(),
            timeout: Duration::from_secs(30),
            parameters: serde_json::json!({}),
        };

        assert!(matches!(
            service.prepare(draft(" \n\t ")),
            Err(super::PrepareError::EmptyText)
        ));
        assert!(matches!(
            service.prepare(draft("\u{0007}")),
            Err(super::PrepareError::ControlOnlyText)
        ));
    }

    #[test]
    fn prepared_configuration_diff_is_typed_and_uses_frozen_snapshots() {
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );
        let draft = |text: &str, timeout| super::TranslationDraft {
            text: text.to_owned(),
            source: LanguageSelection::Automatic,
            target: TargetLanguageSelection::Automatic,
            intent: IntentSelection::Automatic,
            mode: BuiltInTranslationMode::Standard.snapshot(),
            timeout,
            parameters: serde_json::json!({"temperature": 0.2}),
        };
        let first = service
            .prepare(draft("hello", Duration::from_secs(30)))
            .expect("first");
        let retried = service
            .prepare(draft("changed", Duration::from_mins(1)))
            .expect("retry");

        let diff = first.configuration().diff(retried.configuration());

        assert!(diff.text_changed());
        assert!(diff.timeout_changed());
        assert!(!diff.direction_changed());
        assert!(!diff.mode_changed());
        assert!(!diff.parameters_changed());
    }

    #[tokio::test]
    async fn submit_preflight_failure_never_calls_the_primary_provider() {
        use crate::infrastructure::ai::{
            contract_fixtures::{AiTranslationProvider, ProviderError, ProviderTranslationRequest},
            registry::{AiProviderRegistry, ProviderDescriptor},
        };
        use async_trait::async_trait;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use tokio_util::sync::CancellationToken;

        struct CountingProvider(AtomicUsize);
        #[async_trait]
        impl AiTranslationProvider for CountingProvider {
            fn descriptor(&self) -> &ProviderDescriptor {
                AiProviderRegistry::new()
                    .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
                    .expect("descriptor")
            }
            async fn translate(
                &self,
                _: ProviderTranslationRequest,
                _: CancellationToken,
            ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError>
            {
                self.0.fetch_add(1, Ordering::SeqCst);
                Err(ProviderError::new(
                    crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Network,
                ))
            }
        }
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );
        let provider = Arc::new(CountingProvider(AtomicUsize::new(0)));

        let result = service
            .submit_primary(
                &crate::application::translation::supervisor::QuerySupervisor::new(),
                "main",
                provider.clone(),
                super::TranslationDraft {
                    text: "   ".to_owned(),
                    source: LanguageSelection::Automatic,
                    target: TargetLanguageSelection::Automatic,
                    intent: IntentSelection::Automatic,
                    mode: BuiltInTranslationMode::Standard.snapshot(),
                    timeout: Duration::from_secs(30),
                    parameters: serde_json::json!({}),
                },
            )
            .await;

        assert!(matches!(
            result,
            Err(super::SubmitError::Prepare(super::PrepareError::EmptyText))
        ));
        assert_eq!(provider.0.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn successful_service_submission_attaches_its_frozen_configuration_to_the_attempt() {
        use crate::infrastructure::ai::{
            contract_fixtures::{AiTranslationProvider, ProviderError, ProviderTranslationRequest},
            registry::{AiProviderRegistry, ProviderDescriptor},
        };
        use async_trait::async_trait;
        use std::sync::Arc;
        use tokio_util::sync::CancellationToken;
        struct SuccessProvider;
        #[async_trait]
        impl AiTranslationProvider for SuccessProvider {
            fn descriptor(&self) -> &ProviderDescriptor {
                AiProviderRegistry::new()
                    .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
                    .expect("descriptor")
            }
            async fn translate(
                &self,
                _: ProviderTranslationRequest,
                _: CancellationToken,
            ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError>
            {
                Ok(
                    crate::domain::translation::AdaptiveTranslationResult::Sentence(
                        crate::domain::translation::SentenceResultV1 {
                            schema_version: 1,
                            primary_translation: "你好".to_owned(),
                            wording_notes: Vec::new(),
                            grammar_notes: Vec::new(),
                        },
                    ),
                )
            }
        }
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );
        let supervisor = crate::application::translation::supervisor::QuerySupervisor::new();
        let (session_id, _) = service
            .submit_primary(
                &supervisor,
                "main",
                Arc::new(SuccessProvider),
                super::TranslationDraft {
                    text: "hello".to_owned(),
                    source: LanguageSelection::Automatic,
                    target: TargetLanguageSelection::Automatic,
                    intent: IntentSelection::Automatic,
                    mode: BuiltInTranslationMode::Standard.snapshot(),
                    timeout: Duration::from_secs(30),
                    parameters: serde_json::json!({}),
                },
            )
            .await
            .expect("success");

        assert!(
            !supervisor
                .latest_attempt(session_id, "__primary")
                .expect("attempt")
                .configuration()
                .expect("configuration")
                .diff(
                    service
                        .prepare(super::TranslationDraft {
                            text: "hello".to_owned(),
                            source: LanguageSelection::Automatic,
                            target: TargetLanguageSelection::Automatic,
                            intent: IntentSelection::Automatic,
                            mode: BuiltInTranslationMode::Standard.snapshot(),
                            timeout: Duration::from_secs(30),
                            parameters: serde_json::json!({})
                        })
                        .expect("prepared")
                        .configuration()
                )
                .text_changed()
        );
    }

    #[tokio::test]
    async fn retry_uses_the_current_explicit_draft_and_exposes_its_diff_from_the_prior_snapshot() {
        use crate::infrastructure::ai::{
            contract_fixtures::{AiTranslationProvider, ProviderError, ProviderTranslationRequest},
            registry::{AiProviderRegistry, ProviderDescriptor},
        };
        use async_trait::async_trait;
        use std::sync::Arc;
        use tokio_util::sync::CancellationToken;
        struct SuccessProvider;
        #[async_trait]
        impl AiTranslationProvider for SuccessProvider {
            fn descriptor(&self) -> &ProviderDescriptor {
                AiProviderRegistry::new()
                    .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
                    .expect("descriptor")
            }
            async fn translate(
                &self,
                _: ProviderTranslationRequest,
                _: CancellationToken,
            ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError>
            {
                Ok(
                    crate::domain::translation::AdaptiveTranslationResult::Sentence(
                        crate::domain::translation::SentenceResultV1 {
                            schema_version: 1,
                            primary_translation: "ok".to_owned(),
                            wording_notes: Vec::new(),
                            grammar_notes: Vec::new(),
                        },
                    ),
                )
            }
        }
        let service = super::TranslationService::new(
            EnglishDetector,
            LanguageDirectionPreferences::new(Language::ChineseSimplified, Language::English)
                .expect("preferences"),
        );
        let supervisor = crate::application::translation::supervisor::QuerySupervisor::new();
        let draft = |text: &str, timeout| super::TranslationDraft {
            text: text.to_owned(),
            source: LanguageSelection::Automatic,
            target: TargetLanguageSelection::Automatic,
            intent: IntentSelection::Automatic,
            mode: BuiltInTranslationMode::Standard.snapshot(),
            timeout,
            parameters: serde_json::json!({"temperature": 0.2}),
        };
        let (session_id, _) = service
            .submit_primary(
                &supervisor,
                "main",
                Arc::new(SuccessProvider),
                draft("original", Duration::from_secs(30)),
            )
            .await
            .expect("first success");
        let previous = supervisor
            .latest_attempt(session_id, "__primary")
            .expect("previous");

        service
            .retry_primary(
                &supervisor,
                session_id,
                Arc::new(SuccessProvider),
                draft("changed", Duration::from_mins(1)),
            )
            .await
            .expect("retry success");
        let current = supervisor
            .latest_attempt(session_id, "__primary")
            .expect("current");
        let diff = supervisor
            .configuration_diff(session_id, "__primary", previous.id(), current.id())
            .expect("typed diff");

        assert_ne!(previous.id(), current.id());
        assert!(diff.text_changed());
        assert!(diff.timeout_changed());
        assert!(!diff.direction_changed());
    }
}
