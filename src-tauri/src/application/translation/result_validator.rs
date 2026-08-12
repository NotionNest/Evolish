use std::collections::BTreeSet;

use crate::domain::{
    error::ResultValidationError,
    query_intent::QueryIntent,
    translation::{AdaptiveTranslationResult, TranslatedParagraphV1},
};

const ADAPTIVE_RESULT_SCHEMA_VERSION: u16 = 1;

/// Validates a complete provider result before it can cross the UI boundary.
///
/// # Errors
///
/// Returns a typed error when the result's intent, schema version, primary
/// translation, or long-text paragraph structure is invalid.
pub fn validate_adaptive_result(
    result: &AdaptiveTranslationResult,
    expected_intent: QueryIntent,
) -> Result<(), ResultValidationError> {
    let actual_intent = result.intent();
    if actual_intent != expected_intent {
        return Err(ResultValidationError::IntentMismatch {
            expected: expected_intent,
            actual: actual_intent,
        });
    }

    if result.schema_version() != ADAPTIVE_RESULT_SCHEMA_VERSION {
        return Err(ResultValidationError::UnsupportedSchemaVersion {
            received: result.schema_version(),
        });
    }

    match result {
        AdaptiveTranslationResult::Word(result) => {
            validate_primary_translation(&result.primary_translation)
        }
        AdaptiveTranslationResult::Phrase(result) => {
            validate_primary_translation(&result.primary_translation)
        }
        AdaptiveTranslationResult::Sentence(result) => {
            validate_primary_translation(&result.primary_translation)
        }
        AdaptiveTranslationResult::LongText { paragraphs, .. } => validate_paragraphs(paragraphs),
    }
}

fn validate_primary_translation(translation: &str) -> Result<(), ResultValidationError> {
    if translation.trim().is_empty() {
        return Err(ResultValidationError::EmptyPrimaryTranslation);
    }

    Ok(())
}

fn validate_paragraphs(paragraphs: &[TranslatedParagraphV1]) -> Result<(), ResultValidationError> {
    if paragraphs.is_empty() {
        return Err(ResultValidationError::EmptyParagraphs);
    }

    let mut indexes = BTreeSet::new();
    for paragraph in paragraphs {
        if paragraph.source_text.trim().is_empty() {
            return Err(ResultValidationError::EmptyParagraphSource {
                index: paragraph.index,
            });
        }
        if paragraph.translated_text.trim().is_empty() {
            return Err(ResultValidationError::EmptyParagraphTranslation {
                index: paragraph.index,
            });
        }
        if !indexes.insert(paragraph.index) {
            return Err(ResultValidationError::DuplicateParagraphIndex {
                index: paragraph.index,
            });
        }
    }

    if indexes
        .iter()
        .copied()
        .enumerate()
        .any(|(expected, actual)| usize::try_from(actual) != Ok(expected))
    {
        return Err(ResultValidationError::MissingParagraphIndexes);
    }

    Ok(())
}
