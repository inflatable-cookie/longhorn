use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure outside normal keymap outcomes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandHostErrorCode {
    /// The injected authority could not serve the request.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// An invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandHostError {
    /// Stable failure category.
    pub code: CommandHostErrorCode,
    /// Diagnostic safe at the host boundary.
    pub message: String,
    /// Whether fresh authority may make a retry succeed.
    pub retryable: bool,
}

impl CommandHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: CommandHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: CommandHostErrorCode::StateUnavailable,
            message: "command handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: CommandHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for CommandHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CommandHostError {}
