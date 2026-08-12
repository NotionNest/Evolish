use crate::{
    application::translation::ports::LanguageDetection,
    domain::language::{Language, LanguageDirection, LanguageSelection},
};

/// Target selection for one translation query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetLanguageSelection {
    Automatic,
    Manual(Language),
}

/// The configured automatic target pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageDirectionPreferences {
    primary_target: Language,
    secondary_target: Language,
}

impl LanguageDirectionPreferences {
    /// Creates distinct automatic target languages.
    ///
    /// # Errors
    ///
    /// Returns [`DirectionResolutionError::SameLanguage`] when targets match.
    pub fn new(
        primary_target: Language,
        secondary_target: Language,
    ) -> Result<Self, DirectionResolutionError> {
        if primary_target == secondary_target {
            return Err(DirectionResolutionError::SameLanguage(primary_target));
        }
        Ok(Self {
            primary_target,
            secondary_target,
        })
    }
}

/// Stable local-direction resolution failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectionResolutionError {
    SourceLanguageUncertain,
    SameLanguage(Language),
}

/// Resolves source and target choices without performing I/O.
pub struct LanguageDirectionResolver;

impl LanguageDirectionResolver {
    /// Resolves manual selections first; otherwise uses the configured target pair.
    ///
    /// # Errors
    /// Returns an error for uncertain automatic sources or same-language directions.
    pub fn resolve(
        preferences: LanguageDirectionPreferences,
        source_selection: LanguageSelection,
        target_selection: TargetLanguageSelection,
        detection: &LanguageDetection,
    ) -> Result<LanguageDirection, DirectionResolutionError> {
        let source = match source_selection {
            LanguageSelection::Manual(language) => language,
            LanguageSelection::Automatic => detection
                .language()
                .ok_or(DirectionResolutionError::SourceLanguageUncertain)?,
        };
        let target = match target_selection {
            TargetLanguageSelection::Manual(language) => language,
            TargetLanguageSelection::Automatic if source == preferences.primary_target => {
                preferences.secondary_target
            }
            TargetLanguageSelection::Automatic => preferences.primary_target,
        };
        LanguageDirection::new(source, target)
            .map_err(|_| DirectionResolutionError::SameLanguage(source))
    }
}
