use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure category outside settings domain outcomes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsHostErrorCode {
    /// The injected authority could not serve the command.
    AuthorityUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// A revision-hint event could not be published.
    EventPublication,
}

/// Typed Tauri adapter failure. Domain conflicts and rejections stay in their
/// normal settings outcomes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SettingsHostError {
    /// Stable failure category.
    pub code: SettingsHostErrorCode,
    /// Diagnostic safe to expose at the host boundary.
    pub message: String,
    /// Whether retrying after fresh authority may succeed.
    pub retryable: bool,
}

impl SettingsHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: SettingsHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: SettingsHostErrorCode::StateUnavailable,
            message: "settings handler state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: SettingsHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for SettingsHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SettingsHostError {}
