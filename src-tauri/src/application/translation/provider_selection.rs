/// Selects the explicitly configured primary provider before a request is built.
///
/// # Errors
///
/// Returns [`PrimaryProviderSelectionError::PrimaryMissing`] without any provider side effect.
pub fn select_primary_provider(
    primary_provider_id: Option<&str>,
) -> Result<&str, PrimaryProviderSelectionError> {
    primary_provider_id.ok_or(PrimaryProviderSelectionError::PrimaryMissing)
}

/// A safe, stable preflight failure for primary-provider selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryProviderSelectionError {
    PrimaryMissing,
}

impl PrimaryProviderSelectionError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PrimaryMissing => "translation.provider.primary_missing",
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unconfigured_workspace_returns_primary_missing_before_a_provider_can_be_requested() {
        assert_eq!(
            super::select_primary_provider(None),
            Err(super::PrimaryProviderSelectionError::PrimaryMissing)
        );
        assert_eq!(
            super::PrimaryProviderSelectionError::PrimaryMissing.code(),
            "translation.provider.primary_missing"
        );
    }
}
