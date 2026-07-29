use std::{error::Error, fmt};

use longhorn_core::{
    ClientRect, DropZoneId, PanelInstanceId, ScreenPoint, TransferClientId, TransferRequestId,
};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{
    ClientEpoch, InsertionPosition, LeaseGeneration, TRANSFER_PROTOCOL_VERSION, TransferCapability,
    TransferTargetBinding,
};

/// Exact supported transfer wire-protocol version.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct TransferProtocolVersion(u32);

impl TransferProtocolVersion {
    /// Current transfer wire-protocol version.
    pub const CURRENT: Self = Self(TRANSFER_PROTOCOL_VERSION);

    /// Returns the exact serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for TransferProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        if value == TRANSFER_PROTOCOL_VERSION {
            Ok(Self(value))
        } else {
            Err(de::Error::custom(UnsupportedTransferProtocolVersion(value)))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct UnsupportedTransferProtocolVersion(u32);

impl fmt::Display for UnsupportedTransferProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "transfer protocol version {} is unsupported; expected {TRANSFER_PROTOCOL_VERSION}",
            self.0
        )
    }
}

impl Error for UnsupportedTransferProtocolVersion {}

/// Renderer request to admit one current movable panel.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PanelSessionStartRequest {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    panel_instance_id: PanelInstanceId,
}

impl PanelSessionStartRequest {
    /// Constructs one request without renderer-supplied host authority.
    #[must_use]
    pub const fn new(request_id: TransferRequestId, panel_instance_id: PanelInstanceId) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            panel_instance_id,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &TransferRequestId {
        &self.request_id
    }

    /// Returns the renderer-selected panel identity.
    #[must_use]
    pub const fn panel_instance_id(&self) -> &PanelInstanceId {
        &self.panel_instance_id
    }
}

/// Renderer request to cancel one process-local transfer session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferCancelRequest {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: crate::DragSessionId,
}

impl TransferCancelRequest {
    /// Constructs one cancellation request.
    #[must_use]
    pub const fn new(request_id: TransferRequestId, session_id: crate::DragSessionId) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            session_id,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &TransferRequestId {
        &self.request_id
    }

    /// Returns the named session.
    #[must_use]
    pub const fn session_id(&self) -> crate::DragSessionId {
        self.session_id
    }
}

/// One renderer-local advisory drop zone before checked host projection.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ClientDropZone {
    id: DropZoneId,
    bounds: ClientRect,
    insertion_position: Option<InsertionPosition>,
    accepted_capability: TransferCapability,
    target: TransferTargetBinding,
}

impl ClientDropZone {
    /// Constructs one client-local advisory zone.
    #[must_use]
    pub const fn new(
        id: DropZoneId,
        bounds: ClientRect,
        insertion_position: Option<InsertionPosition>,
        accepted_capability: TransferCapability,
        target: TransferTargetBinding,
    ) -> Self {
        Self {
            id,
            bounds,
            insertion_position,
            accepted_capability,
            target,
        }
    }

    /// Returns stable lease-local zone identity.
    #[must_use]
    pub const fn id(&self) -> &DropZoneId {
        &self.id
    }

    /// Returns validated client-local bounds.
    #[must_use]
    pub const fn bounds(&self) -> ClientRect {
        self.bounds
    }

    /// Returns the optional bounded insertion position.
    #[must_use]
    pub const fn insertion_position(&self) -> Option<InsertionPosition> {
        self.insertion_position
    }

    /// Returns the advertised accepted capability.
    #[must_use]
    pub const fn accepted_capability(&self) -> TransferCapability {
        self.accepted_capability
    }

    /// Returns advisory target authority for fresh host revalidation.
    #[must_use]
    pub const fn target(&self) -> &TransferTargetBinding {
        &self.target
    }
}

/// Complete replacement lease for the caller's managed window.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct TransferLeaseRequest {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    client_id: TransferClientId,
    client_epoch: ClientEpoch,
    generation: LeaseGeneration,
    zones: Vec<ClientDropZone>,
}

impl TransferLeaseRequest {
    /// Constructs one complete replacement without window or clock evidence.
    #[must_use]
    pub fn new(
        request_id: TransferRequestId,
        client_id: TransferClientId,
        client_epoch: ClientEpoch,
        generation: LeaseGeneration,
        zones: impl IntoIterator<Item = ClientDropZone>,
    ) -> Self {
        Self {
            protocol_version: TransferProtocolVersion::CURRENT,
            request_id,
            client_id,
            client_epoch,
            generation,
            zones: zones.into_iter().collect(),
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &TransferRequestId {
        &self.request_id
    }

    /// Returns the client identity issued by the host snapshot.
    #[must_use]
    pub const fn client_id(&self) -> &TransferClientId {
        &self.client_id
    }

    /// Returns the host-issued current client epoch.
    #[must_use]
    pub const fn client_epoch(&self) -> ClientEpoch {
        self.client_epoch
    }

    /// Returns the monotonic replacement generation.
    #[must_use]
    pub const fn generation(&self) -> LeaseGeneration {
        self.generation
    }

    /// Returns the complete client-local zone replacement.
    #[must_use]
    pub fn zones(&self) -> &[ClientDropZone] {
        self.zones.as_slice()
    }
}

/// Renderer-safe terminal target selector.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum TransferCommitSelector {
    /// Name one current leased target.
    ExplicitZone {
        /// Current lease-local zone identity.
        drop_zone_id: DropZoneId,
    },
    /// Use one untrusted screen-DIP point for fresh host hit-testing.
    ScreenPoint {
        /// Screen-DIP point with no target or mutation authority.
        #[cfg_attr(feature = "bindings", ts(type = "{ x: number; y: number }"))]
        point: ScreenPoint,
    },
}

/// Renderer request for one terminal same-document panel move.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PanelTransferCommand {
    protocol_version: TransferProtocolVersion,
    request_id: TransferRequestId,
    session_id: crate::DragSessionId,
    selector: TransferCommitSelector,
}

impl PanelTransferCommand {
    /// Constructs one terminal move request.
    #[must_use]
    pub const fn new(
        request_id: TransferRequestId,
        session_id: crate::DragSessionId,
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
    pub const fn session_id(&self) -> crate::DragSessionId {
        self.session_id
    }

    /// Returns the renderer-safe target selector.
    #[must_use]
    pub const fn selector(&self) -> &TransferCommitSelector {
        &self.selector
    }
}
