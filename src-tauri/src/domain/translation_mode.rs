use std::str::FromStr;

use crate::domain::translation::TranslationModeId;

/// Whether a translation-mode snapshot is product-owned or user-owned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationModeKind {
    BuiltIn,
    Custom,
}

/// The five product-owned translation modes with permanent identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltInTranslationMode {
    Standard,
    Literal,
    Natural,
    Academic,
    Concise,
}

impl BuiltInTranslationMode {
    pub const ALL: [Self; 5] = [
        Self::Standard,
        Self::Literal,
        Self::Natural,
        Self::Academic,
        Self::Concise,
    ];

    /// Returns the immutable, version-one snapshot for this built-in mode.
    ///
    /// # Panics
    ///
    /// Panics only if a hard-coded product UUID stops being a UUID v7.
    #[must_use]
    pub fn snapshot(self) -> TranslationModeSnapshot {
        let (id, name, instruction) = match self {
            Self::Standard => (
                "018f0a10-0000-7000-8000-000000000001",
                "Standard",
                "Translate accurately and clearly.",
            ),
            Self::Literal => (
                "018f0a10-0000-7000-8000-000000000002",
                "Literal",
                "Preserve the source wording and structure where natural.",
            ),
            Self::Natural => (
                "018f0a10-0000-7000-8000-000000000003",
                "Natural",
                "Prefer idiomatic target-language expression.",
            ),
            Self::Academic => (
                "018f0a10-0000-7000-8000-000000000004",
                "Academic",
                "Use precise formal terminology and an academic register.",
            ),
            Self::Concise => (
                "018f0a10-0000-7000-8000-000000000005",
                "Concise",
                "Translate concisely without omitting meaning.",
            ),
        };
        TranslationModeSnapshot {
            id: TranslationModeId::from_str(id).expect("built-in IDs are valid UUID v7"),
            kind: TranslationModeKind::BuiltIn,
            name: name.to_owned(),
            instruction: instruction.to_owned(),
            version: 1,
        }
    }
}

/// Immutable query-time mode data; built-ins cannot be edited in place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationModeSnapshot {
    id: TranslationModeId,
    kind: TranslationModeKind,
    name: String,
    instruction: String,
    version: u32,
}

impl TranslationModeSnapshot {
    #[must_use]
    pub const fn id(&self) -> TranslationModeId {
        self.id
    }
    #[must_use]
    pub const fn kind(&self) -> TranslationModeKind {
        self.kind
    }
    #[must_use]
    pub const fn is_immutable(&self) -> bool {
        matches!(self.kind, TranslationModeKind::BuiltIn)
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn instruction(&self) -> &str {
        &self.instruction
    }
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    #[must_use]
    pub fn copy_as_custom(&self, name: &str) -> Self {
        Self {
            id: TranslationModeId::generate(),
            kind: TranslationModeKind::Custom,
            name: name.to_owned(),
            instruction: self.instruction.clone(),
            version: 1,
        }
    }

    /// Updates a custom mode and advances its immutable snapshot version.
    ///
    /// # Errors
    ///
    /// Returns an error when callers attempt to edit a built-in mode or submit
    /// an invalid custom template.
    pub fn edit(&mut self, name: &str, instruction: &str) -> Result<(), TranslationModeEditError> {
        if self.is_immutable() {
            return Err(TranslationModeEditError::BuiltInImmutable);
        }
        if name.trim().is_empty() || instruction.trim().is_empty() {
            return Err(TranslationModeEditError::EmptyField);
        }
        if instruction
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        {
            return Err(TranslationModeEditError::IllegalControlCharacter);
        }
        if instruction.contains("{{target_language}}")
            || instruction.contains("{{schema")
            || instruction.contains("{{system")
        {
            return Err(TranslationModeEditError::ReservedPlaceholder);
        }
        name.clone_into(&mut self.name);
        instruction.clone_into(&mut self.instruction);
        self.version = self
            .version
            .checked_add(1)
            .ok_or(TranslationModeEditError::VersionOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationModeEditError {
    BuiltInImmutable,
    EmptyField,
    VersionOverflow,
    IllegalControlCharacter,
    ReservedPlaceholder,
}

#[cfg(test)]
mod tests {
    use super::{BuiltInTranslationMode, TranslationModeKind};

    #[test]
    fn built_in_modes_have_stable_ids_and_are_immutable() {
        let modes = BuiltInTranslationMode::ALL.map(BuiltInTranslationMode::snapshot);

        assert_eq!(modes.len(), 5);
        assert_eq!(
            modes[0].id().to_string(),
            "018f0a10-0000-7000-8000-000000000001"
        );
        assert_eq!(
            modes[1].id().to_string(),
            "018f0a10-0000-7000-8000-000000000002"
        );
        assert_eq!(
            modes[2].id().to_string(),
            "018f0a10-0000-7000-8000-000000000003"
        );
        assert_eq!(
            modes[3].id().to_string(),
            "018f0a10-0000-7000-8000-000000000004"
        );
        assert_eq!(
            modes[4].id().to_string(),
            "018f0a10-0000-7000-8000-000000000005"
        );
        assert!(
            modes
                .iter()
                .all(|mode| mode.kind() == TranslationModeKind::BuiltIn)
        );
        assert!(
            modes
                .iter()
                .all(super::TranslationModeSnapshot::is_immutable)
        );
    }

    #[test]
    fn copying_a_builtin_creates_an_editable_custom_mode_with_version_increments() {
        let built_in = BuiltInTranslationMode::Natural.snapshot();
        let mut custom = built_in.copy_as_custom("Natural for essays");

        assert_ne!(custom.id(), built_in.id());
        assert_eq!(custom.kind(), TranslationModeKind::Custom);
        assert!(!custom.is_immutable());
        assert_eq!(custom.version(), 1);

        custom
            .edit(
                "Natural for essays",
                "Use a polished academic but natural voice.",
            )
            .expect("valid custom mode edit");
        assert_eq!(custom.version(), 2);
    }

    #[test]
    fn custom_mode_rejects_control_characters_and_reserved_placeholders() {
        let mut custom = BuiltInTranslationMode::Standard
            .snapshot()
            .copy_as_custom("Personal");

        assert_eq!(
            custom.edit("Personal", "Do this\u{0007}"),
            Err(super::TranslationModeEditError::IllegalControlCharacter)
        );
        assert_eq!(
            custom.edit("Personal", "Translate {{target_language}}."),
            Err(super::TranslationModeEditError::ReservedPlaceholder)
        );
    }
}
