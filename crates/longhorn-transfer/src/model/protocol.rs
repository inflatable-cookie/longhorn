use longhorn_core::ScreenPoint;
use serde::{Deserialize, Serialize};

use crate::{
    DragSessionId, ResolvedTransferTarget, TransferDuration, TransferInstant,
    TransferSourceAuthority,
};

/// Current renderer payload protocol version.
pub const TRANSFER_PROTOCOL_VERSION: u32 = 1;

/// Minimal renderer-visible transfer payload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferPayload {
    protocol_version: u32,
    session_id: DragSessionId,
}

impl TransferPayload {
    pub(crate) const fn new(session_id: DragSessionId) -> Self {
        Self {
            protocol_version: TRANSFER_PROTOCOL_VERSION,
            session_id,
        }
    }

    /// Returns the exact protocol version.
    #[must_use]
    pub const fn protocol_version(self) -> u32 {
        self.protocol_version
    }

    /// Returns the process-local session id.
    #[must_use]
    pub const fn session_id(self) -> DragSessionId {
        self.session_id
    }
}

/// Host-resolved request to create one bounded transfer session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferSessionRequest {
    source: TransferSourceAuthority,
    lifetime: TransferDuration,
}

impl TransferSessionRequest {
    /// Constructs a request from current host-resolved authority.
    #[must_use]
    pub const fn new(source: TransferSourceAuthority, lifetime: TransferDuration) -> Self {
        Self { source, lifetime }
    }

    pub(crate) const fn source(&self) -> &TransferSourceAuthority {
        &self.source
    }

    pub(crate) const fn lifetime(&self) -> TransferDuration {
        self.lifetime
    }
}

/// Successful process-local session allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionCreationReceipt {
    payload: TransferPayload,
    expires_at: TransferInstant,
}

impl SessionCreationReceipt {
    pub(crate) const fn new(payload: TransferPayload, expires_at: TransferInstant) -> Self {
        Self {
            payload,
            expires_at,
        }
    }

    /// Returns the only renderer-visible session payload.
    #[must_use]
    pub const fn payload(self) -> TransferPayload {
        self.payload
    }

    /// Returns the process-local expiry instant.
    #[must_use]
    pub const fn expires_at(self) -> TransferInstant {
        self.expires_at
    }
}

/// Idempotent cancellation result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SessionCancellationStatus {
    /// The active session became cancelled.
    Cancelled,
    /// The session was already cancelled.
    AlreadyCancelled,
}

/// Successful cancellation evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SessionCancellationReceipt {
    session_id: DragSessionId,
    status: SessionCancellationStatus,
}

impl SessionCancellationReceipt {
    pub(crate) const fn new(session_id: DragSessionId, status: SessionCancellationStatus) -> Self {
        Self { session_id, status }
    }

    /// Returns the named session.
    #[must_use]
    pub const fn session_id(self) -> DragSessionId {
        self.session_id
    }

    /// Returns whether cancellation changed state.
    #[must_use]
    pub const fn status(self) -> SessionCancellationStatus {
        self.status
    }
}

/// Consumed session authority and its resolved advisory target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalTransferAttempt {
    session_id: DragSessionId,
    source: TransferSourceAuthority,
    target: ResolvedTransferTarget,
}

/// Consumed whole-Surface session whose screen point hit no managed window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmptyDisplayTransferAttempt {
    session_id: DragSessionId,
    source: TransferSourceAuthority,
    screen_point: ScreenPoint,
}

impl EmptyDisplayTransferAttempt {
    pub(crate) const fn new(
        session_id: DragSessionId,
        source: TransferSourceAuthority,
        screen_point: ScreenPoint,
    ) -> Self {
        Self {
            session_id,
            source,
            screen_point,
        }
    }

    /// Returns the consumed session identity.
    #[must_use]
    pub const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    /// Returns source authority recorded at session creation.
    #[must_use]
    pub const fn source(&self) -> &TransferSourceAuthority {
        &self.source
    }

    /// Returns the fresh host screen point outside all managed windows.
    #[must_use]
    pub const fn screen_point(&self) -> ScreenPoint {
        self.screen_point
    }
}

/// First terminal resolution, including explicit empty-display evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalTransferResolution {
    /// A current leased target resolved normally.
    Target(TerminalTransferAttempt),
    /// A screen point hit no current managed window.
    EmptyDisplay(EmptyDisplayTransferAttempt),
}

impl TerminalTransferAttempt {
    pub(crate) const fn new(
        session_id: DragSessionId,
        source: TransferSourceAuthority,
        target: ResolvedTransferTarget,
    ) -> Self {
        Self {
            session_id,
            source,
            target,
        }
    }

    /// Returns the consumed session identity.
    #[must_use]
    pub const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    /// Returns source authority recorded at session creation.
    #[must_use]
    pub const fn source(&self) -> &TransferSourceAuthority {
        &self.source
    }

    /// Returns deterministic target-resolution evidence.
    #[must_use]
    pub const fn target(&self) -> &ResolvedTransferTarget {
        &self.target
    }
}
