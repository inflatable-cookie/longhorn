use std::{error::Error, fmt};

const MAX_OBSERVATION_ID_BYTES: usize = 128;
const MAX_EVIDENCE_NAMESPACE_BYTES: usize = 64;
const MAX_EVIDENCE_VALUE_BYTES: usize = 512;
const MAX_LABEL_BYTES: usize = 256;

pub(crate) fn validate_observation_id(value: &str) -> Result<(), DisplayTextError> {
    validate_nonempty("observation id", value)?;
    validate_length("observation id", value, MAX_OBSERVATION_ID_BYTES)?;
    if let Some((index, _)) = value.char_indices().find(|(_, character)| {
        !(character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-'))
    }) {
        return Err(DisplayTextError::InvalidCharacter {
            field: "observation id",
            index,
        });
    }
    Ok(())
}

pub(crate) fn validate_evidence_namespace(value: &str) -> Result<(), DisplayTextError> {
    validate_nonempty("evidence namespace", value)?;
    validate_length("evidence namespace", value, MAX_EVIDENCE_NAMESPACE_BYTES)?;

    let mut characters = value.char_indices();
    if !characters
        .next()
        .is_some_and(|(_, character)| character.is_ascii_lowercase())
    {
        return Err(DisplayTextError::InvalidCharacter {
            field: "evidence namespace",
            index: 0,
        });
    }
    if let Some((index, _)) = characters.find(|(_, character)| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-'))
    }) {
        return Err(DisplayTextError::InvalidCharacter {
            field: "evidence namespace",
            index,
        });
    }
    Ok(())
}

pub(crate) fn validate_evidence_value(value: &str) -> Result<(), DisplayTextError> {
    validate_nonempty("evidence value", value)?;
    validate_length("evidence value", value, MAX_EVIDENCE_VALUE_BYTES)?;
    if let Some((index, _)) = value
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(DisplayTextError::InvalidCharacter {
            field: "evidence value",
            index,
        });
    }
    Ok(())
}

pub(crate) fn validate_label(value: &str) -> Result<(), DisplayTextError> {
    if value.trim().is_empty() {
        return Err(DisplayTextError::Empty {
            field: "display label",
        });
    }
    validate_length("display label", value, MAX_LABEL_BYTES)?;
    if let Some((index, _)) = value
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(DisplayTextError::InvalidCharacter {
            field: "display label",
            index,
        });
    }
    Ok(())
}

fn validate_nonempty(field: &'static str, value: &str) -> Result<(), DisplayTextError> {
    if value.is_empty() {
        Err(DisplayTextError::Empty { field })
    } else {
        Ok(())
    }
}

fn validate_length(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), DisplayTextError> {
    if value.len() > maximum {
        Err(DisplayTextError::TooLong {
            field,
            maximum,
            actual: value.len(),
        })
    } else {
        Ok(())
    }
}

/// Validation failure for display observation text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayTextError {
    /// A required value was empty.
    Empty {
        /// Field being validated.
        field: &'static str,
    },
    /// A value exceeded its serialized byte limit.
    TooLong {
        /// Field being validated.
        field: &'static str,
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// A value contained a disallowed character.
    InvalidCharacter {
        /// Field being validated.
        field: &'static str,
        /// Byte index of the invalid character.
        index: usize,
    },
}

impl fmt::Display for DisplayTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} cannot be empty"),
            Self::TooLong {
                field,
                maximum,
                actual,
            } => write!(formatter, "{field} is {actual} bytes; maximum is {maximum}"),
            Self::InvalidCharacter { field, index } => {
                write!(
                    formatter,
                    "{field} has an invalid character at byte {index}"
                )
            }
        }
    }
}

impl Error for DisplayTextError {}
