use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable host-adapter failure category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BridgeHostErrorCode {
    /// The injected authority could not supply current host facts.
    AuthorityUnavailable,
    /// The caller did not own the named active session.
    InvalidSession,
    /// The requested domain was not registered.
    UnknownDomain,
    /// The opaque domain route was not registered for this operation kind.
    UnknownRoute,
    /// The negotiated session did not advertise the route capability.
    CapabilityUnavailable,
    /// The current domain authority did not permit a read.
    ReadDenied,
    /// The current domain authority did not permit a write.
    WriteDenied,
    /// The current domain authority did not permit execution control.
    ExecutionDenied,
    /// The command or event named a superseded authority tenure.
    StaleAuthority,
    /// A provider returned facts inconsistent with the registered host.
    InvalidAuthority,
    /// A domain handler returned invalid correlation or authority metadata.
    InvalidReply,
    /// A typed domain payload could not cross the erased host seam.
    PayloadCodec,
    /// Shared adapter state could not be acquired.
    StateUnavailable,
    /// This assembly intentionally has no event channel.
    EventUnavailable,
    /// The configured event sink rejected publication.
    EventPublication,
    /// Domain or route registration was invalid or duplicated.
    InvalidRegistration,
}

/// Typed Tauri bridge adapter failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeHostError {
    /// Stable failure category.
    pub code: BridgeHostErrorCode,
    /// Diagnostic safe to expose at the host boundary.
    pub message: String,
    /// Whether retrying after fresh authority may succeed.
    pub retryable: bool,
}

impl BridgeHostError {
    /// Constructs a stable host-adapter failure.
    #[must_use]
    pub fn new(code: BridgeHostErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }

    /// Constructs an injected-authority failure.
    #[must_use]
    pub fn authority(message: impl Into<String>, retryable: bool) -> Self {
        Self::new(
            BridgeHostErrorCode::AuthorityUnavailable,
            message,
            retryable,
        )
    }

    pub(crate) fn state_unavailable() -> Self {
        Self::new(
            BridgeHostErrorCode::StateUnavailable,
            "bridge handler state is unavailable",
            true,
        )
    }

    pub(crate) fn codec(message: impl Into<String>) -> Self {
        Self::new(BridgeHostErrorCode::PayloadCodec, message, false)
    }
}

impl fmt::Display for BridgeHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for BridgeHostError {}
