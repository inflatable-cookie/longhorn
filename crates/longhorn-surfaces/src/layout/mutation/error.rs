use std::{error::Error, fmt};

use longhorn_core::{LayoutRequestId, SurfaceRevision};
use serde::{Deserialize, Serialize};

use crate::SurfaceDocument;

/// Stable typed layout mutation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LayoutMutationRejectionCode {
    /// The supplied current document failed validation.
    InvalidCurrentDocument,
    /// Expected revision did not equal the current revision.
    StaleRevision,
    /// The current revision could not advance without wrapping.
    RevisionOverflow,
    /// A layout container was not present.
    UnknownSurface,
    /// A semantic region was not present in the selected container.
    UnknownRegion,
    /// A sizing slot was not present in the selected container.
    UnknownSizingSlot,
    /// A panel definition was not registered.
    UnknownPanelDefinition,
    /// A panel instance was not present.
    UnknownPanelInstance,
    /// A create command reused an existing panel-instance id.
    DuplicatePanelInstance,
    /// Registered policy rejected panel placement.
    PanelPlacementNotAllowed,
    /// Registered instance-count policy rejected the candidate.
    InstancePolicyExceeded,
    /// Registered close policy rejected the command.
    PanelNotCloseable,
    /// Registered move policy rejected the command.
    PanelNotMovable,
    /// A move targeted the panel's existing region.
    MoveTargetUnchanged,
    /// An insertion index exceeded the target region length.
    InvalidInsertionIndex,
    /// A reorder omitted or added members.
    IncompleteReorder,
    /// A reorder repeated one member.
    DuplicateReorderMember,
    /// A reorder included a member from outside the target region.
    ForeignReorderMember,
    /// A sizing ratio fell outside registered bounds.
    InvalidSizingRatio,
    /// Collapse state was requested for an unsupported region.
    UnsupportedCollapse,
    /// The private candidate failed a remaining invariant.
    InvalidCandidate,
    /// One request id was reused with different request content.
    RequestIdConflict,
}

/// Failed mutation with exact unchanged-state evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct LayoutMutationRejection {
    request_id: LayoutRequestId,
    current_revision: SurfaceRevision,
    code: LayoutMutationRejectionCode,
    detail: String,
    authoritative_document: SurfaceDocument,
}

impl LayoutMutationRejection {
    pub(super) fn new(
        request_id: LayoutRequestId,
        code: LayoutMutationRejectionCode,
        detail: impl Into<String>,
        authoritative_document: &SurfaceDocument,
    ) -> Self {
        Self {
            request_id,
            current_revision: authoritative_document.revision(),
            code,
            detail: detail.into(),
            authoritative_document: authoritative_document.clone(),
        }
    }

    /// Returns stable request identity.
    #[must_use]
    pub const fn request_id(&self) -> &LayoutRequestId {
        &self.request_id
    }

    /// Returns the unchanged current revision.
    #[must_use]
    pub const fn current_revision(&self) -> SurfaceRevision {
        self.current_revision
    }

    /// Returns the stable rejection category.
    #[must_use]
    pub const fn code(&self) -> LayoutMutationRejectionCode {
        self.code
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns the exact unchanged authoritative source document.
    #[must_use]
    pub const fn authoritative_document(&self) -> &SurfaceDocument {
        &self.authoritative_document
    }
}

impl fmt::Display for LayoutMutationRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for LayoutMutationRejection {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OperationRejection {
    pub(super) code: LayoutMutationRejectionCode,
    pub(super) detail: String,
}

pub(super) fn operation_rejection(
    code: LayoutMutationRejectionCode,
    detail: impl Into<String>,
) -> OperationRejection {
    OperationRejection {
        code,
        detail: detail.into(),
    }
}
