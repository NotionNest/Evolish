use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::domain::{error::AttemptTransitionError, query_intent::QueryIntent};

macro_rules! uuid_v7_identifier {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = IdentifierParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let identifier =
                    Uuid::parse_str(value).map_err(|_| IdentifierParseError::InvalidUuid)?;
                if identifier.get_version_num() != 7 {
                    return Err(IdentifierParseError::WrongVersion);
                }

                Ok(Self(identifier))
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::from_str(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

uuid_v7_identifier!(TranslationSessionId);
uuid_v7_identifier!(TranslationAttemptId);
uuid_v7_identifier!(ProviderProfileId);
uuid_v7_identifier!(TranslationModeId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifierParseError {
    InvalidUuid,
    WrongVersion,
}

impl fmt::Display for IdentifierParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuid => formatter.write_str("identifier is not a UUID"),
            Self::WrongVersion => formatter.write_str("identifier must be a UUID v7"),
        }
    }
}

impl Error for IdentifierParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResultVersion(u32);

impl ResultVersion {
    /// Creates a positive result version.
    ///
    /// # Errors
    ///
    /// Returns [`ResultVersionError::Zero`] when `value` is zero.
    pub const fn new(value: u32) -> Result<Self, ResultVersionError> {
        if value == 0 {
            return Err(ResultVersionError::Zero);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn initial() -> Self {
        Self(1)
    }

    /// Returns the next result version.
    ///
    /// # Errors
    ///
    /// Returns [`ResultVersionError::Overflow`] when this is the largest
    /// representable version.
    pub const fn next(self) -> Result<Self, ResultVersionError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(ResultVersionError::Overflow),
        }
    }

    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl Serialize for ResultVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for ResultVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u32::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultVersionError {
    Zero,
    Overflow,
}

impl fmt::Display for ResultVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("result version must be positive"),
            Self::Overflow => formatter.write_str("result version cannot exceed u32::MAX"),
        }
    }
}

impl Error for ResultVersionError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdaptiveTranslationResult {
    Word(WordResultV1),
    Phrase(PhraseResultV1),
    Sentence(SentenceResultV1),
    LongText {
        schema_version: u16,
        paragraphs: Vec<TranslatedParagraphV1>,
    },
}

impl AdaptiveTranslationResult {
    #[must_use]
    pub const fn intent(&self) -> QueryIntent {
        match self {
            Self::Word(_) => QueryIntent::Word,
            Self::Phrase(_) => QueryIntent::Phrase,
            Self::Sentence(_) => QueryIntent::Sentence,
            Self::LongText { .. } => QueryIntent::LongText,
        }
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        match self {
            Self::Word(result) => result.schema_version,
            Self::Phrase(result) => result.schema_version,
            Self::Sentence(result) => result.schema_version,
            Self::LongText { schema_version, .. } => *schema_version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WordResultV1 {
    pub schema_version: u16,
    pub primary_translation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senses: Vec<WordSenseV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipa: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inflections: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collocations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<BilingualExampleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WordSenseV1 {
    pub part_of_speech: PartOfSpeech,
    pub translation: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Preposition,
    Conjunction,
    Interjection,
    Determiner,
    Article,
    Numeral,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhraseResultV1 {
    pub schema_version: u16,
    pub primary_translation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage_notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub natural_translation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<BilingualExampleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SentenceResultV1 {
    pub schema_version: u16,
    pub primary_translation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wording_notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grammar_notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranslatedParagraphV1 {
    pub index: u32,
    pub source_text: String,
    pub translated_text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BilingualExampleV1 {
    pub source_text: String,
    pub translated_text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    NotRequested,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslationAttempt {
    id: TranslationAttemptId,
    result_version: ResultVersion,
    status: AttemptStatus,
}

impl TranslationAttempt {
    #[must_use]
    pub const fn new(id: TranslationAttemptId, result_version: ResultVersion) -> Self {
        Self {
            id,
            result_version,
            status: AttemptStatus::NotRequested,
        }
    }

    #[must_use]
    pub const fn id(self) -> TranslationAttemptId {
        self.id
    }

    #[must_use]
    pub const fn result_version(self) -> ResultVersion {
        self.result_version
    }

    #[must_use]
    pub const fn status(self) -> AttemptStatus {
        self.status
    }

    /// Applies a legal in-place lifecycle status transition.
    ///
    /// # Errors
    ///
    /// Returns [`AttemptTransitionError::IllegalTransition`] when the state
    /// machine does not permit the requested transition.
    pub const fn transition(
        self,
        next_status: AttemptStatus,
    ) -> Result<Self, AttemptTransitionError> {
        if !is_legal_transition(self.status, next_status) {
            return Err(AttemptTransitionError::IllegalTransition {
                from: self.status,
                to: next_status,
            });
        }

        Ok(Self {
            status: next_status,
            ..self
        })
    }

    /// Starts a retry using a new identity and a later result version.
    ///
    /// # Errors
    ///
    /// Returns an error unless this attempt completed, `next_id` differs from
    /// the current ID, and `next_result_version` is greater than the current version.
    pub fn retry(
        self,
        next_id: TranslationAttemptId,
        next_result_version: ResultVersion,
    ) -> Result<Self, AttemptTransitionError> {
        if !matches!(
            self.status,
            AttemptStatus::Succeeded | AttemptStatus::Failed
        ) {
            return Err(AttemptTransitionError::RetryRequiresCompletedAttempt);
        }
        if self.id == next_id {
            return Err(AttemptTransitionError::RetryMustUseNewAttemptId);
        }
        if next_result_version.value() <= self.result_version.value() {
            return Err(AttemptTransitionError::RetryMustIncreaseResultVersion);
        }

        Ok(Self {
            id: next_id,
            result_version: next_result_version,
            status: AttemptStatus::Queued,
        })
    }
}

const fn is_legal_transition(from: AttemptStatus, to: AttemptStatus) -> bool {
    matches!(
        (from, to),
        (AttemptStatus::NotRequested, AttemptStatus::Queued)
            | (
                AttemptStatus::Queued,
                AttemptStatus::Running | AttemptStatus::Cancelled
            )
            | (
                AttemptStatus::Running,
                AttemptStatus::Succeeded | AttemptStatus::Failed | AttemptStatus::Cancelled
            )
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{
        application::translation::result_validator::validate_adaptive_result,
        domain::{
            error::ResultValidationError,
            query_intent::QueryIntent,
            translation::{
                AdaptiveTranslationResult, AttemptStatus, PartOfSpeech, PhraseResultV1,
                SentenceResultV1, TranslatedParagraphV1, TranslationAttempt, WordResultV1,
                WordSenseV1,
            },
        },
    };

    use super::{
        ProviderProfileId, ResultVersion, TranslationAttemptId, TranslationModeId,
        TranslationSessionId,
    };

    #[test]
    fn generates_uuid_v7_stable_identifiers() {
        let session_id = TranslationSessionId::generate();
        let attempt_id = TranslationAttemptId::generate();
        let profile_id = ProviderProfileId::generate();
        let mode_id = TranslationModeId::generate();

        for identifier in [
            session_id.to_string(),
            attempt_id.to_string(),
            profile_id.to_string(),
            mode_id.to_string(),
        ] {
            let parsed = uuid::Uuid::parse_str(&identifier).expect("identifier must be a UUID");
            assert_eq!(parsed.get_version_num(), 7);
        }
    }

    #[test]
    fn parses_only_uuid_v7_identifiers() {
        let id = TranslationSessionId::generate();
        assert_eq!(
            TranslationSessionId::from_str(&id.to_string()).expect("v7 must parse"),
            id
        );
        assert!(TranslationSessionId::from_str("550e8400-e29b-41d4-a716-446655440000").is_err());
        assert!(TranslationSessionId::from_str("not-an-id").is_err());
    }

    #[test]
    fn serializes_identifiers_as_canonical_uuid_strings() {
        let id = TranslationAttemptId::generate();
        let serialized = serde_json::to_string(&id).expect("identifier must serialize");

        assert_eq!(serialized, format!(r#""{id}""#));
        assert_eq!(
            serde_json::from_str::<TranslationAttemptId>(&serialized)
                .expect("identifier must deserialize"),
            id
        );
        assert!(
            serde_json::from_str::<TranslationAttemptId>(
                r#""550e8400-e29b-41d4-a716-446655440000""#
            )
            .is_err()
        );
    }

    #[test]
    fn result_version_is_positive_and_monotonic() {
        assert!(ResultVersion::new(0).is_err());

        let initial = ResultVersion::initial();
        assert_eq!(initial.value(), 1);
        assert_eq!(
            initial.next().expect("version one has a successor").value(),
            2
        );
        assert!(serde_json::from_str::<ResultVersion>("0").is_err());
    }

    #[test]
    fn validates_complete_v1_results_for_every_supported_intent() {
        let word = AdaptiveTranslationResult::Word(WordResultV1 {
            schema_version: 1,
            primary_translation: "偶然发现的美好事物".into(),
            senses: vec![WordSenseV1 {
                part_of_speech: PartOfSpeech::Noun,
                translation: "意外发现的珍奇事物".into(),
            }],
            ipa: Some("ˌserənˈdipədē".into()),
            inflections: Vec::new(),
            collocations: Vec::new(),
            examples: Vec::new(),
        });
        let phrase = AdaptiveTranslationResult::Phrase(PhraseResultV1 {
            schema_version: 1,
            primary_translation: "弥补".into(),
            usage_notes: Vec::new(),
            natural_translation: Some("补偿".into()),
            examples: Vec::new(),
        });
        let sentence = AdaptiveTranslationResult::Sentence(SentenceResultV1 {
            schema_version: 1,
            primary_translation: "我正在学习一门新的语言。".into(),
            wording_notes: Vec::new(),
            grammar_notes: Vec::new(),
        });
        let long_text = AdaptiveTranslationResult::LongText {
            schema_version: 1,
            paragraphs: vec![
                TranslatedParagraphV1 {
                    index: 0,
                    source_text: "First paragraph.".into(),
                    translated_text: "第一段。".into(),
                },
                TranslatedParagraphV1 {
                    index: 1,
                    source_text: "Second paragraph.".into(),
                    translated_text: "第二段。".into(),
                },
            ],
        };

        assert!(validate_adaptive_result(&word, QueryIntent::Word).is_ok());
        assert!(validate_adaptive_result(&phrase, QueryIntent::Phrase).is_ok());
        assert!(validate_adaptive_result(&sentence, QueryIntent::Sentence).is_ok());
        assert!(validate_adaptive_result(&long_text, QueryIntent::LongText).is_ok());
    }

    #[test]
    fn rejects_invalid_or_mismatched_adaptive_results() {
        let empty_word = AdaptiveTranslationResult::Word(WordResultV1 {
            schema_version: 1,
            primary_translation: " \n ".into(),
            senses: Vec::new(),
            ipa: None,
            inflections: Vec::new(),
            collocations: Vec::new(),
            examples: Vec::new(),
        });
        let missing_and_duplicate_paragraphs = AdaptiveTranslationResult::LongText {
            schema_version: 1,
            paragraphs: vec![
                TranslatedParagraphV1 {
                    index: 0,
                    source_text: "First".into(),
                    translated_text: "第一段".into(),
                },
                TranslatedParagraphV1 {
                    index: 2,
                    source_text: "Third".into(),
                    translated_text: "第三段".into(),
                },
                TranslatedParagraphV1 {
                    index: 2,
                    source_text: "Also third".into(),
                    translated_text: "还是第三段".into(),
                },
            ],
        };
        let missing_paragraph_index = AdaptiveTranslationResult::LongText {
            schema_version: 1,
            paragraphs: vec![
                TranslatedParagraphV1 {
                    index: 0,
                    source_text: "First".into(),
                    translated_text: "第一段".into(),
                },
                TranslatedParagraphV1 {
                    index: 2,
                    source_text: "Third".into(),
                    translated_text: "第三段".into(),
                },
            ],
        };

        assert_eq!(
            validate_adaptive_result(&empty_word, QueryIntent::Word),
            Err(ResultValidationError::EmptyPrimaryTranslation)
        );
        assert_eq!(
            validate_adaptive_result(&empty_word, QueryIntent::Sentence),
            Err(ResultValidationError::IntentMismatch {
                expected: QueryIntent::Sentence,
                actual: QueryIntent::Word,
            })
        );
        assert_eq!(
            validate_adaptive_result(&missing_and_duplicate_paragraphs, QueryIntent::LongText),
            Err(ResultValidationError::DuplicateParagraphIndex { index: 2 })
        );
        assert_eq!(
            validate_adaptive_result(&missing_paragraph_index, QueryIntent::LongText),
            Err(ResultValidationError::MissingParagraphIndexes)
        );
    }

    #[test]
    fn uses_tagged_camel_case_result_json_and_rejects_unknown_part_of_speech() {
        let result = AdaptiveTranslationResult::Word(WordResultV1 {
            schema_version: 1,
            primary_translation: "学习".into(),
            senses: vec![WordSenseV1 {
                part_of_speech: PartOfSpeech::Verb,
                translation: "学习".into(),
            }],
            ipa: None,
            inflections: Vec::new(),
            collocations: Vec::new(),
            examples: Vec::new(),
        });

        let serialized = serde_json::to_string(&result).expect("result must serialize");
        assert_eq!(
            serialized,
            r#"{"kind":"word","schemaVersion":1,"primaryTranslation":"学习","senses":[{"partOfSpeech":"verb","translation":"学习"}]}"#
        );
        assert!(serde_json::from_str::<AdaptiveTranslationResult>(
            r#"{"kind":"word","schemaVersion":1,"primaryTranslation":"learn","senses":[{"partOfSpeech":"unknown","translation":"learn"}]}"#
        )
        .is_err());
    }

    #[test]
    fn attempt_state_machine_rejects_illegal_transitions_and_retries_with_new_identity() {
        let initial =
            TranslationAttempt::new(TranslationAttemptId::generate(), ResultVersion::initial());
        assert_eq!(initial.status(), AttemptStatus::NotRequested);
        assert!(initial.transition(AttemptStatus::Succeeded).is_err());

        let completed = initial
            .transition(AttemptStatus::Queued)
            .expect("not requested can queue")
            .transition(AttemptStatus::Running)
            .expect("queued can run")
            .transition(AttemptStatus::Succeeded)
            .expect("running can succeed");
        let retry = completed
            .retry(
                TranslationAttemptId::generate(),
                completed
                    .result_version()
                    .next()
                    .expect("version increments"),
            )
            .expect("completed attempt can retry");

        assert_eq!(retry.status(), AttemptStatus::Queued);
        assert_ne!(retry.id(), completed.id());
        assert!(retry.result_version().value() > completed.result_version().value());
        assert!(
            completed
                .retry(
                    completed.id(),
                    completed
                        .result_version()
                        .next()
                        .expect("version increments")
                )
                .is_err()
        );

        let cancelled =
            TranslationAttempt::new(TranslationAttemptId::generate(), ResultVersion::initial())
                .transition(AttemptStatus::Queued)
                .expect("not requested can queue")
                .transition(AttemptStatus::Cancelled)
                .expect("queued attempt can cancel");
        assert_eq!(cancelled.status(), AttemptStatus::Cancelled);
        assert!(
            cancelled
                .retry(
                    TranslationAttemptId::generate(),
                    ResultVersion::new(2).expect("positive version")
                )
                .is_err()
        );

        let failed =
            TranslationAttempt::new(TranslationAttemptId::generate(), ResultVersion::initial())
                .transition(AttemptStatus::Queued)
                .expect("not requested can queue")
                .transition(AttemptStatus::Running)
                .expect("queued can run")
                .transition(AttemptStatus::Failed)
                .expect("running attempt can fail");
        assert_eq!(failed.status(), AttemptStatus::Failed);
    }
}
