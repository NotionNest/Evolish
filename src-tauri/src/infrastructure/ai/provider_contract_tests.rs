use crate::infrastructure::ai::contract_fixtures::{
    ProviderErrorKind, ProviderRequestError, ProviderTranslationRequest, assert_provider_contract,
    fixture_provider, provider_request,
};

#[tokio::test]
async fn shared_provider_contract_accepts_complete_results_and_classifies_failures() {
    assert_provider_contract(fixture_provider()).await;
}

#[test]
fn provider_contract_fixtures_and_public_errors_never_contain_a_secret_canary() {
    const SECRET_CANARY: &str = "evolish-secret-canary";
    let request = provider_request();
    let fixture = include_str!("../../../tests/fixtures/providers/provider_contract.json");

    assert!(!format!("{request:?}").contains(SECRET_CANARY));
    assert!(!fixture.contains(SECRET_CANARY));
    assert_eq!(
        ProviderErrorKind::AuthenticationFailed.code(),
        "translation.provider.authentication_failed"
    );
    assert!(matches!(
        ProviderTranslationRequest::new(
            request.direction(),
            request.intent(),
            request.prompt().clone(),
            request.schema_version(),
            request.timeout(),
            serde_json::json!({"api_key": SECRET_CANARY}),
        ),
        Err(ProviderRequestError::SecretInParameters)
    ));
}
