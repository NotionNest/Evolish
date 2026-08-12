use unicode_segmentation::UnicodeSegmentation;

use crate::domain::query_intent::{IntentSelection, QueryIntent};

/// Why the local classifier chose its query intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassificationReason {
    ManualOverride,
    ParagraphBreak,
    TextLength,
    TerminalPunctuation,
    TokenCount,
}

/// Deterministic local text-classification output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextClassification {
    intent: QueryIntent,
    confidence_percent: u8,
    reason: ClassificationReason,
}

impl TextClassification {
    #[must_use]
    pub const fn intent(self) -> QueryIntent {
        self.intent
    }

    #[must_use]
    pub const fn confidence_percent(self) -> u8 {
        self.confidence_percent
    }

    #[must_use]
    pub const fn reason(self) -> ClassificationReason {
        self.reason
    }
}

/// A pure classifier for the four accepted result shapes.
#[derive(Default)]
pub struct IntentClassifier;

impl IntentClassifier {
    #[must_use]
    pub fn classify(&self, text: &str, selection: IntentSelection) -> TextClassification {
        if let IntentSelection::Manual(intent) = selection {
            return TextClassification {
                intent,
                confidence_percent: 100,
                reason: ClassificationReason::ManualOverride,
            };
        }

        let trimmed = text.trim();
        let paragraph_count = trimmed
            .split("\n\n")
            .filter(|paragraph| !paragraph.trim().is_empty())
            .count();
        if paragraph_count > 1 {
            return TextClassification {
                intent: QueryIntent::LongText,
                confidence_percent: 100,
                reason: ClassificationReason::ParagraphBreak,
            };
        }
        if trimmed.graphemes(true).count() >= 500 {
            return TextClassification {
                intent: QueryIntent::LongText,
                confidence_percent: 95,
                reason: ClassificationReason::TextLength,
            };
        }
        if trimmed.ends_with(['.', '!', '?', '。', '！', '？']) {
            return TextClassification {
                intent: QueryIntent::Sentence,
                confidence_percent: 95,
                reason: ClassificationReason::TerminalPunctuation,
            };
        }
        let token_count = trimmed.unicode_words().count();
        let intent = if token_count <= 1 && !trimmed.contains(char::is_whitespace) {
            QueryIntent::Word
        } else {
            QueryIntent::Phrase
        };
        TextClassification {
            intent,
            confidence_percent: 85,
            reason: ClassificationReason::TokenCount,
        }
    }
}
