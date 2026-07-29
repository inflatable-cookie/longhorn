use longhorn_core::{
    DomainId, LayoutContainerId, RegionId, TransferClientId, TransferHostBindingId,
    TransferSubjectId, WindowId,
};
use serde::{Deserialize, Serialize};

use crate::{ClientEpoch, TransferRevision};

/// Capability admitted for one transfer subject and target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TransferCapability {
    /// Move one panel through authoritative layout mutation.
    MovePanel,
    /// Move one hosted Surface through authoritative Surface mutation.
    MoveSurface,
}

/// Product-neutral transfer-subject category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TransferSubjectKind {
    /// Surface-independent panel instance.
    Panel,
    /// Optional hosted Surface, represented without importing a Surface type.
    Surface,
}

/// Host-resolved source authority retained by one process-local session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransferSourceAuthority {
    /// Panel source bound to one current layout container and region.
    Panel {
        /// Initiating renderer client.
        client_id: TransferClientId,
        /// Current initiating renderer epoch.
        client_epoch: ClientEpoch,
        /// Current managed source window.
        source_window_id: WindowId,
        /// Adapter-projected panel subject identity.
        subject_id: TransferSubjectId,
        /// Current panel host binding.
        host_binding_id: TransferHostBindingId,
        /// Authoritative layout document identity.
        document_id: DomainId,
        /// Layout revision recorded at session creation.
        revision: TransferRevision,
        /// Current source layout container.
        container_id: LayoutContainerId,
        /// Current source region.
        region_id: RegionId,
    },
    /// Hosted Surface source retained as opaque host and document evidence.
    Surface {
        /// Initiating renderer client.
        client_id: TransferClientId,
        /// Current initiating renderer epoch.
        client_epoch: ClientEpoch,
        /// Current managed source window.
        source_window_id: WindowId,
        /// Adapter-projected Surface subject identity.
        subject_id: TransferSubjectId,
        /// Current Surface host binding.
        host_binding_id: TransferHostBindingId,
        /// Authoritative Surface document identity.
        document_id: DomainId,
        /// Surface revision recorded at session creation.
        revision: TransferRevision,
    },
}

impl TransferSourceAuthority {
    /// Returns the source renderer client.
    #[must_use]
    pub const fn client_id(&self) -> &TransferClientId {
        match self {
            Self::Panel { client_id, .. } | Self::Surface { client_id, .. } => client_id,
        }
    }

    /// Returns the source renderer epoch.
    #[must_use]
    pub const fn client_epoch(&self) -> ClientEpoch {
        match self {
            Self::Panel { client_epoch, .. } | Self::Surface { client_epoch, .. } => *client_epoch,
        }
    }

    /// Returns the current source window.
    #[must_use]
    pub const fn source_window_id(&self) -> &WindowId {
        match self {
            Self::Panel {
                source_window_id, ..
            }
            | Self::Surface {
                source_window_id, ..
            } => source_window_id,
        }
    }

    /// Returns the adapter-supplied transfer subject identity.
    #[must_use]
    pub const fn subject_id(&self) -> &TransferSubjectId {
        match self {
            Self::Panel { subject_id, .. } | Self::Surface { subject_id, .. } => subject_id,
        }
    }

    /// Returns the source host binding identity.
    #[must_use]
    pub const fn host_binding_id(&self) -> &TransferHostBindingId {
        match self {
            Self::Panel {
                host_binding_id, ..
            }
            | Self::Surface {
                host_binding_id, ..
            } => host_binding_id,
        }
    }

    /// Returns the source authoritative document identity.
    #[must_use]
    pub const fn document_id(&self) -> &DomainId {
        match self {
            Self::Panel { document_id, .. } | Self::Surface { document_id, .. } => document_id,
        }
    }

    /// Returns the source revision recorded at session creation.
    #[must_use]
    pub const fn revision(&self) -> TransferRevision {
        match self {
            Self::Panel { revision, .. } | Self::Surface { revision, .. } => *revision,
        }
    }

    /// Returns the subject category.
    #[must_use]
    pub const fn subject_kind(&self) -> TransferSubjectKind {
        match self {
            Self::Panel { .. } => TransferSubjectKind::Panel,
            Self::Surface { .. } => TransferSubjectKind::Surface,
        }
    }

    /// Returns the only target capability admitted by this source shape.
    #[must_use]
    pub const fn capability(&self) -> TransferCapability {
        match self {
            Self::Panel { .. } => TransferCapability::MovePanel,
            Self::Surface { .. } => TransferCapability::MoveSurface,
        }
    }

    /// Returns panel source placement when the subject is a panel.
    #[must_use]
    pub const fn panel_placement(&self) -> Option<(&LayoutContainerId, &RegionId)> {
        match self {
            Self::Panel {
                container_id,
                region_id,
                ..
            } => Some((container_id, region_id)),
            Self::Surface { .. } => None,
        }
    }
}
