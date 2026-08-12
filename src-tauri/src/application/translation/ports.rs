use crate::domain::language::Language;

/// The explanation attached to a local source-language prediction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetectionReason {
    /// A language-exclusive Unicode script identified the language.
    ScriptRule,
    /// Lingua's locally loaded statistical model identified the language.
    LinguaModel,
    /// More than one meaningful writing script was found.
    MixedScript,
    /// The local model could not distinguish a language with enough confidence.
    LowConfidence,
}

/// A local language prediction that represents uncertainty explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageDetection {
    language: Option<Language>,
    confidence_percent: u8,
    reason: DetectionReason,
}

impl LanguageDetection {
    #[must_use]
    pub const fn certain(
        language: Language,
        confidence_percent: u8,
        reason: DetectionReason,
    ) -> Self {
        Self {
            language: Some(language),
            confidence_percent,
            reason,
        }
    }

    #[must_use]
    pub const fn uncertain(confidence_percent: u8, reason: DetectionReason) -> Self {
        Self {
            language: None,
            confidence_percent,
            reason,
        }
    }

    #[must_use]
    pub const fn language(self) -> Option<Language> {
        self.language
    }

    #[must_use]
    pub const fn confidence_percent(self) -> u8 {
        self.confidence_percent
    }

    #[must_use]
    pub const fn reason(self) -> DetectionReason {
        self.reason
    }
}

/// Local-only source-language detection boundary.
pub trait LanguageDetectorPort: Send + Sync {
    /// Returns a certain or explicitly uncertain local prediction for `text`.
    fn detect(&self, text: &str) -> LanguageDetection;
}
