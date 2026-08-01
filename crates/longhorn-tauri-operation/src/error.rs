use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure outside ordinary checked rejection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationHostErrorCode {
    /// The injected authority could not serve the request.
    AuthorityUnavailable,
    /// Shared authority state could not be acquired.
    AuthorityStateUnavailable,
    /// Shared executor state could not be acquired.
    ExecutorStateUnavailable,
    /// A non-durable invalidation hint could not be published.
    EventPublication,
}

/// Typed Tauri operation adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OperationHostError {
    /// Stable failure category.
    pub code: OperationHostErrorCode,
    /// Diagnostic safe at the host boundary.
    pub message: String,
    /// Whether fresh authority may make a retry succeed.
    pub retryable: bool,
}

impl OperationHostError {
    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: OperationHostErrorCode::AuthorityUnavailable,
            message: message.into(),
            retryable,
        }
    }

    pub(crate) fn authority_state_unavailable() -> Self {
        Self {
            code: OperationHostErrorCode::AuthorityStateUnavailable,
            message: "operation authority state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn executor_state_unavailable() -> Self {
        Self {
            code: OperationHostErrorCode::ExecutorStateUnavailable,
            message: "operation executor state is unavailable".into(),
            retryable: true,
        }
    }

    pub(crate) fn event_publication(message: impl Into<String>) -> Self {
        Self {
            code: OperationHostErrorCode::EventPublication,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for OperationHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for OperationHostError {}
