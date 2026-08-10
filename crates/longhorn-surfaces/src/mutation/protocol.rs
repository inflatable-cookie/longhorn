use longhorn_core::{LayoutContainerId, SurfaceId, SurfaceRequestId, SurfaceRevision, WindowId};
use serde::{Deserialize, Serialize};

use crate::{SurfaceDocument, SurfaceHostPreference, SurfacePresentation};

use super::{LayoutContainerCleanupIntent, SurfaceMutationRejection};

/// One strict expected-revision Surface mutation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceMutationRequest {
    request_id: SurfaceRequestId,
    expected_revision: SurfaceRevision,
    command: SurfaceMutationCommand,
}

impl SurfaceMutationRequest {
    /// Constructs one mutation request.
    #[must_use]
    pub const fn new(
        request_id: SurfaceRequestId,
        expected_revision: SurfaceRevision,
        command: SurfaceMutationCommand,
    ) -> Self {
        Self {
            request_id,
            expected_revision,
            command,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &SurfaceRequestId {
        &self.request_id
    }

    /// Returns the revision required for admission.
    #[must_use]
    pub const fn expected_revision(&self) -> SurfaceRevision {
        self.expected_revision
    }

    /// Returns the requested command.
    #[must_use]
    pub const fn command(&self) -> &SurfaceMutationCommand {
        &self.command
    }
}

/// Authoritative Surface lifecycle command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum SurfaceMutationCommand {
    /// Creates a caller-identified Surface bound to an existing container.
    CreateSurface {
        /// New durable Surface identity.
        surface_id: SurfaceId,
        /// Existing unbound layout-container identity.
        layout_container_id: LayoutContainerId,
        /// Optional generic display label.
        label: Option<String>,
        /// Complete candidate-host policy with explicit window orders.
        host_preferences: Vec<SurfaceHostPreference>,
    },
    /// Copies generic Surface metadata to caller-supplied fresh identities.
    DuplicateSurface {
        /// Existing generic metadata source.
        source_surface_id: SurfaceId,
        /// New durable Surface identity.
        surface_id: SurfaceId,
        /// Existing unbound layout-container identity.
        layout_container_id: LayoutContainerId,
    },
    /// Replaces one Surface's optional display label.
    RenameSurface {
        /// Existing Surface identity.
        surface_id: SurfaceId,
        /// New optional generic label.
        label: Option<String>,
    },
    /// Replaces how one Surface presents its bound layout container.
    SetSurfacePresentation {
        /// Existing Surface identity.
        surface_id: SurfaceId,
        /// Regional layout, or one panel rendered full-surface.
        presentation: SurfacePresentation,
    },
    /// Selects one declared member in a participating window.
    ActivateSurface {
        /// Participating window.
        window_id: WindowId,
        /// Declared Surface member.
        surface_id: SurfaceId,
    },
    /// Replaces one window's membership order with a complete permutation.
    ReorderWindow {
        /// Participating window.
        window_id: WindowId,
        /// Complete ordered permutation of declared members.
        surface_ids: Vec<SurfaceId>,
    },
    /// Makes another declared candidate the Surface's primary host.
    MoveSurface {
        /// Existing Surface identity.
        surface_id: SurfaceId,
        /// Declared target candidate window.
        target_window_id: WindowId,
        /// Zero-based target-window insertion index.
        insertion_index: u32,
    },
    /// Removes one Surface and returns external cleanup intent.
    CloseSurface {
        /// Existing Surface identity.
        surface_id: SurfaceId,
    },
}

/// Command-specific committed Surface mutation evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
pub enum SurfaceMutationOutcome {
    /// One Surface was created.
    SurfaceCreated {
        /// Created Surface identity.
        surface_id: SurfaceId,
    },
    /// Generic Surface metadata was duplicated without layout contents.
    SurfaceDuplicated {
        /// Generic metadata source.
        source_surface_id: SurfaceId,
        /// Created Surface identity.
        surface_id: SurfaceId,
    },
    /// One Surface label was replaced.
    SurfaceRenamed {
        /// Renamed Surface identity.
        surface_id: SurfaceId,
    },
    /// One Surface changed how it presents its container.
    SurfacePresentationSet {
        /// Affected Surface identity.
        surface_id: SurfaceId,
        /// Committed presentation.
        presentation: SurfacePresentation,
        /// Presentation replaced by this command.
        previous_presentation: SurfacePresentation,
    },
    /// One window selected a declared Surface.
    SurfaceActivated {
        /// Participating window.
        window_id: WindowId,
        /// Newly active Surface.
        surface_id: SurfaceId,
        /// Previous active member, when selected.
        previous_active_surface_id: Option<SurfaceId>,
    },
    /// One window accepted a complete committed membership order.
    WindowReordered {
        /// Reordered participating window.
        window_id: WindowId,
        /// Complete committed membership order.
        surface_ids: Vec<SurfaceId>,
    },
    /// One Surface changed primary host.
    SurfaceMoved {
        /// Moved Surface identity.
        surface_id: SurfaceId,
        /// Former primary host.
        source_window_id: WindowId,
        /// New primary host.
        target_window_id: WindowId,
        /// Committed target membership index.
        insertion_index: u32,
    },
    /// One Surface was removed without executing cross-domain cleanup.
    SurfaceClosed {
        /// Closed Surface identity.
        surface_id: SurfaceId,
        /// Explicit unexecuted cross-domain cleanup work.
        cleanup: LayoutContainerCleanupIntent,
    },
}

/// Successful authoritative Surface mutation receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceMutationReceipt {
    request_id: SurfaceRequestId,
    previous_revision: SurfaceRevision,
    committed_revision: SurfaceRevision,
    outcome: SurfaceMutationOutcome,
    authoritative_document: SurfaceDocument,
}

impl SurfaceMutationReceipt {
    pub(super) const fn new(
        request_id: SurfaceRequestId,
        previous_revision: SurfaceRevision,
        committed_revision: SurfaceRevision,
        outcome: SurfaceMutationOutcome,
        authoritative_document: SurfaceDocument,
    ) -> Self {
        Self {
            request_id,
            previous_revision,
            committed_revision,
            outcome,
            authoritative_document,
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &SurfaceRequestId {
        &self.request_id
    }

    /// Returns the admitted source revision.
    #[must_use]
    pub const fn previous_revision(&self) -> SurfaceRevision {
        self.previous_revision
    }

    /// Returns the single committed successor revision.
    #[must_use]
    pub const fn committed_revision(&self) -> SurfaceRevision {
        self.committed_revision
    }

    /// Returns command-specific committed evidence.
    #[must_use]
    pub const fn outcome(&self) -> &SurfaceMutationOutcome {
        &self.outcome
    }

    /// Returns the complete normalized authoritative document.
    #[must_use]
    pub const fn authoritative_document(&self) -> &SurfaceDocument {
        &self.authoritative_document
    }
}

/// Serialized outcome of one authoritative Surface mutation request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "snake_case"))]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "status")]
pub enum SurfaceMutationResponse {
    /// The mutation committed once.
    Committed {
        /// Complete authoritative mutation receipt.
        receipt: SurfaceMutationReceipt,
    },
    /// The mutation preserved the exact current authority.
    Rejected {
        /// Typed unchanged-state rejection.
        rejection: SurfaceMutationRejection,
    },
}

impl From<Result<SurfaceMutationReceipt, SurfaceMutationRejection>> for SurfaceMutationResponse {
    fn from(result: Result<SurfaceMutationReceipt, SurfaceMutationRejection>) -> Self {
        match result {
            Ok(receipt) => Self::Committed { receipt },
            Err(rejection) => Self::Rejected { rejection },
        }
    }
}
