use std::collections::BTreeSet;

use longhorn_core::{WindowId, WindowPlacement};
use serde::{Deserialize, Serialize};

use super::HostWindowHandle;

/// One independently declared host mutation capability.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostCapability {
    /// Create a neutral hidden unmaximized host slot.
    Create,
    /// Change host bookkeeping from a transport handle to a logical id.
    Retag,
    /// Apply outer origin plus inner content size.
    MoveResize,
    /// Enter maximized state.
    Maximize,
    /// Leave maximized state.
    Unmaximize,
    /// Show a host window.
    Show,
    /// Hide a host window.
    Hide,
    /// Focus a host window.
    Focus,
    /// Close a host window.
    Close,
}

/// Explicit host capability set.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct HostCapabilities(BTreeSet<HostCapability>);

impl HostCapabilities {
    /// Constructs a host with no mutation capabilities.
    #[must_use]
    pub const fn none() -> Self {
        Self(BTreeSet::new())
    }

    /// Constructs the complete Card 016 capability set.
    #[must_use]
    pub fn all() -> Self {
        Self(BTreeSet::from([
            HostCapability::Create,
            HostCapability::Retag,
            HostCapability::MoveResize,
            HostCapability::Maximize,
            HostCapability::Unmaximize,
            HostCapability::Show,
            HostCapability::Hide,
            HostCapability::Focus,
            HostCapability::Close,
        ]))
    }

    /// Constructs an exact capability set.
    #[must_use]
    pub fn from_capabilities(capabilities: impl IntoIterator<Item = HostCapability>) -> Self {
        Self(capabilities.into_iter().collect())
    }

    /// Returns whether the host declared one capability.
    #[must_use]
    pub fn supports(&self, capability: HostCapability) -> bool {
        self.0.contains(&capability)
    }
}

/// Stable operation category used by diagnostics and feedback evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowOperationKind {
    /// Retag host bookkeeping.
    Retag,
    /// Create a host slot.
    Create,
    /// Leave maximized state.
    Unmaximize,
    /// Apply placement.
    MoveResize,
    /// Enter maximized state.
    Maximize,
    /// Show a window.
    Show,
    /// Hide a window.
    Hide,
    /// Focus a window.
    Focus,
    /// Close a window.
    Close,
}

/// One pure host mutation instruction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WindowOperation {
    /// Assign a logical id to an existing protected host slot.
    Retag {
        /// Desired logical identity.
        window_id: WindowId,
        /// Existing opaque host handle.
        transport_handle: HostWindowHandle,
    },
    /// Create a neutral hidden unmaximized slot for a logical window.
    Create {
        /// Desired logical identity.
        window_id: WindowId,
    },
    /// Leave maximized state before restoring normal geometry.
    Unmaximize {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
    },
    /// Apply desired outer origin and inner content size.
    MoveResize {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
        /// Desired frame-distinct placement.
        placement: WindowPlacement,
    },
    /// Enter maximized state after normal geometry is honest.
    Maximize {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
    },
    /// Show a window after geometry is applied.
    Show {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
    },
    /// Hide a window after geometry is applied.
    Hide {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
    },
    /// Focus one visible desired window.
    Focus {
        /// Logical identity.
        window_id: WindowId,
        /// Existing handle, absent only for a newly created slot.
        transport_handle: Option<HostWindowHandle>,
    },
    /// Close one stale logical window.
    Close {
        /// Stale logical identity.
        window_id: WindowId,
        /// Existing opaque host handle.
        transport_handle: HostWindowHandle,
    },
}

impl WindowOperation {
    /// Returns operation category.
    #[must_use]
    pub const fn kind(&self) -> WindowOperationKind {
        match self {
            Self::Retag { .. } => WindowOperationKind::Retag,
            Self::Create { .. } => WindowOperationKind::Create,
            Self::Unmaximize { .. } => WindowOperationKind::Unmaximize,
            Self::MoveResize { .. } => WindowOperationKind::MoveResize,
            Self::Maximize { .. } => WindowOperationKind::Maximize,
            Self::Show { .. } => WindowOperationKind::Show,
            Self::Hide { .. } => WindowOperationKind::Hide,
            Self::Focus { .. } => WindowOperationKind::Focus,
            Self::Close { .. } => WindowOperationKind::Close,
        }
    }

    /// Returns stable logical target identity.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        match self {
            Self::Retag { window_id, .. }
            | Self::Create { window_id }
            | Self::Unmaximize { window_id, .. }
            | Self::MoveResize { window_id, .. }
            | Self::Maximize { window_id, .. }
            | Self::Show { window_id, .. }
            | Self::Hide { window_id, .. }
            | Self::Focus { window_id, .. }
            | Self::Close { window_id, .. } => window_id,
        }
    }

    /// Returns an existing transport handle when the operation has one.
    #[must_use]
    pub const fn transport_handle(&self) -> Option<&HostWindowHandle> {
        match self {
            Self::Retag {
                transport_handle, ..
            }
            | Self::Close {
                transport_handle, ..
            } => Some(transport_handle),
            Self::Unmaximize {
                transport_handle, ..
            }
            | Self::MoveResize {
                transport_handle, ..
            }
            | Self::Maximize {
                transport_handle, ..
            }
            | Self::Show {
                transport_handle, ..
            }
            | Self::Hide {
                transport_handle, ..
            }
            | Self::Focus {
                transport_handle, ..
            } => transport_handle.as_ref(),
            Self::Create { .. } => None,
        }
    }
}
