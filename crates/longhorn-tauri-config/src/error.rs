use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

// Per-adapter fork, recorded as policy (Card 222): every Tauri adapter owns
// its `{code, message, retryable}` scaffold. The codes are wire-stable -- serde
// camelCase to TypeScript -- and adapters with two mutable states name them
// distinctly (`AuthorityStateUnavailable` vs `ExecutorStateUnavailable`), so the
// drifting vocabulary is semantic, not accidental. A shared crate would save six
// lines and churn wire codes a consumer may match on.
/// Stable operational failure outside normal config-operation outcomes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigOperationsHostErrorCode {
    /// Injected authority could not serve the command.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// Host-owned selection or encryption interaction failed.
    HostInteraction,
}

/// Typed Tauri adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigOperationsHostError {
    /// Stable failure category.
    pub code: ConfigOperationsHostErrorCode,
    /// Redacted diagnostic safe at the renderer boundary.
    pub message: String,
    /// Whether fresh host authority may make retry useful.
    pub retryable: bool,
}

impl ConfigOperationsHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: ConfigOperationsHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    /// Constructs a host interaction failure without secret detail.
    #[must_use]
    pub fn interaction(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: ConfigOperationsHostErrorCode::HostInteraction,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: ConfigOperationsHostErrorCode::StateUnavailable,
            message: "config operations handler state is unavailable".into(),
            retryable: true,
        }
    }
}

impl fmt::Display for ConfigOperationsHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ConfigOperationsHostError {}
