use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

macro_rules! uuid_v7_identifier {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = IdentifierParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let identifier =
                    Uuid::parse_str(value).map_err(|_| IdentifierParseError::InvalidUuid)?;
                if identifier.get_version_num() != 7 {
                    return Err(IdentifierParseError::WrongVersion);
                }

                Ok(Self(identifier))
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::from_str(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

uuid_v7_identifier!(TranslationSessionId);
uuid_v7_identifier!(TranslationAttemptId);
uuid_v7_identifier!(ProviderProfileId);
uuid_v7_identifier!(TranslationModeId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifierParseError {
    InvalidUuid,
    WrongVersion,
}

impl fmt::Display for IdentifierParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuid => formatter.write_str("identifier is not a UUID"),
            Self::WrongVersion => formatter.write_str("identifier must be a UUID v7"),
        }
    }
}

impl Error for IdentifierParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResultVersion(u32);

impl ResultVersion {
    /// Creates a positive result version.
    ///
    /// # Errors
    ///
    /// Returns [`ResultVersionError::Zero`] when `value` is zero.
    pub const fn new(value: u32) -> Result<Self, ResultVersionError> {
        if value == 0 {
            return Err(ResultVersionError::Zero);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn initial() -> Self {
        Self(1)
    }

    /// Returns the next result version.
    ///
    /// # Errors
    ///
    /// Returns [`ResultVersionError::Overflow`] when this is the largest
    /// representable version.
    pub const fn next(self) -> Result<Self, ResultVersionError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(ResultVersionError::Overflow),
        }
    }

    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl Serialize for ResultVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for ResultVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u32::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultVersionError {
    Zero,
    Overflow,
}

impl fmt::Display for ResultVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => formatter.write_str("result version must be positive"),
            Self::Overflow => formatter.write_str("result version cannot exceed u32::MAX"),
        }
    }
}

impl Error for ResultVersionError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{
        ProviderProfileId, ResultVersion, TranslationAttemptId, TranslationModeId,
        TranslationSessionId,
    };

    #[test]
    fn generates_uuid_v7_stable_identifiers() {
        let session_id = TranslationSessionId::generate();
        let attempt_id = TranslationAttemptId::generate();
        let profile_id = ProviderProfileId::generate();
        let mode_id = TranslationModeId::generate();

        for identifier in [
            session_id.to_string(),
            attempt_id.to_string(),
            profile_id.to_string(),
            mode_id.to_string(),
        ] {
            let parsed = uuid::Uuid::parse_str(&identifier).expect("identifier must be a UUID");
            assert_eq!(parsed.get_version_num(), 7);
        }
    }

    #[test]
    fn parses_only_uuid_v7_identifiers() {
        let id = TranslationSessionId::generate();
        assert_eq!(
            TranslationSessionId::from_str(&id.to_string()).expect("v7 must parse"),
            id
        );
        assert!(TranslationSessionId::from_str("550e8400-e29b-41d4-a716-446655440000").is_err());
        assert!(TranslationSessionId::from_str("not-an-id").is_err());
    }

    #[test]
    fn serializes_identifiers_as_canonical_uuid_strings() {
        let id = TranslationAttemptId::generate();
        let serialized = serde_json::to_string(&id).expect("identifier must serialize");

        assert_eq!(serialized, format!(r#""{id}""#));
        assert_eq!(
            serde_json::from_str::<TranslationAttemptId>(&serialized)
                .expect("identifier must deserialize"),
            id
        );
        assert!(
            serde_json::from_str::<TranslationAttemptId>(
                r#""550e8400-e29b-41d4-a716-446655440000""#
            )
            .is_err()
        );
    }

    #[test]
    fn result_version_is_positive_and_monotonic() {
        assert!(ResultVersion::new(0).is_err());

        let initial = ResultVersion::initial();
        assert_eq!(initial.value(), 1);
        assert_eq!(
            initial.next().expect("version one has a successor").value(),
            2
        );
        assert!(serde_json::from_str::<ResultVersion>("0").is_err());
    }
}
