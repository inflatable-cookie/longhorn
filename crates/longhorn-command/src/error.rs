use std::{error::Error, fmt};

use longhorn_core::CommandFieldId;
use serde::{Deserialize, Serialize};

/// Stable category for invalid command registry input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandRegistryErrorCode {
    /// One or more explicit limits are invalid.
    InvalidLimits,
    /// A declaration category exceeded its configured limit.
    LimitExceeded,
    /// An id was registered more than once in its category.
    DuplicateId,
    /// A declaration references an unknown id.
    MissingReference,
    /// The context graph has no single `global` root.
    InvalidContextRoot,
    /// The context graph contains a cycle.
    ContextCycle,
    /// A context path exceeds the configured depth.
    ContextDepthExceeded,
    /// Required text is empty.
    EmptyText,
    /// Bounded text exceeds its configured byte limit.
    TextTooLong,
    /// A list contains the same identity more than once.
    DuplicateItem,
    /// Visibility metadata is contradictory or empty.
    InvalidVisibility,
    /// An argument schema is invalid.
    InvalidArgumentSchema,
    /// An argument default is invalid for its field.
    InvalidArgumentDefault,
    /// Canonical registry encoding failed.
    DigestEncoding,
}

/// Invalid command declaration or registry composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRegistryError {
    code: CommandRegistryErrorCode,
    detail: String,
}

impl CommandRegistryError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> CommandRegistryErrorCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CommandRegistryError {}

pub(crate) fn registry_error(
    code: CommandRegistryErrorCode,
    detail: impl Into<String>,
) -> CommandRegistryError {
    CommandRegistryError {
        code,
        detail: detail.into(),
    }
}

/// Stable category for structurally invalid command arguments.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandArgumentErrorCode {
    /// A no-argument command received a value.
    ArgumentsNotAllowed,
    /// An object argument schema received a non-object value.
    ObjectRequired,
    /// Input contains a field absent from the schema.
    UnknownField,
    /// A required field without a default is absent.
    MissingRequiredField,
    /// A field has the wrong primitive type.
    TypeMismatch,
    /// A number or integer is outside its declared range.
    OutOfRange,
    /// A string exceeds its declared byte bound.
    StringTooLong,
    /// A string is not in the closed enum.
    UnknownEnumValue,
}

/// Structurally invalid input for one command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandArgumentError {
    code: CommandArgumentErrorCode,
    field_id: Option<CommandFieldId>,
    detail: String,
}

impl CommandArgumentError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> CommandArgumentErrorCode {
        self.code
    }

    /// Returns the affected field when the error is field-specific.
    #[must_use]
    pub fn field_id(&self) -> Option<&CommandFieldId> {
        self.field_id.as_ref()
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CommandArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CommandArgumentError {}

pub(crate) fn argument_error(
    code: CommandArgumentErrorCode,
    field_id: Option<CommandFieldId>,
    detail: impl Into<String>,
) -> CommandArgumentError {
    CommandArgumentError {
        code,
        field_id,
        detail: detail.into(),
    }
}

/// A command search query exceeds the sealed registry limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSearchError {
    maximum_bytes: usize,
    actual_bytes: usize,
}

impl CommandSearchError {
    /// Returns the configured byte limit.
    #[must_use]
    pub const fn maximum_bytes(&self) -> usize {
        self.maximum_bytes
    }

    /// Returns the supplied query size.
    #[must_use]
    pub const fn actual_bytes(&self) -> usize {
        self.actual_bytes
    }

    pub(crate) const fn new(maximum_bytes: usize, actual_bytes: usize) -> Self {
        Self {
            maximum_bytes,
            actual_bytes,
        }
    }
}

impl fmt::Display for CommandSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "command search query contains {} bytes; maximum is {}",
            self.actual_bytes, self.maximum_bytes
        )
    }
}

impl Error for CommandSearchError {}
