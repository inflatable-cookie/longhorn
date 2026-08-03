use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Hard byte ceiling for one fork-tree identity.
pub const MAXIMUM_FORK_ID_BYTES: usize = 128;

macro_rules! fork_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[cfg_attr(feature = "bindings", ts(type = "string"))]
        pub struct $name(String);

        impl $name {
            /// Validates and constructs an identity.
            pub fn new(value: impl Into<String>) -> Result<Self, ForkIdentityError> {
                let value = value.into();
                validate(&value)?;
                Ok(Self(value))
            }

            /// Returns the stable textual identity.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(de::Error::custom)
            }
        }
    };
}

fork_id!(
    ForkBranchId,
    "Injected stable identity for one branch reference."
);
fork_id!(
    ForkCheckpointId,
    "Opaque identity for one consumer-owned checkpoint reference."
);

fn validate(value: &str) -> Result<(), ForkIdentityError> {
    if value.is_empty() {
        return Err(ForkIdentityError::Empty);
    }
    if value.len() > MAXIMUM_FORK_ID_BYTES {
        return Err(ForkIdentityError::TooLong {
            maximum: MAXIMUM_FORK_ID_BYTES,
            actual: value.len(),
        });
    }
    if let Some((index, _)) = value.char_indices().find(|(_, character)| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | ':' | '-'))
    }) {
        return Err(ForkIdentityError::InvalidCharacter { index });
    }
    Ok(())
}

/// Invalid fork-tree identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkIdentityError {
    /// Identity was empty.
    Empty,
    /// Identity exceeded its hard byte limit.
    TooLong {
        /// Maximum accepted bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// Identity contained a character outside the stable grammar.
    InvalidCharacter {
        /// Invalid UTF-8 byte index.
        index: usize,
    },
}

impl fmt::Display for ForkIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("fork identity cannot be empty"),
            Self::TooLong { maximum, actual } => {
                write!(
                    formatter,
                    "fork identity is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidCharacter { index } => {
                write!(
                    formatter,
                    "fork identity has an invalid character at byte {index}"
                )
            }
        }
    }
}

impl Error for ForkIdentityError {}
