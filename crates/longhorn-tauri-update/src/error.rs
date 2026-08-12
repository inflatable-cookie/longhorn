use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure outside ordinary command rejection.
///
/// Distinct from `UpdateRejectionCode`, which says why the *controller*
/// refused. This says the request never reached it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateHostErrorCode {
    /// Injected consumer authority could not serve the request.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// A non-durable invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri update adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateHostError {
    /// Stable failure category.
    pub code: UpdateHostErrorCode,
    /// Host-safe diagnostic.
    pub message: String,
    /// Whether fresh authority may make retry succeed.
    pub retryable: bool,
}

impl UpdateHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: UpdateHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: UpdateHostErrorCode::StateUnavailable,
            message: "update handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: UpdateHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for UpdateHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for UpdateHostError {}
