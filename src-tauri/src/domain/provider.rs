use std::time::Duration;

use serde_json::Value;
use url::{Host, Url};

use crate::domain::translation::ProviderProfileId;

/// The fixed protocol families supported by Evolish provider profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
    Gemini,
    OpenAiCompatible,
}

/// A custom provider endpoint that has passed Evolish's network safety policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderEndpoint(Url);

impl ProviderEndpoint {
    /// Parses and validates a provider endpoint before it is stored or requested.
    ///
    /// # Errors
    ///
    /// Returns a typed validation error for malformed, unsafe, or unsupported URLs.
    pub fn parse(value: &str) -> Result<Self, ProviderProfileError> {
        let endpoint = Url::parse(value).map_err(|_| ProviderProfileError::InvalidEndpoint)?;
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err(ProviderProfileError::EndpointContainsUserInfo);
        }
        if endpoint.fragment().is_some() {
            return Err(ProviderProfileError::EndpointContainsFragment);
        }
        match endpoint.scheme() {
            "https" => Ok(Self(endpoint)),
            "http" if is_loopback_host(endpoint.host().as_ref()) => Ok(Self(endpoint)),
            "http" => Err(ProviderProfileError::InsecureRemoteEndpoint),
            _ => Err(ProviderProfileError::UnsupportedEndpointScheme),
        }
    }

    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.0
    }
}

fn is_loopback_host(host: Option<&Host<&str>>) -> bool {
    match host {
        Some(Host::Domain("localhost")) => true,
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(_)) | None => false,
    }
}

/// Non-secret request configuration for a provider profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderParameters {
    timeout: Duration,
    options: Value,
}

impl ProviderParameters {
    /// Creates validated, non-secret request parameters.
    ///
    /// # Errors
    ///
    /// Returns an error when the timeout is zero or options are not an object.
    pub fn new(timeout: Duration, options: Value) -> Result<Self, ProviderProfileError> {
        if timeout.is_zero() {
            return Err(ProviderProfileError::InvalidTimeout);
        }
        if !options.is_object() {
            return Err(ProviderProfileError::ParametersMustBeObject);
        }
        if contains_secret_key(&options) {
            return Err(ProviderProfileError::SecretInParameters);
        }
        Ok(Self { timeout, options })
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[must_use]
    pub const fn options(&self) -> &Value {
        &self.options
    }
}

fn contains_secret_key(value: &Value) -> bool {
    match value {
        Value::Object(entries) => entries.iter().any(|(key, value)| {
            let normalized = key.to_ascii_lowercase();
            matches!(
                normalized.as_str(),
                "api_key" | "authorization" | "secret" | "token"
            ) || contains_secret_key(value)
        }),
        Value::Array(entries) => entries.iter().any(contains_secret_key),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => false,
    }
}

impl Default for ProviderParameters {
    fn default() -> Self {
        Self {
            timeout: Duration::from_mins(2),
            options: Value::Object(serde_json::Map::new()),
        }
    }
}

/// One user-configured instance of a compiled-in provider protocol.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderProfile {
    id: ProviderProfileId,
    protocol: ProviderProtocol,
    display_name: String,
    endpoint: Option<ProviderEndpoint>,
    model: String,
    parameters: ProviderParameters,
    secret_ref: Option<String>,
    enabled: bool,
    sort_order: u32,
}

impl ProviderProfile {
    /// Creates a validated provider profile with only a non-secret credential reference.
    ///
    /// # Errors
    ///
    /// Returns a typed validation error for invalid display data, endpoint, or parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ProviderProfileId,
        protocol: ProviderProtocol,
        display_name: &str,
        endpoint: Option<&str>,
        model: &str,
        parameters: ProviderParameters,
        secret_ref: Option<&str>,
        enabled: bool,
        sort_order: u32,
    ) -> Result<Self, ProviderProfileError> {
        if display_name.trim().is_empty() {
            return Err(ProviderProfileError::EmptyDisplayName);
        }
        if model.trim().is_empty() {
            return Err(ProviderProfileError::EmptyModel);
        }
        let endpoint = endpoint.map(ProviderEndpoint::parse).transpose()?;
        let secret_ref = secret_ref.map(str::to_owned);
        Ok(Self {
            id,
            protocol,
            display_name: display_name.to_owned(),
            endpoint,
            model: model.to_owned(),
            parameters,
            secret_ref,
            enabled,
            sort_order,
        })
    }

    #[must_use]
    pub const fn id(&self) -> ProviderProfileId {
        self.id
    }

    #[must_use]
    pub const fn protocol(&self) -> ProviderProtocol {
        self.protocol
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn endpoint(&self) -> Option<&ProviderEndpoint> {
        self.endpoint.as_ref()
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    #[must_use]
    pub const fn parameters(&self) -> &ProviderParameters {
        &self.parameters
    }

    #[must_use]
    pub fn secret_ref(&self) -> Option<&str> {
        self.secret_ref.as_deref()
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn sort_order(&self) -> u32 {
        self.sort_order
    }
}

/// Validation failures that are safe to expose without revealing secrets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProfileError {
    EmptyDisplayName,
    EmptyModel,
    InvalidEndpoint,
    EndpointContainsUserInfo,
    EndpointContainsFragment,
    InsecureRemoteEndpoint,
    UnsupportedEndpointScheme,
    InvalidTimeout,
    ParametersMustBeObject,
    SecretInParameters,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::{
        ProviderEndpoint, ProviderParameters, ProviderProfile, ProviderProfileError,
        ProviderProtocol,
    };
    use crate::domain::translation::ProviderProfileId;

    #[test]
    fn accepts_multiple_instances_of_the_same_provider_protocol() {
        let first = ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAiCompatible,
            "Personal OpenAI-compatible service",
            Some("https://one.example/v1"),
            "model-one",
            ProviderParameters::default(),
            Some("credential-one"),
            true,
            0,
        )
        .expect("first instance");
        let second = ProviderProfile::new(
            ProviderProfileId::generate(),
            ProviderProtocol::OpenAiCompatible,
            "Work OpenAI-compatible service",
            Some("https://two.example/v1"),
            "model-two",
            ProviderParameters::default(),
            Some("credential-two"),
            true,
            1,
        )
        .expect("second instance");

        assert_ne!(first.id(), second.id());
        assert_eq!(first.protocol(), second.protocol());
    }

    #[test]
    fn endpoint_rejects_credentials_fragments_and_non_loopback_http() {
        assert_eq!(
            ProviderEndpoint::parse("https://secret@example.com/v1"),
            Err(ProviderProfileError::EndpointContainsUserInfo)
        );
        assert_eq!(
            ProviderEndpoint::parse("https://api.example.com/v1#ignored"),
            Err(ProviderProfileError::EndpointContainsFragment)
        );
        assert_eq!(
            ProviderEndpoint::parse("http://api.example.com/v1"),
            Err(ProviderProfileError::InsecureRemoteEndpoint)
        );
        assert!(ProviderEndpoint::parse("http://localhost:11434/v1").is_ok());
        assert!(ProviderEndpoint::parse("http://127.0.0.1:8080/v1").is_ok());
        assert!(ProviderEndpoint::parse("http://[::1]:8080/v1").is_ok());
    }

    #[test]
    fn parameters_default_to_120_seconds_and_accept_safe_json_options() {
        let defaults = ProviderParameters::default();
        assert_eq!(defaults.timeout(), Duration::from_mins(2));

        let parameters = ProviderParameters::new(
            Duration::from_secs(45),
            json!({"temperature": 0.2, "reasoning_effort": "low"}),
        )
        .expect("safe parameters");
        assert_eq!(parameters.timeout(), Duration::from_secs(45));
        assert_eq!(
            parameters.options(),
            &json!({"temperature": 0.2, "reasoning_effort": "low"})
        );
    }

    #[test]
    fn parameters_reject_secret_material_instead_of_persisting_it_with_a_profile() {
        assert_eq!(
            ProviderParameters::new(Duration::from_mins(2), json!({"api_key": "canary"})),
            Err(ProviderProfileError::SecretInParameters)
        );
    }
}
