use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure outside ordinary graph-navigation rejection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ForkHistoryHostErrorCode {
    /// Injected consumer authority could not serve the request.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// A non-durable invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri fork-history adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ForkHistoryHostError {
    /// Stable failure category.
    pub code: ForkHistoryHostErrorCode,
    /// Host-safe diagnostic.
    pub message: String,
    /// Whether fresh authority may make retry succeed.
    pub retryable: bool,
}

impl ForkHistoryHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: ForkHistoryHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: ForkHistoryHostErrorCode::StateUnavailable,
            message: "fork-history handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: ForkHistoryHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for ForkHistoryHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ForkHistoryHostError {}
