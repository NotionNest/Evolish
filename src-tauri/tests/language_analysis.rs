use evolish_lib::{
    application::translation::{
        classifier::{ClassificationReason, IntentClassifier},
        language_direction::{
            DirectionResolutionError, LanguageDirectionPreferences, LanguageDirectionResolver,
            TargetLanguageSelection,
        },
        ports::{DetectionReason, LanguageDetectorPort},
    },
    domain::{
        language::{Language, LanguageSelection},
        query_intent::{IntentSelection, QueryIntent},
    },
    infrastructure::language::lingua_detector::LinguaDetector,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct LanguageFixture {
    language: Language,
    text: String,
}

#[test]
fn resolves_primary_secondary_manual_and_uncertain_directions() {
    let preferences =
        LanguageDirectionPreferences::new(Language::English, Language::ChineseSimplified)
            .expect("different configured targets");
    let detector = LinguaDetector::new();

    let chinese = detector.detect("我正在学习一门新的语言。");
    let automatic = LanguageDirectionResolver::resolve(
        preferences,
        LanguageSelection::Automatic,
        TargetLanguageSelection::Automatic,
        &chinese,
    )
    .expect("Chinese source should use primary English target");
    assert_eq!(automatic.source(), Language::ChineseSimplified);
    assert_eq!(automatic.target(), Language::English);

    let english = detector.detect("Language learning takes deliberate practice.");
    let secondary = LanguageDirectionResolver::resolve(
        preferences,
        LanguageSelection::Automatic,
        TargetLanguageSelection::Automatic,
        &english,
    )
    .expect("primary-language source should use secondary target");
    assert_eq!(secondary.source(), Language::English);
    assert_eq!(secondary.target(), Language::ChineseSimplified);

    let manual = LanguageDirectionResolver::resolve(
        preferences,
        LanguageSelection::Manual(Language::Japanese),
        TargetLanguageSelection::Manual(Language::Korean),
        &chinese,
    )
    .expect("manual source and target overrides are authoritative");
    assert_eq!(manual.source(), Language::Japanese);
    assert_eq!(manual.target(), Language::Korean);

    let same_language = LanguageDirectionResolver::resolve(
        preferences,
        LanguageSelection::Manual(Language::English),
        TargetLanguageSelection::Manual(Language::English),
        &chinese,
    );
    assert_eq!(
        same_language,
        Err(DirectionResolutionError::SameLanguage(Language::English))
    );

    let uncertain = detector.detect("hello 世界");
    assert_eq!(uncertain.language(), None);
    assert_eq!(uncertain.reason(), DetectionReason::MixedScript);
    assert_eq!(
        LanguageDirectionResolver::resolve(
            preferences,
            LanguageSelection::Automatic,
            TargetLanguageSelection::Automatic,
            &uncertain,
        ),
        Err(DirectionResolutionError::SourceLanguageUncertain)
    );

    let short = detector.detect("x");
    assert_eq!(short.language(), None);
    assert_eq!(short.reason(), DetectionReason::LowConfidence);
}

#[test]
fn locally_detects_the_48_language_corpus_with_deterministic_directions() {
    let fixtures: Vec<LanguageFixture> =
        serde_json::from_str(include_str!("fixtures/language_detection.json"))
            .expect("fixture must be valid JSON");
    assert_eq!(fixtures.len(), Language::ALL.len());

    let detector = LinguaDetector::new();
    let first_pass: Vec<_> = fixtures
        .iter()
        .map(|fixture| detector.detect(&fixture.text))
        .collect();
    let second_pass: Vec<_> = fixtures
        .iter()
        .map(|fixture| detector.detect(&fixture.text))
        .collect();
    assert_eq!(
        first_pass, second_pass,
        "local detection must be deterministic"
    );

    let correct = fixtures
        .iter()
        .zip(&first_pass)
        .filter(|(fixture, result)| result.language() == Some(fixture.language))
        .count();
    assert!(
        correct * 100 >= fixtures.len() * 95,
        "expected at least 95% correct directions, got {correct}/{}",
        fixtures.len()
    );
}

#[test]
fn classifies_text_deterministically_and_honors_manual_override() {
    let classifier = IntentClassifier;

    assert_eq!(
        classifier
            .classify("serendipity", IntentSelection::Automatic)
            .intent(),
        QueryIntent::Word
    );
    assert_eq!(
        classifier
            .classify("make up for", IntentSelection::Automatic)
            .intent(),
        QueryIntent::Phrase
    );
    assert_eq!(
        classifier
            .classify("I am learning a new language.", IntentSelection::Automatic)
            .intent(),
        QueryIntent::Sentence
    );
    assert_eq!(
        classifier
            .classify(
                "First paragraph.\n\nSecond paragraph.",
                IntentSelection::Automatic
            )
            .intent(),
        QueryIntent::LongText
    );

    let manual = classifier.classify("word", IntentSelection::Manual(QueryIntent::LongText));
    assert_eq!(manual.intent(), QueryIntent::LongText);
    assert_eq!(manual.reason(), ClassificationReason::ManualOverride);
}
