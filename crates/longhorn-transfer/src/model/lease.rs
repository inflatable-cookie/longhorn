use longhorn_core::{
    DomainId, DropZoneId, LayoutContainerId, RegionId, ScreenRect, TransferHostBindingId,
};
use serde::{Deserialize, Serialize};

use crate::{InsertionPosition, TransferRevision};

use super::TransferCapability;

/// Authority binding advertised by one advisory drop zone.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransferTargetBinding {
    /// One Surface-independent layout region.
    PanelRegion {
        /// Current panel-host binding.
        host_binding_id: TransferHostBindingId,
        /// Authoritative layout document identity.
        document_id: DomainId,
        /// Advertised layout revision.
        revision: TransferRevision,
        /// Target layout container.
        container_id: LayoutContainerId,
        /// Target semantic region.
        region_id: RegionId,
    },
    /// One hosted-window target without importing a Surface type.
    SurfaceWindow {
        /// Current target host binding.
        host_binding_id: TransferHostBindingId,
        /// Authoritative Surface document identity.
        document_id: DomainId,
        /// Advertised Surface revision.
        revision: TransferRevision,
    },
}

impl TransferTargetBinding {
    /// Returns the capability required to select this target.
    #[must_use]
    pub const fn capability(&self) -> TransferCapability {
        match self {
            Self::PanelRegion { .. } => TransferCapability::MovePanel,
            Self::SurfaceWindow { .. } => TransferCapability::MoveSurface,
        }
    }

    /// Returns the adapter-supplied host binding.
    #[must_use]
    pub const fn host_binding_id(&self) -> &TransferHostBindingId {
        match self {
            Self::PanelRegion {
                host_binding_id, ..
            }
            | Self::SurfaceWindow {
                host_binding_id, ..
            } => host_binding_id,
        }
    }

    /// Returns the authoritative target document.
    #[must_use]
    pub const fn document_id(&self) -> &DomainId {
        match self {
            Self::PanelRegion { document_id, .. } | Self::SurfaceWindow { document_id, .. } => {
                document_id
            }
        }
    }

    /// Returns the advertised target revision.
    #[must_use]
    pub const fn revision(&self) -> TransferRevision {
        match self {
            Self::PanelRegion { revision, .. } | Self::SurfaceWindow { revision, .. } => *revision,
        }
    }
}

/// One bounded advisory target inside a complete window lease.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DropZone {
    id: DropZoneId,
    bounds: ScreenRect,
    insertion_position: Option<InsertionPosition>,
    accepted_capability: TransferCapability,
    target: TransferTargetBinding,
}

impl DropZone {
    /// Constructs one advisory target. Publication performs contextual checks.
    #[must_use]
    pub const fn new(
        id: DropZoneId,
        bounds: ScreenRect,
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

    /// Returns the zone identity.
    #[must_use]
    pub const fn id(&self) -> &DropZoneId {
        &self.id
    }

    /// Returns the global screen-DIP rectangle.
    #[must_use]
    pub const fn bounds(&self) -> ScreenRect {
        self.bounds
    }

    /// Returns the advisory insertion position.
    #[must_use]
    pub const fn insertion_position(&self) -> Option<InsertionPosition> {
        self.insertion_position
    }

    /// Returns the accepted capability.
    #[must_use]
    pub const fn accepted_capability(&self) -> TransferCapability {
        self.accepted_capability
    }

    /// Returns the advertised target authority.
    #[must_use]
    pub const fn target(&self) -> &TransferTargetBinding {
        &self.target
    }
}
