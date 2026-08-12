use serde::{Deserialize, Serialize};

/// The result shape selected for a translation query.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    Word,
    Phrase,
    Sentence,
    LongText,
}

impl QueryIntent {
    #[must_use]
    pub const fn is_supported(self) -> bool {
        true
    }
}

/// A user override is authoritative over the later local classifier result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSelection {
    Automatic,
    Manual(QueryIntent),
}

impl IntentSelection {
    #[must_use]
    pub const fn resolve(self, detected_intent: QueryIntent) -> QueryIntent {
        match self {
            Self::Automatic => detected_intent,
            Self::Manual(intent) => intent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IntentSelection, QueryIntent};

    struct IntentFixture {
        name: &'static str,
        text: &'static str,
        expected: QueryIntent,
    }

    #[test]
    fn fixture_contract_covers_supported_input_shapes() {
        let fixtures = [
            IntentFixture {
                name: "english word",
                text: "serendipity",
                expected: QueryIntent::Word,
            },
            IntentFixture {
                name: "chinese word",
                text: "学习",
                expected: QueryIntent::Word,
            },
            IntentFixture {
                name: "english phrase",
                text: "make up for",
                expected: QueryIntent::Phrase,
            },
            IntentFixture {
                name: "chinese sentence",
                text: "我正在学习一门新的语言。",
                expected: QueryIntent::Sentence,
            },
            IntentFixture {
                name: "paragraph",
                text: "The first paragraph explains the idea.\n\nThe second paragraph adds context.",
                expected: QueryIntent::LongText,
            },
            IntentFixture {
                name: "mixed unicode",
                text: "在 Rust 中使用 UUID v7。",
                expected: QueryIntent::Sentence,
            },
            IntentFixture {
                name: "code",
                text: "let id = Uuid::now_v7();",
                expected: QueryIntent::Phrase,
            },
        ];

        assert_eq!(fixtures.len(), 7);
        assert!(fixtures.iter().all(|fixture| !fixture.name.is_empty()));
        assert!(fixtures.iter().all(|fixture| !fixture.text.is_empty()));
        assert!(
            fixtures
                .iter()
                .all(|fixture| fixture.expected.is_supported())
        );
    }

    #[test]
    fn manual_intent_override_wins_over_detected_intent() {
        let selection = IntentSelection::Manual(QueryIntent::LongText);

        assert_eq!(selection.resolve(QueryIntent::Word), QueryIntent::LongText);
        assert_eq!(
            IntentSelection::Automatic.resolve(QueryIntent::Word),
            QueryIntent::Word
        );
    }

    #[test]
    fn serializes_stable_query_intent_values() {
        let serialized =
            serde_json::to_string(&QueryIntent::LongText).expect("intent must serialize");
        assert_eq!(serialized, r#""long_text""#);
        assert_eq!(
            serde_json::from_str::<QueryIntent>(&serialized)
                .expect("stable intent value must deserialize"),
            QueryIntent::LongText
        );
        assert!(serde_json::from_str::<QueryIntent>(r#""unknown""#).is_err());
    }
}
