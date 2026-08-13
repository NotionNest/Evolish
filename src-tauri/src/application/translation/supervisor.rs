use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::{
    domain::translation::{
        AdaptiveTranslationResult, ResultVersion, TranslationAttemptId, TranslationSessionId,
    },
    infrastructure::ai::contract_fixtures::{
        AiTranslationProvider, ProviderError, ProviderTranslationRequest,
    },
};

/// Coordinates one active root request per window and starts only its primary provider.
pub struct QuerySupervisor {
    state: Mutex<SupervisorState>,
}

#[derive(Default)]
struct SupervisorState {
    windows: HashMap<String, TranslationSessionId>,
    sessions: HashMap<TranslationSessionId, SessionState>,
    completed_submissions: HashMap<String, TranslationSessionId>,
    in_flight_submissions: HashMap<String, Arc<Notify>>,
}

struct SessionState {
    cancellation: CancellationToken,
    completed_results: HashMap<String, AdaptiveTranslationResult>,
    attempts: HashMap<String, Vec<AttemptRecord>>,
}

fn completed_submission_from_state(
    state: &SupervisorState,
    submission_id: &str,
) -> Option<(TranslationSessionId, AdaptiveTranslationResult)> {
    let session_id = *state.completed_submissions.get(submission_id)?;
    let result = state
        .sessions
        .get(&session_id)?
        .completed_results
        .get("__primary")?
        .clone();
    Some((session_id, result))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptSnapshot {
    id: TranslationAttemptId,
    result_version: ResultVersion,
    result: Option<AdaptiveTranslationResult>,
    configuration:
        Option<crate::application::translation::service::TranslationConfigurationSnapshot>,
}

impl AttemptSnapshot {
    #[must_use]
    pub const fn id(&self) -> TranslationAttemptId {
        self.id
    }

    #[must_use]
    pub const fn result_version(&self) -> ResultVersion {
        self.result_version
    }

    #[must_use]
    pub const fn result(&self) -> Option<&AdaptiveTranslationResult> {
        self.result.as_ref()
    }

    #[must_use]
    pub const fn configuration(
        &self,
    ) -> Option<&crate::application::translation::service::TranslationConfigurationSnapshot> {
        self.configuration.as_ref()
    }
}

#[derive(Clone)]
struct AttemptRecord {
    snapshot: AttemptSnapshot,
}

impl Default for QuerySupervisor {
    fn default() -> Self {
        Self {
            state: Mutex::new(SupervisorState::default()),
        }
    }
}

impl QuerySupervisor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs only the supplied primary provider and keeps other providers dormant until explicitly requested.
    ///
    /// # Errors
    ///
    /// Returns the primary provider's classified error or cancellation result.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal window-state mutex is poisoned.
    pub async fn submit_primary(
        &self,
        window_label: &str,
        primary: Arc<dyn AiTranslationProvider>,
        _unexpanded_providers: Vec<Arc<dyn AiTranslationProvider>>,
        request: ProviderTranslationRequest,
    ) -> Result<(TranslationSessionId, AdaptiveTranslationResult), ProviderError> {
        let cancellation = CancellationToken::new();
        let session_id = TranslationSessionId::generate();
        {
            let mut state = self.state.lock().expect("supervisor state lock");
            if let Some(previous_id) = state.windows.insert(window_label.to_owned(), session_id)
                && let Some(previous) = state.sessions.get(&previous_id)
            {
                previous.cancellation.cancel();
            }
            state.sessions.insert(
                session_id,
                SessionState {
                    cancellation: cancellation.clone(),
                    completed_results: HashMap::new(),
                    attempts: HashMap::from([(
                        "__primary".to_owned(),
                        vec![AttemptRecord {
                            snapshot: AttemptSnapshot {
                                id: TranslationAttemptId::generate(),
                                result_version: ResultVersion::initial(),
                                result: None,
                                configuration: None,
                            },
                        }],
                    )]),
                },
            );
        }
        let result = primary.translate(request, cancellation.clone()).await;
        if cancellation.is_cancelled() {
            return Err(ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            ));
        }
        let translation = result?;
        let mut state = self.state.lock().expect("supervisor state lock");
        let current_session = state.windows.get(window_label) == Some(&session_id);
        let session = state.sessions.get_mut(&session_id);
        if !current_session
            || session
                .as_ref()
                .is_none_or(|session| session.cancellation.is_cancelled())
        {
            return Err(ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            ));
        }
        let session = session.expect("checked session exists");
        session
            .completed_results
            .insert("__primary".to_owned(), translation.clone());
        let attempt = session
            .attempts
            .get_mut("__primary")
            .expect("primary attempt exists")
            .last_mut()
            .expect("primary latest attempt");
        attempt.snapshot.result = Some(translation.clone());
        Ok((session_id, translation))
    }

    /// Submits a primary request once for a completed submission ID and returns the already-published session on repeat.
    ///
    /// # Errors
    ///
    /// Returns the primary provider's classified failure or cancellation result.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    pub async fn submit_primary_idempotent(
        &self,
        window_label: &str,
        submission_id: &str,
        primary: Arc<dyn AiTranslationProvider>,
        request: ProviderTranslationRequest,
    ) -> Result<(TranslationSessionId, AdaptiveTranslationResult), ProviderError> {
        loop {
            let wait = {
                let mut state = self.state.lock().expect("supervisor state lock");
                if let Some(existing) = completed_submission_from_state(&state, submission_id) {
                    return Ok(existing);
                }
                if let Some(notify) = state.in_flight_submissions.get(submission_id) {
                    Some(Arc::clone(notify).notified_owned())
                } else {
                    state
                        .in_flight_submissions
                        .insert(submission_id.to_owned(), Arc::new(Notify::new()));
                    None
                }
            };
            if let Some(wait) = wait {
                wait.await;
            } else {
                break;
            }
        }
        let result = self
            .submit_primary(window_label, primary, Vec::new(), request)
            .await;
        let mut state = self.state.lock().expect("supervisor state lock");
        let notify = state
            .in_flight_submissions
            .remove(submission_id)
            .expect("submission was registered");
        if let Ok((session_id, _)) = result {
            state
                .completed_submissions
                .insert(submission_id.to_owned(), session_id);
        }
        drop(state);
        notify.notify_waiters();
        result
    }

    /// Returns the latest attempt snapshot for a provider within a session.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    #[must_use]
    pub fn latest_attempt(
        &self,
        session_id: TranslationSessionId,
        provider_key: &str,
    ) -> Option<AttemptSnapshot> {
        self.state
            .lock()
            .expect("supervisor state lock")
            .sessions
            .get(&session_id)?
            .attempts
            .get(provider_key)?
            .last()
            .map(|record| record.snapshot.clone())
    }

    /// Attaches the frozen non-secret configuration used by a completed attempt.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    pub fn attach_configuration(
        &self,
        session_id: TranslationSessionId,
        provider_key: &str,
        configuration: crate::application::translation::service::TranslationConfigurationSnapshot,
    ) {
        if let Some(attempt) = self
            .state
            .lock()
            .expect("supervisor state lock")
            .sessions
            .get_mut(&session_id)
            .and_then(|session| session.attempts.get_mut(provider_key))
            .and_then(|attempts| attempts.last_mut())
        {
            attempt.snapshot.configuration = Some(configuration);
        }
    }

    /// Compares two immutable attempt configurations from the same provider/session lineage.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    #[must_use]
    pub fn configuration_diff(
        &self,
        session_id: TranslationSessionId,
        provider_key: &str,
        previous_attempt_id: TranslationAttemptId,
        current_attempt_id: TranslationAttemptId,
    ) -> Option<crate::application::translation::service::ConfigurationDiff> {
        let state = self.state.lock().expect("supervisor state lock");
        let attempts = state
            .sessions
            .get(&session_id)?
            .attempts
            .get(provider_key)?;
        let previous = attempts
            .iter()
            .find(|attempt| attempt.snapshot.id == previous_attempt_id)?
            .snapshot
            .configuration
            .as_ref()?;
        let current = attempts
            .iter()
            .find(|attempt| attempt.snapshot.id == current_attempt_id)?
            .snapshot
            .configuration
            .as_ref()?;
        Some(previous.diff(current))
    }

    /// Repeats a completed provider attempt using a new identity and later result version.
    ///
    /// # Errors
    ///
    /// Returns cancellation when the session is inactive, or the provider's classified failure. The prior successful attempt remains available.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    pub async fn retry_provider(
        &self,
        session_id: TranslationSessionId,
        provider_key: &str,
        provider: Arc<dyn AiTranslationProvider>,
        request: ProviderTranslationRequest,
    ) -> Result<AdaptiveTranslationResult, ProviderError> {
        let cancellation = {
            let mut state = self.state.lock().expect("supervisor state lock");
            let session = state.sessions.get_mut(&session_id).ok_or_else(|| {
                ProviderError::new(
                    crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
                )
            })?;
            if session.cancellation.is_cancelled() {
                return Err(ProviderError::new(
                    crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
                ));
            }
            let previous = session
                .attempts
                .get(provider_key)
                .and_then(|attempts| attempts.last())
                .ok_or_else(|| {
                    ProviderError::new(
                        crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
                    )
                })?;
            let next_version = previous.snapshot.result_version.next().map_err(|_| {
                ProviderError::new(crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::ResponseMalformed)
            })?;
            session
                .attempts
                .entry(provider_key.to_owned())
                .or_default()
                .push(AttemptRecord {
                    snapshot: AttemptSnapshot {
                        id: TranslationAttemptId::generate(),
                        result_version: next_version,
                        result: None,
                        configuration: None,
                    },
                });
            session.cancellation.clone()
        };
        let translation = provider.translate(request, cancellation.clone()).await?;
        let mut state = self.state.lock().expect("supervisor state lock");
        let session = state.sessions.get_mut(&session_id).ok_or_else(|| {
            ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            )
        })?;
        if cancellation.is_cancelled() || session.cancellation.is_cancelled() {
            return Err(ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            ));
        }
        let latest = session
            .attempts
            .get_mut(provider_key)
            .and_then(|attempts| attempts.last_mut())
            .expect("retry attempt exists");
        latest.snapshot.result = Some(translation.clone());
        session
            .completed_results
            .insert(provider_key.to_owned(), translation.clone());
        Ok(translation)
    }

    /// Requests an on-demand provider once per successful session result and reuses that complete result thereafter.
    ///
    /// # Errors
    ///
    /// Returns a classified provider error when the session is no longer active, is cancelled, or the provider fails.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal state mutex is poisoned.
    pub async fn expand_provider(
        &self,
        session_id: TranslationSessionId,
        provider_key: &str,
        provider: Arc<dyn AiTranslationProvider>,
        request: ProviderTranslationRequest,
    ) -> Result<AdaptiveTranslationResult, ProviderError> {
        let cancellation = {
            let state = self.state.lock().expect("supervisor state lock");
            let session = state.sessions.get(&session_id).ok_or_else(|| {
                ProviderError::new(
                    crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
                )
            })?;
            if session.cancellation.is_cancelled() {
                return Err(ProviderError::new(
                    crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
                ));
            }
            if let Some(result) = session.completed_results.get(provider_key) {
                return Ok(result.clone());
            }
            session.cancellation.clone()
        };
        let translation = provider.translate(request, cancellation.clone()).await?;
        let mut state = self.state.lock().expect("supervisor state lock");
        let session = state.sessions.get_mut(&session_id).ok_or_else(|| {
            ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            )
        })?;
        if cancellation.is_cancelled() || session.cancellation.is_cancelled() {
            return Err(ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled,
            ));
        }
        session
            .completed_results
            .insert(provider_key.to_owned(), translation.clone());
        Ok(translation)
    }

    /// Cancels the active session in a window, if any.
    ///
    /// # Panics
    ///
    /// Panics only if the supervisor's internal window-state mutex is poisoned.
    pub fn cancel_window(&self, window_label: &str) {
        let mut state = self.state.lock().expect("supervisor state lock");
        if let Some(session_id) = state.windows.remove(window_label)
            && let Some(session) = state.sessions.get(&session_id)
        {
            session.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use tokio::sync::Notify;
    use tokio_util::sync::CancellationToken;

    use crate::infrastructure::ai::{
        contract_fixtures::{
            AiTranslationProvider, ProviderError, ProviderTranslationRequest, provider_request,
        },
        registry::{AiProviderRegistry, ProviderDescriptor},
    };

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
        ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(1)).await;
            Err(ProviderError::new(
                crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Network,
            ))
        }
    }

    #[tokio::test]
    async fn submitting_starts_exactly_one_primary_request_and_no_unexpanded_secondary_requests() {
        let primary = Arc::new(CountingProvider(AtomicUsize::new(0)));
        let other_one = Arc::new(CountingProvider(AtomicUsize::new(0)));
        let other_two = Arc::new(CountingProvider(AtomicUsize::new(0)));
        let other_three = Arc::new(CountingProvider(AtomicUsize::new(0)));
        let supervisor = super::QuerySupervisor::new();
        let primary_provider: Arc<dyn AiTranslationProvider> = primary.clone();
        let secondary_providers: Vec<Arc<dyn AiTranslationProvider>> =
            vec![other_one, other_two, other_three];

        let _ = supervisor
            .submit_primary(
                "main",
                primary_provider,
                secondary_providers,
                provider_request(),
            )
            .await;

        assert_eq!(primary.0.load(Ordering::SeqCst), 1);
    }

    struct LateSuccessProvider {
        started: Notify,
        release: Notify,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl AiTranslationProvider for LateSuccessProvider {
        fn descriptor(&self) -> &ProviderDescriptor {
            AiProviderRegistry::new()
                .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
                .expect("descriptor")
        }

        async fn translate(
            &self,
            _: ProviderTranslationRequest,
            _: CancellationToken,
        ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            self.release.notified().await;
            Ok(
                crate::domain::translation::AdaptiveTranslationResult::Sentence(
                    crate::domain::translation::SentenceResultV1 {
                        schema_version: 1,
                        primary_translation: "late".to_owned(),
                        wording_notes: Vec::new(),
                        grammar_notes: Vec::new(),
                    },
                ),
            )
        }
    }

    #[tokio::test]
    async fn a_late_completion_from_a_replaced_window_session_is_cancelled_not_returned() {
        let supervisor = Arc::new(super::QuerySupervisor::new());
        let late = Arc::new(LateSuccessProvider {
            started: Notify::new(),
            release: Notify::new(),
            calls: AtomicUsize::new(0),
        });
        let late_provider: Arc<dyn AiTranslationProvider> = late.clone();
        let first_supervisor = Arc::clone(&supervisor);
        let first = tokio::spawn(async move {
            first_supervisor
                .submit_primary("main", late_provider, Vec::new(), provider_request())
                .await
        });
        late.started.notified().await;

        let replacement: Arc<dyn AiTranslationProvider> =
            Arc::new(CountingProvider(AtomicUsize::new(0)));
        let _ = supervisor
            .submit_primary("main", replacement, Vec::new(), provider_request())
            .await;
        late.release.notify_one();

        let result = first.await.expect("first task");
        assert_eq!(
            result.expect_err("stale result must be cancelled").kind(),
            crate::infrastructure::ai::contract_fixtures::ProviderErrorKind::Cancelled
        );
    }

    struct SuccessCountingProvider(AtomicUsize);

    #[async_trait]
    impl AiTranslationProvider for SuccessCountingProvider {
        fn descriptor(&self) -> &ProviderDescriptor {
            AiProviderRegistry::new()
                .descriptor(crate::domain::provider::ProviderProtocol::OpenAi)
                .expect("descriptor")
        }

        async fn translate(
            &self,
            _: ProviderTranslationRequest,
            _: CancellationToken,
        ) -> Result<crate::domain::translation::AdaptiveTranslationResult, ProviderError> {
            self.0.fetch_add(1, Ordering::SeqCst);
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

    #[tokio::test]
    async fn expanding_a_provider_once_requests_it_once_and_reexpanding_reuses_its_success() {
        let supervisor = super::QuerySupervisor::new();
        let primary: Arc<dyn AiTranslationProvider> =
            Arc::new(SuccessCountingProvider(AtomicUsize::new(0)));
        let secondary = Arc::new(SuccessCountingProvider(AtomicUsize::new(0)));
        let (session_id, _) = supervisor
            .submit_primary("main", primary, Vec::new(), provider_request())
            .await
            .expect("primary success");

        supervisor
            .expand_provider(
                session_id,
                "secondary",
                secondary.clone(),
                provider_request(),
            )
            .await
            .expect("first expansion");
        supervisor
            .expand_provider(
                session_id,
                "secondary",
                secondary.clone(),
                provider_request(),
            )
            .await
            .expect("cached expansion");

        assert_eq!(secondary.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_creates_a_new_attempt_version_while_retaining_the_prior_success() {
        let supervisor = super::QuerySupervisor::new();
        let provider = Arc::new(SuccessCountingProvider(AtomicUsize::new(0)));
        let primary: Arc<dyn AiTranslationProvider> = provider.clone();
        let (session_id, first_result) = supervisor
            .submit_primary("main", primary, Vec::new(), provider_request())
            .await
            .expect("first success");
        let first = supervisor
            .latest_attempt(session_id, "__primary")
            .expect("first attempt");

        let retried = supervisor
            .retry_provider(
                session_id,
                "__primary",
                provider.clone(),
                provider_request(),
            )
            .await
            .expect("retry success");
        let latest = supervisor
            .latest_attempt(session_id, "__primary")
            .expect("latest attempt");

        assert_eq!(first.result(), Some(&first_result));
        assert_ne!(latest.id(), first.id());
        assert_eq!(
            latest.result_version().value(),
            first.result_version().value() + 1
        );
        assert_eq!(retried, *latest.result().expect("latest result"));
        assert_eq!(provider.0.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn duplicate_submission_id_reuses_the_existing_session_without_a_second_primary_call() {
        let supervisor = super::QuerySupervisor::new();
        let provider = Arc::new(SuccessCountingProvider(AtomicUsize::new(0)));
        let first = supervisor
            .submit_primary_idempotent("main", "submit-1", provider.clone(), provider_request())
            .await
            .expect("first success");
        let duplicate = supervisor
            .submit_primary_idempotent("main", "submit-1", provider.clone(), provider_request())
            .await
            .expect("duplicate success");

        assert_eq!(first.0, duplicate.0);
        assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_duplicate_submission_id_waits_for_one_primary_request() {
        let supervisor = Arc::new(super::QuerySupervisor::new());
        let provider = Arc::new(LateSuccessProvider {
            started: Notify::new(),
            release: Notify::new(),
            calls: AtomicUsize::new(0),
        });
        let first_supervisor = Arc::clone(&supervisor);
        let first_provider: Arc<dyn AiTranslationProvider> = provider.clone();
        let first = tokio::spawn(async move {
            first_supervisor
                .submit_primary_idempotent(
                    "main",
                    "submit-concurrent",
                    first_provider,
                    provider_request(),
                )
                .await
        });
        provider.started.notified().await;
        let second_supervisor = Arc::clone(&supervisor);
        let second_provider: Arc<dyn AiTranslationProvider> = provider.clone();
        let second = tokio::spawn(async move {
            second_supervisor
                .submit_primary_idempotent(
                    "main",
                    "submit-concurrent",
                    second_provider,
                    provider_request(),
                )
                .await
        });

        provider.release.notify_one();
        let first = first.await.expect("first task").expect("first success");
        let second = second.await.expect("second task").expect("second success");

        assert_eq!(first.0, second.0);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    }
}
