use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable operational failure category crossing the Tauri transfer wire edge.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferHostErrorCode {
    /// Current managed-window readback failed.
    RuntimeUnavailable,
    /// Shared handler state could not be acquired.
    StateUnavailable,
    /// The transfer handler was already torn down.
    Inactive,
    /// Process-local client identity space was exhausted.
    IdentityExhausted,
    /// Host-issued client identity construction failed.
    InvalidIssuedClientId,
    /// Binding fresh host-issued client authority failed.
    ClientBinding,
    /// The invoking Tauri window label was not a valid host window handle.
    InvalidCallerHandle,
}

/// Typed Tauri transfer adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TransferHostError {
    /// Stable failure category.
    pub code: TransferHostErrorCode,
    /// Diagnostic safe at the host boundary.
    pub message: String,
    /// Whether retrying after fresh host state may succeed.
    pub retryable: bool,
}

impl TransferHostError {
    pub(crate) fn state_unavailable() -> Self {
        Self {
            code: TransferHostErrorCode::StateUnavailable,
            message: "transfer handler state is unavailable".into(),
            retryable: false,
        }
    }

    pub(crate) fn invalid_caller(message: impl Into<String>) -> Self {
        Self {
            code: TransferHostErrorCode::InvalidCallerHandle,
            message: message.into(),
            retryable: false,
        }
    }
}

impl fmt::Display for TransferHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for TransferHostError {}
