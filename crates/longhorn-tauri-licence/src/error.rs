use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure outside ordinary command rejection.
///
/// Distinct from `LicenceRejectionCode`, which says why the *authority*
/// refused. This says the request never reached it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LicenceHostErrorCode {
    /// Injected consumer authority could not serve the request.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// A non-durable invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri licence adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceHostError {
    /// Stable failure category.
    pub code: LicenceHostErrorCode,
    /// Host-safe diagnostic. Never credential material: the authority's own
    /// errors carry detail strings, and nothing in this crate adds a secret
    /// to one.
    pub message: String,
    /// Whether fresh authority may make retry succeed.
    pub retryable: bool,
}

impl LicenceHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: LicenceHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: LicenceHostErrorCode::StateUnavailable,
            message: "licence handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: LicenceHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for LicenceHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for LicenceHostError {}
