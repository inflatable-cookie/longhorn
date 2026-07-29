use std::{error::Error, fmt};

/// Stable category for rejected Surface resolution input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceResolutionErrorCode {
    /// The durable document failed Surface validation.
    InvalidDocument,
    /// Current admitted Surface count exceeded configured limits.
    TooManyAdmittedSurfaces,
    /// Current available-window count exceeded configured limits.
    TooManyAvailableWindows,
    /// One admitted Surface id appeared more than once.
    DuplicateAdmittedSurface,
    /// One available window id appeared more than once.
    DuplicateAvailableWindow,
    /// Presence input named a Surface outside the document.
    UnknownAdmittedSurface,
    /// Availability input named a non-participating window.
    UnknownAvailableWindow,
}

/// Rejected current Surface resolution input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceResolutionError {
    code: SurfaceResolutionErrorCode,
    detail: String,
}

impl SurfaceResolutionError {
    /// Returns the stable rejection category.
    #[must_use]
    pub const fn code(&self) -> SurfaceResolutionErrorCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SurfaceResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceResolutionError {}

pub(super) fn resolution_error(
    code: SurfaceResolutionErrorCode,
    detail: impl Into<String>,
) -> SurfaceResolutionError {
    SurfaceResolutionError {
        code,
        detail: detail.into(),
    }
}
