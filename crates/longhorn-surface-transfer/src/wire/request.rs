use longhorn_core::{SurfaceId, TransferRequestId};
use longhorn_transfer::{DragSessionId, TransferCommitSelector, TransferProtocolVersion};
use serde::{Deserialize, Serialize};

/// Renderer request to admit one current whole Surface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceSessionStartRequest {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    surface_id: SurfaceId,
}

impl SurfaceSessionStartRequest {
    /// Constructs one request without renderer-supplied host authority.
    #[must_use]
    pub const fn new(request_id: TransferRequestId, surface_id: SurfaceId) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            surface_id,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &TransferRequestId {
        &self.request_id
    }

    /// Returns the renderer-selected Surface identity.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }
}

/// Renderer request for one terminal whole-Surface move.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceTransferCommand {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: DragSessionId,
    selector: TransferCommitSelector,
}

impl SurfaceTransferCommand {
    /// Constructs one terminal move request.
    #[must_use]
    pub const fn new(
        request_id: TransferRequestId,
        session_id: DragSessionId,
        selector: TransferCommitSelector,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            session_id,
            selector,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &TransferRequestId {
        &self.request_id
    }

    /// Returns the consumed session candidate.
    #[must_use]
    pub const fn session_id(&self) -> DragSessionId {
        self.session_id
    }

    /// Returns the renderer-safe target selector.
    #[must_use]
    pub const fn selector(&self) -> &TransferCommitSelector {
        &self.selector
    }
}
