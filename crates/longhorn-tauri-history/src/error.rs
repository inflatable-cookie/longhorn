use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

// Per-adapter fork, recorded as policy (Card 222): every Tauri adapter owns
// its `{code, message, retryable}` scaffold. The codes are wire-stable -- serde
// camelCase to TypeScript -- and adapters with two mutable states name them
// distinctly (`AuthorityStateUnavailable` vs `ExecutorStateUnavailable`), so the
// drifting vocabulary is semantic, not accidental. A shared crate would save six
// lines and churn wire codes a consumer may match on.
/// Stable operational failure outside ordinary navigation rejection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryHostErrorCode {
    /// The injected consumer authority could not serve the request.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// A non-durable history hint could not be published.
    EventPublication,
}

/// Typed Tauri history adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoryHostError {
    /// Stable failure category.
    pub code: HistoryHostErrorCode,
    /// Diagnostic safe at the host boundary.
    pub message: String,
    /// Whether fresh authority may make a retry succeed.
    pub retryable: bool,
}

impl HistoryHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: HistoryHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: HistoryHostErrorCode::StateUnavailable,
            message: "history handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: HistoryHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for HistoryHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HistoryHostError {}
