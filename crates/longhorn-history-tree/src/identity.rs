use std::{error::Error, fmt};

/// Hard byte ceiling for one fork-tree identity.
pub const MAXIMUM_FORK_ID_BYTES: usize = 128;

/// Injected stable identity for one branch reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ForkBranchId(String);

impl ForkBranchId {
    /// Validates and constructs a branch identity.
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

impl fmt::Display for ForkBranchId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

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
