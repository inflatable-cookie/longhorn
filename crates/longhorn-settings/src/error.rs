use std::{error::Error, fmt};

/// Stable category for invalid settings registry input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsRegistryErrorCode {
    /// One or more explicit limits are invalid.
    InvalidLimits,
    /// A declaration category exceeded its configured limit.
    LimitExceeded,
    /// An id was registered more than once in its category.
    DuplicateId,
    /// An anchor id appeared on more than one page.
    DuplicateAnchor,
    /// A declaration references an unknown id.
    MissingReference,
    /// A declaration crosses an ownership boundary it may not cross.
    OwnershipMismatch,
    /// A label or keyword is empty.
    EmptyText,
    /// A label or keyword exceeds its configured byte limit.
    TextTooLong,
    /// The composed capability set contains an unregistered capability.
    UnknownComposedCapability,
    /// Canonical registry encoding failed.
    DigestEncoding,
}

/// Invalid settings declaration or registry composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsRegistryError {
    code: SettingsRegistryErrorCode,
    detail: String,
}

impl SettingsRegistryError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn code(&self) -> SettingsRegistryErrorCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SettingsRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SettingsRegistryError {}

pub(crate) fn registry_error(
    code: SettingsRegistryErrorCode,
    detail: impl Into<String>,
) -> SettingsRegistryError {
    SettingsRegistryError {
        code,
        detail: detail.into(),
    }
}
