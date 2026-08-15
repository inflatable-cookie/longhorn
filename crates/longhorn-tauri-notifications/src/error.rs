use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

// Per-adapter fork, recorded as policy (Card 222): every Tauri adapter owns
// its `{code, message, retryable}` scaffold. The codes are wire-stable -- serde
// camelCase to TypeScript -- and adapters with two mutable states name them
// distinctly (`AuthorityStateUnavailable` vs `ExecutorStateUnavailable`), so the
// drifting vocabulary is semantic, not accidental. A shared crate would save six
// lines and churn wire codes a consumer may match on.
/// Stable host failure outside ordinary checked ledger rejection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationHostErrorCode {
    /// The injected authority could not serve the request.
    AuthorityUnavailable,
    /// Shared authority state could not be acquired.
    AuthorityStateUnavailable,
    /// A non-durable invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri notification adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NotificationHostError {
    /// Stable failure category.
    pub code: NotificationHostErrorCode,
    /// Diagnostic safe at the host boundary.
    pub message: String,
    /// Whether fresh authority may make a retry succeed.
    pub retryable: bool,
}

impl NotificationHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: NotificationHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn authority_state_unavailable() -> Self {
        Self {
            code: NotificationHostErrorCode::AuthorityStateUnavailable,
            message: "notification authority state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: NotificationHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for NotificationHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl Error for NotificationHostError {}
