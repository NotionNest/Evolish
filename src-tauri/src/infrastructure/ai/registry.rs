use crate::domain::provider::{ProviderEndpoint, ProviderProfile, ProviderProtocol};

/// Product-owned capabilities for one compiled-in provider protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderDescriptor {
    protocol: ProviderProtocol,
    supports_custom_endpoint: bool,
}

impl ProviderDescriptor {
    #[must_use]
    pub const fn protocol(self) -> ProviderProtocol {
        self.protocol
    }

    #[must_use]
    pub const fn supports_custom_endpoint(self) -> bool {
        self.supports_custom_endpoint
    }
}

const DESCRIPTORS: [ProviderDescriptor; 4] = [
    ProviderDescriptor {
        protocol: ProviderProtocol::OpenAi,
        supports_custom_endpoint: false,
    },
    ProviderDescriptor {
        protocol: ProviderProtocol::Anthropic,
        supports_custom_endpoint: false,
    },
    ProviderDescriptor {
        protocol: ProviderProtocol::Gemini,
        supports_custom_endpoint: false,
    },
    ProviderDescriptor {
        protocol: ProviderProtocol::OpenAiCompatible,
        supports_custom_endpoint: true,
    },
];

/// Registry containing only Evolish's compiled-in provider protocols.
pub struct AiProviderRegistry;

impl AiProviderRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    pub const fn descriptors(&self) -> &[ProviderDescriptor] {
        &DESCRIPTORS
    }

    /// Resolves a profile only when it is enabled and its endpoint remains safe.
    ///
    /// # Errors
    ///
    /// Returns a typed error before any HTTP request can be constructed.
    pub fn resolve(
        &self,
        profile: &ProviderProfile,
    ) -> Result<&'static ProviderDescriptor, ProviderRegistryError> {
        if !profile.enabled() {
            return Err(ProviderRegistryError::ProfileDisabled);
        }
        if let Some(endpoint) = profile.endpoint() {
            ProviderEndpoint::parse(endpoint.url().as_str())
                .map_err(|_| ProviderRegistryError::InvalidEndpoint)?;
        }
        DESCRIPTORS
            .iter()
            .find(|descriptor| descriptor.protocol == profile.protocol())
            .ok_or(ProviderRegistryError::UnsupportedProtocol)
    }
}

impl Default for AiProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRegistryError {
    ProfileDisabled,
    InvalidEndpoint,
    UnsupportedProtocol,
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        provider::{ProviderParameters, ProviderProfile, ProviderProtocol},
        translation::ProviderProfileId,
    };

    #[test]
    fn registry_exposes_exactly_the_compiled_in_protocols_and_resolves_enabled_profiles() {
        let registry = super::AiProviderRegistry::new();
        assert_eq!(registry.descriptors().len(), 4);
        let profile = ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAiCompatible,
            "Local service",
            Some("http://localhost:11434/v1"),
            "local-model",
            ProviderParameters::default(),
            Some("credential-reference"),
            true,
            0,
        )
        .expect("profile");

        let descriptor = registry.resolve(&profile).expect("registered profile");
        assert_eq!(descriptor.protocol(), ProviderProtocol::OpenAiCompatible);
        assert!(descriptor.supports_custom_endpoint());
    }

    #[test]
    fn registry_rejects_disabled_profiles_before_any_http_request() {
        let profile = ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAi,
            "Disabled",
            None,
            "gpt-test",
            ProviderParameters::default(),
            Some("credential-reference"),
            false,
            0,
        )
        .expect("profile");

        assert_eq!(
            super::AiProviderRegistry::new().resolve(&profile),
            Err(super::ProviderRegistryError::ProfileDisabled)
        );
    }
}
