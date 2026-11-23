use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Stable transfer rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TransferErrorCode {
    /// An injected clock moved backwards.
    ClockRegressed,
    /// A requested session or lease lifetime was invalid.
    InvalidLifetime,
    /// The finite session store had no reclaimable capacity.
    SessionCapacity,
    /// The injected entropy source failed.
    SessionIdAllocation,
    /// The allocator returned a current session id.
    SessionIdCollision,
    /// The named session was not retained.
    UnknownSession,
    /// The named session reached its expiry boundary.
    SessionExpired,
    /// The named session was cancelled.
    SessionCancelled,
    /// A terminal attempt already consumed the session.
    SessionReplayed,
    /// The source window was destroyed.
    SourceWindowDestroyed,
    /// The source renderer epoch changed.
    SourceClientChanged,
    /// The finite current client-window registry was full.
    ClientWindowCapacity,
    /// A client epoch was unknown for the named window.
    UnknownClientEpoch,
    /// A client or epoch did not match current host authority.
    StaleClientEpoch,
    /// A complete lease generation did not advance.
    StaleLeaseGeneration,
    /// The finite complete lease registry was full.
    LeaseCapacity,
    /// Lease geometry, count, capability, or insertion evidence was invalid.
    InvalidLease,
    /// Current lease authority reached its expiry boundary.
    LeaseExpired,
    /// Fresh managed-window input was invalid.
    InvalidLiveWindows,
    /// A leased target window disappeared.
    TargetWindowMissing,
    /// Fresh window bounds no longer match the leased snapshot.
    StaleWindowGeometry,
    /// No current eligible target matched.
    NoTarget,
    /// More than one current managed window contained the point.
    AmbiguousWindow,
    /// More than one current eligible zone matched.
    AmbiguousZone,
    /// The zone did not accept the source capability.
    IneligibleCapability,
}

impl TransferErrorCode {
    /// Returns the stable wire representation of this rejection code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClockRegressed => "clock_regressed",
            Self::InvalidLifetime => "invalid_lifetime",
            Self::SessionCapacity => "session_capacity",
            Self::SessionIdAllocation => "session_id_allocation",
            Self::SessionIdCollision => "session_id_collision",
            Self::UnknownSession => "unknown_session",
            Self::SessionExpired => "session_expired",
            Self::SessionCancelled => "session_cancelled",
            Self::SessionReplayed => "session_replayed",
            Self::SourceWindowDestroyed => "source_window_destroyed",
            Self::SourceClientChanged => "source_client_changed",
            Self::ClientWindowCapacity => "client_window_capacity",
            Self::UnknownClientEpoch => "unknown_client_epoch",
            Self::StaleClientEpoch => "stale_client_epoch",
            Self::StaleLeaseGeneration => "stale_lease_generation",
            Self::LeaseCapacity => "lease_capacity",
            Self::InvalidLease => "invalid_lease",
            Self::LeaseExpired => "lease_expired",
            Self::InvalidLiveWindows => "invalid_live_windows",
            Self::TargetWindowMissing => "target_window_missing",
            Self::StaleWindowGeometry => "stale_window_geometry",
            Self::NoTarget => "no_target",
            Self::AmbiguousWindow => "ambiguous_window",
            Self::AmbiguousZone => "ambiguous_zone",
            Self::IneligibleCapability => "ineligible_capability",
        }
    }
}

impl fmt::Display for TransferErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Typed process-local transfer rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferError {
    code: TransferErrorCode,
    detail: String,
    session_consumed: bool,
}

impl TransferError {
    pub(crate) fn new(code: TransferErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            session_consumed: false,
        }
    }

    pub(crate) fn consumed(mut self) -> Self {
        self.session_consumed = true;
        self
    }

    /// Returns the stable rejection category.
    #[must_use]
    pub const fn code(&self) -> TransferErrorCode {
        self.code
    }

    /// Returns diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns whether this rejection followed the first terminal attempt.
    #[must_use]
    pub const fn session_consumed(&self) -> bool {
        self.session_consumed
    }
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for TransferError {}
