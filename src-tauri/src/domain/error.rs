use std::{error::Error, fmt};

use crate::domain::{query_intent::QueryIntent, translation::AttemptStatus};

/// A complete-result payload violates the Evolish result contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultValidationError {
    IntentMismatch {
        expected: QueryIntent,
        actual: QueryIntent,
    },
    UnsupportedSchemaVersion {
        received: u16,
    },
    EmptyPrimaryTranslation,
    EmptyParagraphs,
    EmptyParagraphSource {
        index: u32,
    },
    EmptyParagraphTranslation {
        index: u32,
    },
    DuplicateParagraphIndex {
        index: u32,
    },
    MissingParagraphIndexes,
}

impl fmt::Display for ResultValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntentMismatch { expected, actual } => {
                write!(
                    formatter,
                    "result intent {actual:?} does not match {expected:?}"
                )
            }
            Self::UnsupportedSchemaVersion { received } => {
                write!(formatter, "unsupported result schema version {received}")
            }
            Self::EmptyPrimaryTranslation => {
                formatter.write_str("primary translation must not be empty")
            }
            Self::EmptyParagraphs => {
                formatter.write_str("long-text result must contain paragraphs")
            }
            Self::EmptyParagraphSource { index } => {
                write!(formatter, "long-text paragraph {index} has no source text")
            }
            Self::EmptyParagraphTranslation { index } => {
                write!(formatter, "long-text paragraph {index} has no translation")
            }
            Self::DuplicateParagraphIndex { index } => {
                write!(formatter, "long-text paragraph index {index} is duplicated")
            }
            Self::MissingParagraphIndexes => formatter
                .write_str("long-text paragraph indexes must start at zero and be contiguous"),
        }
    }
}

impl Error for ResultValidationError {}

/// A requested attempt status change violates the attempt state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttemptTransitionError {
    IllegalTransition {
        from: AttemptStatus,
        to: AttemptStatus,
    },
    RetryRequiresCompletedAttempt,
    RetryMustUseNewAttemptId,
    RetryMustIncreaseResultVersion,
}

impl fmt::Display for AttemptTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalTransition { from, to } => {
                write!(
                    formatter,
                    "cannot transition attempt from {from:?} to {to:?}"
                )
            }
            Self::RetryRequiresCompletedAttempt => {
                formatter.write_str("only a succeeded or failed attempt can be retried")
            }
            Self::RetryMustUseNewAttemptId => {
                formatter.write_str("a retry must use a new attempt identifier")
            }
            Self::RetryMustIncreaseResultVersion => {
                formatter.write_str("a retry must increase the result version")
            }
        }
    }
}

impl Error for AttemptTransitionError {}
