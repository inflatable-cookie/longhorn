use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const MAX_DOMAIN_ID_BYTES: usize = 128;

/// Stable, namespaced identifier for a registered Longhorn domain.
///
/// Identifiers contain lowercase ASCII segments separated by dots. Each
/// segment starts with a letter and may continue with letters, digits, `_`,
/// or `-`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct DomainId(String);

impl DomainId {
    /// Validates and constructs a domain identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainIdError> {
        let value = value.into();

        if value.is_empty() {
            return Err(DomainIdError::Empty);
        }
        if value.len() > MAX_DOMAIN_ID_BYTES {
            return Err(DomainIdError::TooLong {
                maximum: MAX_DOMAIN_ID_BYTES,
                actual: value.len(),
            });
        }

        for (index, segment) in value.split('.').enumerate() {
            let mut characters = segment.chars();
            let valid_start = characters
                .next()
                .is_some_and(|character| character.is_ascii_lowercase());
            let valid_rest = characters.all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '_' | '-')
            });

            if !valid_start || !valid_rest {
                return Err(DomainIdError::InvalidSegment { index });
            }
        }

        Ok(Self(value))
    }

    /// Returns the serialized identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DomainId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DomainId {
    type Err = DomainIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for DomainId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DomainId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Domain identifier validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded the bounded serialized length.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// A segment did not match the required grammar.
    InvalidSegment {
        /// Zero-based segment index.
        index: usize,
    },
}

impl fmt::Display for DomainIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("domain id cannot be empty"),
            Self::TooLong { maximum, actual } => {
                write!(
                    formatter,
                    "domain id is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidSegment { index } => {
                write!(formatter, "domain id segment {index} is invalid")
            }
        }
    }
}

impl Error for DomainIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_namespaced_lowercase_ids() {
        let identifier = DomainId::new("example.window_state-v2").unwrap();

        assert_eq!(identifier.as_str(), "example.window_state-v2");
    }

    #[test]
    fn rejects_empty_or_invalid_segments() {
        assert_eq!(DomainId::new(""), Err(DomainIdError::Empty));
        assert_eq!(
            DomainId::new(format!("a{}", "b".repeat(MAX_DOMAIN_ID_BYTES))),
            Err(DomainIdError::TooLong {
                maximum: MAX_DOMAIN_ID_BYTES,
                actual: MAX_DOMAIN_ID_BYTES + 1,
            })
        );
        assert_eq!(
            DomainId::new("Example.settings"),
            Err(DomainIdError::InvalidSegment { index: 0 })
        );
        assert_eq!(
            DomainId::new("example..settings"),
            Err(DomainIdError::InvalidSegment { index: 1 })
        );
    }

    #[test]
    fn serde_round_trip_uses_a_string() {
        let identifier = DomainId::new("example.settings").unwrap();
        let json = serde_json::to_string(&identifier).unwrap();

        assert_eq!(json, "\"example.settings\"");
        assert_eq!(serde_json::from_str::<DomainId>(&json).unwrap(), identifier);
    }
}
