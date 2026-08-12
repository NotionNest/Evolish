use std::sync::OnceLock;

use lingua::{Language as LinguaLanguage, LanguageDetector, LanguageDetectorBuilder};

use crate::{
    application::translation::ports::{DetectionReason, LanguageDetection, LanguageDetectorPort},
    domain::language::Language,
};

const CONFIDENCE_THRESHOLD_PERCENT: u8 = 70;

/// The local Lingua adapter shared by translation requests.
#[derive(Default)]
pub struct LinguaDetector;

impl LinguaDetector {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LanguageDetectorPort for LinguaDetector {
    fn detect(&self, text: &str) -> LanguageDetection {
        if contains_mixed_meaningful_scripts(text) {
            return LanguageDetection::uncertain(0, DetectionReason::MixedScript);
        }
        if let Some(language) = language_from_exclusive_script(text) {
            return LanguageDetection::certain(language, 100, DetectionReason::ScriptRule);
        }

        let confidences = detector().compute_language_confidence_values(text);
        let Some((language, confidence)) = confidences.first().copied() else {
            return LanguageDetection::uncertain(0, DetectionReason::LowConfidence);
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let confidence_percent = (confidence * 100.0).round().clamp(0.0, 100.0) as u8;
        if confidence_percent < CONFIDENCE_THRESHOLD_PERCENT {
            return LanguageDetection::uncertain(
                confidence_percent,
                DetectionReason::LowConfidence,
            );
        }
        LanguageDetection::certain(
            map_lingua_language(language),
            confidence_percent,
            DetectionReason::LinguaModel,
        )
    }
}

fn detector() -> &'static LanguageDetector {
    static DETECTOR: OnceLock<LanguageDetector> = OnceLock::new();
    DETECTOR.get_or_init(|| {
        let mut builder = LanguageDetectorBuilder::from_languages(&lingua_languages());
        builder.with_minimum_relative_distance(0.1);
        builder.build()
    })
}

fn lingua_languages() -> [LinguaLanguage; 44] {
    [
        LinguaLanguage::Arabic,
        LinguaLanguage::Bengali,
        LinguaLanguage::Bokmal,
        LinguaLanguage::Bulgarian,
        LinguaLanguage::Chinese,
        LinguaLanguage::Croatian,
        LinguaLanguage::Czech,
        LinguaLanguage::Danish,
        LinguaLanguage::Dutch,
        LinguaLanguage::English,
        LinguaLanguage::Estonian,
        LinguaLanguage::Finnish,
        LinguaLanguage::French,
        LinguaLanguage::German,
        LinguaLanguage::Greek,
        LinguaLanguage::Hebrew,
        LinguaLanguage::Hindi,
        LinguaLanguage::Hungarian,
        LinguaLanguage::Indonesian,
        LinguaLanguage::Italian,
        LinguaLanguage::Japanese,
        LinguaLanguage::Korean,
        LinguaLanguage::Latvian,
        LinguaLanguage::Lithuanian,
        LinguaLanguage::Malay,
        LinguaLanguage::Mongolian,
        LinguaLanguage::Persian,
        LinguaLanguage::Polish,
        LinguaLanguage::Portuguese,
        LinguaLanguage::Romanian,
        LinguaLanguage::Russian,
        LinguaLanguage::Serbian,
        LinguaLanguage::Slovak,
        LinguaLanguage::Slovene,
        LinguaLanguage::Spanish,
        LinguaLanguage::Swedish,
        LinguaLanguage::Tagalog,
        LinguaLanguage::Tamil,
        LinguaLanguage::Telugu,
        LinguaLanguage::Thai,
        LinguaLanguage::Turkish,
        LinguaLanguage::Ukrainian,
        LinguaLanguage::Urdu,
        LinguaLanguage::Vietnamese,
    ]
}

fn language_from_exclusive_script(text: &str) -> Option<Language> {
    if text
        .chars()
        .any(|character| ('\u{3040}'..='\u{30ff}').contains(&character))
    {
        return Some(Language::Japanese);
    }
    if text
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7af}').contains(&character))
    {
        return Some(Language::Korean);
    }
    if text
        .chars()
        .any(|character| ('\u{0370}'..='\u{03ff}').contains(&character))
    {
        return Some(Language::Greek);
    }
    if text
        .chars()
        .any(|character| ('\u{0590}'..='\u{05ff}').contains(&character))
    {
        return Some(Language::Hebrew);
    }
    if text
        .chars()
        .any(|character| ('\u{0900}'..='\u{097f}').contains(&character))
    {
        return Some(Language::Hindi);
    }
    if text
        .chars()
        .any(|character| ('\u{0980}'..='\u{09ff}').contains(&character))
    {
        return Some(Language::Bengali);
    }
    if text
        .chars()
        .any(|character| ('\u{0b80}'..='\u{0bff}').contains(&character))
    {
        return Some(Language::Tamil);
    }
    if text
        .chars()
        .any(|character| ('\u{0c00}'..='\u{0c7f}').contains(&character))
    {
        return Some(Language::Telugu);
    }
    if text
        .chars()
        .any(|character| ('\u{0e00}'..='\u{0e7f}').contains(&character))
    {
        return Some(Language::Thai);
    }
    if text
        .chars()
        .any(|character| ('\u{1780}'..='\u{17ff}').contains(&character))
    {
        return Some(Language::Khmer);
    }
    if text
        .chars()
        .any(|character| ('\u{0e80}'..='\u{0eff}').contains(&character))
    {
        return Some(Language::Lao);
    }
    if text
        .chars()
        .any(|character| ('\u{1000}'..='\u{109f}').contains(&character))
    {
        return Some(Language::Burmese);
    }
    if text.chars().any(is_cjk) {
        return Some(if text.chars().any(is_traditional_chinese) {
            Language::ChineseTraditional
        } else {
            Language::ChineseSimplified
        });
    }
    None
}

fn contains_mixed_meaningful_scripts(text: &str) -> bool {
    let has_latin = text
        .chars()
        .any(|character| character.is_alphabetic() && character.is_ascii());
    let has_non_latin = text.chars().any(|character| {
        is_cjk(character)
            || ('\u{0400}'..='\u{052f}').contains(&character)
            || ('\u{0600}'..='\u{06ff}').contains(&character)
            || ('\u{0900}'..='\u{0dff}').contains(&character)
            || ('\u{0e00}'..='\u{0eff}').contains(&character)
            || ('\u{1000}'..='\u{17ff}').contains(&character)
            || ('\u{1780}'..='\u{17ff}').contains(&character)
    });
    has_latin && has_non_latin
}

const fn is_cjk(character: char) -> bool {
    character >= '\u{4e00}' && character <= '\u{9fff}'
}

const fn is_traditional_chinese(character: char) -> bool {
    matches!(character, '學' | '習' | '語' | '譯' | '門' | '練' | '並')
}

fn map_lingua_language(language: LinguaLanguage) -> Language {
    match language {
        LinguaLanguage::Arabic => Language::Arabic,
        LinguaLanguage::Bengali => Language::Bengali,
        LinguaLanguage::Bokmal => Language::Norwegian,
        LinguaLanguage::Bulgarian => Language::Bulgarian,
        LinguaLanguage::Chinese => Language::ChineseSimplified,
        LinguaLanguage::Croatian => Language::Croatian,
        LinguaLanguage::Czech => Language::Czech,
        LinguaLanguage::Danish => Language::Danish,
        LinguaLanguage::Dutch => Language::Dutch,
        LinguaLanguage::English => Language::English,
        LinguaLanguage::Estonian => Language::Estonian,
        LinguaLanguage::Finnish => Language::Finnish,
        LinguaLanguage::French => Language::French,
        LinguaLanguage::German => Language::German,
        LinguaLanguage::Greek => Language::Greek,
        LinguaLanguage::Hebrew => Language::Hebrew,
        LinguaLanguage::Hindi => Language::Hindi,
        LinguaLanguage::Hungarian => Language::Hungarian,
        LinguaLanguage::Indonesian => Language::Indonesian,
        LinguaLanguage::Italian => Language::Italian,
        LinguaLanguage::Japanese => Language::Japanese,
        LinguaLanguage::Korean => Language::Korean,
        LinguaLanguage::Latvian => Language::Latvian,
        LinguaLanguage::Lithuanian => Language::Lithuanian,
        LinguaLanguage::Malay => Language::Malay,
        LinguaLanguage::Mongolian => Language::Mongolian,
        LinguaLanguage::Persian => Language::Persian,
        LinguaLanguage::Polish => Language::Polish,
        LinguaLanguage::Portuguese => Language::Portuguese,
        LinguaLanguage::Romanian => Language::Romanian,
        LinguaLanguage::Russian => Language::Russian,
        LinguaLanguage::Serbian => Language::Serbian,
        LinguaLanguage::Slovak => Language::Slovak,
        LinguaLanguage::Slovene => Language::Slovenian,
        LinguaLanguage::Spanish => Language::Spanish,
        LinguaLanguage::Swedish => Language::Swedish,
        LinguaLanguage::Tagalog => Language::Filipino,
        LinguaLanguage::Tamil => Language::Tamil,
        LinguaLanguage::Telugu => Language::Telugu,
        LinguaLanguage::Thai => Language::Thai,
        LinguaLanguage::Turkish => Language::Turkish,
        LinguaLanguage::Ukrainian => Language::Ukrainian,
        LinguaLanguage::Urdu => Language::Urdu,
        LinguaLanguage::Vietnamese => Language::Vietnamese,
    }
}
