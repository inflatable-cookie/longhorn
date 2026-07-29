use std::{error::Error, fmt};

use longhorn_core::{SurfaceRequestId, SurfaceRevision};
use serde::{Deserialize, Serialize};

use crate::SurfaceDocument;

/// Stable typed Surface mutation rejection category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMutationRejectionCode {
    /// The supplied current document failed validation.
    InvalidCurrentDocument,
    /// Expected revision did not equal the current revision.
    StaleRevision,
    /// The current revision could not advance without wrapping.
    RevisionOverflow,
    /// A Surface was not present.
    UnknownSurface,
    /// A create or duplicate command reused a Surface id.
    DuplicateSurface,
    /// The supplied layout container was not present in the evidence document.
    UnknownLayoutContainer,
    /// The supplied layout container was already bound to another Surface.
    LayoutContainerAlreadyBound,
    /// A participating window was not present.
    UnknownWindow,
    /// A host preference repeated one window.
    DuplicateHostPreference,
    /// A target was not declared in the Surface hosting policy.
    UndeclaredTargetWindow,
    /// A move targeted the Surface's current primary host.
    MoveTargetUnchanged,
    /// An insertion index exceeded the target membership length.
    InvalidInsertionIndex,
    /// A reorder omitted or added members.
    IncompleteReorder,
    /// A reorder repeated one member.
    DuplicateReorderMember,
    /// A reorder included a Surface outside the window membership.
    ForeignReorderMember,
    /// Consumer policy forbids the resulting empty participating window.
    EmptyWindowNotAllowed,
    /// The private candidate failed a remaining invariant.
    InvalidCandidate,
}

/// Failed mutation with exact unchanged-state evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceMutationRejection {
    request_id: SurfaceRequestId,
    current_revision: SurfaceRevision,
    code: SurfaceMutationRejectionCode,
    detail: String,
    authoritative_document: SurfaceDocument,
}

impl SurfaceMutationRejection {
    pub(super) fn new(
        request_id: SurfaceRequestId,
        code: SurfaceMutationRejectionCode,
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
    pub const fn request_id(&self) -> &SurfaceRequestId {
        &self.request_id
    }

    /// Returns the unchanged current revision.
    #[must_use]
    pub const fn current_revision(&self) -> SurfaceRevision {
        self.current_revision
    }

    /// Returns the stable rejection category.
    #[must_use]
    pub const fn code(&self) -> SurfaceMutationRejectionCode {
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

impl fmt::Display for SurfaceMutationRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceMutationRejection {}

pub(super) struct OperationRejection {
    pub(super) code: SurfaceMutationRejectionCode,
    pub(super) detail: String,
}

pub(super) fn operation_rejection(
    code: SurfaceMutationRejectionCode,
    detail: impl Into<String>,
) -> OperationRejection {
    OperationRejection {
        code,
        detail: detail.into(),
    }
}
