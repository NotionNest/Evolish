use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// The closed Phase 1 language catalog shared by all provider adapters.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ChineseSimplified,
    #[serde(rename = "zh-TW")]
    ChineseTraditional,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "sv")]
    Swedish,
    #[serde(rename = "ro")]
    Romanian,
    #[serde(rename = "th")]
    Thai,
    #[serde(rename = "sk")]
    Slovak,
    #[serde(rename = "nl")]
    Dutch,
    #[serde(rename = "hu")]
    Hungarian,
    #[serde(rename = "el")]
    Greek,
    #[serde(rename = "da")]
    Danish,
    #[serde(rename = "fi")]
    Finnish,
    #[serde(rename = "pl")]
    Polish,
    #[serde(rename = "cs")]
    Czech,
    #[serde(rename = "tr")]
    Turkish,
    #[serde(rename = "lt")]
    Lithuanian,
    #[serde(rename = "lv")]
    Latvian,
    #[serde(rename = "uk")]
    Ukrainian,
    #[serde(rename = "bg")]
    Bulgarian,
    #[serde(rename = "id")]
    Indonesian,
    #[serde(rename = "ms")]
    Malay,
    #[serde(rename = "sl")]
    Slovenian,
    #[serde(rename = "et")]
    Estonian,
    #[serde(rename = "vi")]
    Vietnamese,
    #[serde(rename = "fa")]
    Persian,
    #[serde(rename = "hi")]
    Hindi,
    #[serde(rename = "te")]
    Telugu,
    #[serde(rename = "ta")]
    Tamil,
    #[serde(rename = "ur")]
    Urdu,
    #[serde(rename = "fil")]
    Filipino,
    #[serde(rename = "km")]
    Khmer,
    #[serde(rename = "lo")]
    Lao,
    #[serde(rename = "bn")]
    Bengali,
    #[serde(rename = "my")]
    Burmese,
    #[serde(rename = "nb")]
    Norwegian,
    #[serde(rename = "sr")]
    Serbian,
    #[serde(rename = "hr")]
    Croatian,
    #[serde(rename = "mn")]
    Mongolian,
    #[serde(rename = "he")]
    Hebrew,
}

impl Language {
    pub const ALL: [Self; 48] = [
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::English,
        Self::Japanese,
        Self::Korean,
        Self::French,
        Self::Spanish,
        Self::Portuguese,
        Self::Italian,
        Self::German,
        Self::Russian,
        Self::Arabic,
        Self::Swedish,
        Self::Romanian,
        Self::Thai,
        Self::Slovak,
        Self::Dutch,
        Self::Hungarian,
        Self::Greek,
        Self::Danish,
        Self::Finnish,
        Self::Polish,
        Self::Czech,
        Self::Turkish,
        Self::Lithuanian,
        Self::Latvian,
        Self::Ukrainian,
        Self::Bulgarian,
        Self::Indonesian,
        Self::Malay,
        Self::Slovenian,
        Self::Estonian,
        Self::Vietnamese,
        Self::Persian,
        Self::Hindi,
        Self::Telugu,
        Self::Tamil,
        Self::Urdu,
        Self::Filipino,
        Self::Khmer,
        Self::Lao,
        Self::Bengali,
        Self::Burmese,
        Self::Norwegian,
        Self::Serbian,
        Self::Croatian,
        Self::Mongolian,
        Self::Hebrew,
    ];

    #[must_use]
    pub const fn bcp47(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "zh-CN",
            Self::ChineseTraditional => "zh-TW",
            Self::English => "en",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::French => "fr",
            Self::Spanish => "es",
            Self::Portuguese => "pt",
            Self::Italian => "it",
            Self::German => "de",
            Self::Russian => "ru",
            Self::Arabic => "ar",
            Self::Swedish => "sv",
            Self::Romanian => "ro",
            Self::Thai => "th",
            Self::Slovak => "sk",
            Self::Dutch => "nl",
            Self::Hungarian => "hu",
            Self::Greek => "el",
            Self::Danish => "da",
            Self::Finnish => "fi",
            Self::Polish => "pl",
            Self::Czech => "cs",
            Self::Turkish => "tr",
            Self::Lithuanian => "lt",
            Self::Latvian => "lv",
            Self::Ukrainian => "uk",
            Self::Bulgarian => "bg",
            Self::Indonesian => "id",
            Self::Malay => "ms",
            Self::Slovenian => "sl",
            Self::Estonian => "et",
            Self::Vietnamese => "vi",
            Self::Persian => "fa",
            Self::Hindi => "hi",
            Self::Telugu => "te",
            Self::Tamil => "ta",
            Self::Urdu => "ur",
            Self::Filipino => "fil",
            Self::Khmer => "km",
            Self::Lao => "lo",
            Self::Bengali => "bn",
            Self::Burmese => "my",
            Self::Norwegian => "nb",
            Self::Serbian => "sr",
            Self::Croatian => "hr",
            Self::Mongolian => "mn",
            Self::Hebrew => "he",
        }
    }

    #[must_use]
    pub const fn writing_direction(self) -> WritingDirection {
        match self {
            Self::Arabic | Self::Hebrew | Self::Persian | Self::Urdu => {
                WritingDirection::RightToLeft
            }
            _ => WritingDirection::LeftToRight,
        }
    }

    /// Returns a lossless language code in the format required by an adapter.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested format cannot preserve this catalog
    /// entry's language or regional distinction.
    pub const fn provider_code(
        self,
        format: ProviderLanguageCodeFormat,
    ) -> Result<&'static str, ProviderLanguageMappingError> {
        match format {
            ProviderLanguageCodeFormat::Bcp47 => Ok(self.bcp47()),
            ProviderLanguageCodeFormat::Iso639_1 => match self {
                Self::ChineseSimplified | Self::ChineseTraditional => {
                    Err(ProviderLanguageMappingError {
                        language: self,
                        format,
                    })
                }
                Self::Filipino => Ok("tl"),
                _ => Ok(self.bcp47()),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLanguageCodeFormat {
    Bcp47,
    Iso639_1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderLanguageMappingError {
    language: Language,
    format: ProviderLanguageCodeFormat,
}

impl ProviderLanguageMappingError {
    #[must_use]
    pub const fn language(self) -> Language {
        self.language
    }

    #[must_use]
    pub const fn format(self) -> ProviderLanguageCodeFormat {
        self.format
    }
}

impl fmt::Display for ProviderLanguageMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} cannot be represented losslessly as {:?}",
            self.language.bcp47(),
            self.format
        )
    }
}

impl Error for ProviderLanguageMappingError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSelection {
    Automatic,
    Manual(Language),
}

impl LanguageSelection {
    #[must_use]
    pub const fn resolve(self, detected_language: Language) -> Language {
        match self {
            Self::Automatic => detected_language,
            Self::Manual(language) => language,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageDirection {
    source: Language,
    target: Language,
}

impl LanguageDirection {
    /// Builds a direction only when the source and target are distinct.
    ///
    /// # Errors
    ///
    /// Returns [`LanguageDirectionError::SameLanguage`] when both endpoints
    /// refer to the same catalog entry.
    pub fn new(source: Language, target: Language) -> Result<Self, LanguageDirectionError> {
        if source == target {
            return Err(LanguageDirectionError::SameLanguage(source));
        }

        Ok(Self { source, target })
    }

    #[must_use]
    pub const fn source(self) -> Language {
        self.source
    }

    #[must_use]
    pub const fn target(self) -> Language {
        self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageDirectionError {
    SameLanguage(Language),
}

impl fmt::Display for LanguageDirectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameLanguage(language) => {
                write!(formatter, "source and target are both {}", language.bcp47())
            }
        }
    }
}

impl Error for LanguageDirectionError {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        Language, LanguageDirection, LanguageDirectionError, LanguageSelection,
        ProviderLanguageCodeFormat, WritingDirection,
    };

    #[test]
    fn catalog_contains_48_unique_bcp47_tags() {
        assert_eq!(Language::ALL.len(), 48);

        let tags = Language::ALL
            .iter()
            .map(|language| language.bcp47())
            .collect::<HashSet<_>>();

        assert_eq!(tags.len(), Language::ALL.len());
        assert!(tags.contains("zh-CN"));
        assert!(tags.contains("zh-TW"));
        assert!(tags.contains("en"));
        assert!(tags.contains("he"));
    }

    #[test]
    fn catalog_exposes_writing_direction() {
        assert_eq!(
            Language::English.writing_direction(),
            WritingDirection::LeftToRight
        );
        assert_eq!(
            Language::Arabic.writing_direction(),
            WritingDirection::RightToLeft
        );
        assert_eq!(
            Language::Hebrew.writing_direction(),
            WritingDirection::RightToLeft
        );
    }

    #[test]
    fn rejects_lossy_provider_code_mapping() {
        let error = Language::ChineseSimplified
            .provider_code(ProviderLanguageCodeFormat::Iso639_1)
            .expect_err(
                "regional Chinese must not silently collapse to an ambiguous ISO 639-1 code",
            );

        assert_eq!(error.language(), Language::ChineseSimplified);
        assert_eq!(error.format(), ProviderLanguageCodeFormat::Iso639_1);
    }

    #[test]
    fn serializes_stable_bcp47_language_codes() {
        let serialized =
            serde_json::to_string(&Language::ChineseTraditional).expect("language must serialize");
        assert_eq!(serialized, r#""zh-TW""#);

        let deserialized: Language =
            serde_json::from_str(&serialized).expect("stable language code must deserialize");
        assert_eq!(deserialized, Language::ChineseTraditional);
        assert!(serde_json::from_str::<Language>(r#""unknown""#).is_err());
    }

    #[test]
    fn manual_language_selection_and_direction_are_explicit() {
        assert_eq!(
            LanguageSelection::Automatic.resolve(Language::Japanese),
            Language::Japanese
        );
        assert_eq!(
            LanguageSelection::Manual(Language::French).resolve(Language::Japanese),
            Language::French
        );

        let direction = LanguageDirection::new(Language::English, Language::ChineseSimplified)
            .expect("different languages form a direction");
        assert_eq!(direction.source(), Language::English);
        assert_eq!(direction.target(), Language::ChineseSimplified);
        assert_eq!(
            LanguageDirection::new(Language::English, Language::English),
            Err(LanguageDirectionError::SameLanguage(Language::English))
        );
    }
}
