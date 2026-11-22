use longhorn_core::SurfaceRevision;

use crate::{
    LayoutDefinitionRegistry, SurfaceDocument, SurfaceValidationError, normalize_document,
    validate_document,
};

use super::{
    EmptyWindowPolicy, SurfaceMutationReceipt, SurfaceMutationRejection,
    SurfaceMutationRejectionCode, SurfaceMutationRequest, operation::apply_command,
};

/// Shared failure stage from [`apply_document_mutation`].
pub(crate) enum DocumentMutationError<O> {
    InvalidCurrentDocument(SurfaceValidationError),
    StaleRevision {
        expected: SurfaceRevision,
        current: SurfaceRevision,
    },
    RevisionOverflow(String),
    Operation(O),
    InvalidCandidate(SurfaceValidationError),
}

/// Runs the single whole-document mutation pipeline for both Surface and layout commands:
/// validate current document, verify expected revision, advance revision, apply command
/// to a private candidate, and normalize + validate the whole candidate once.
pub(crate) fn apply_document_mutation<Outcome, OpError>(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
    expected_revision: SurfaceRevision,
    apply_op: impl FnOnce(&mut SurfaceDocument) -> Result<Outcome, OpError>,
) -> Result<(SurfaceRevision, Outcome, SurfaceDocument), DocumentMutationError<OpError>> {
    validate_document(registry, document).map_err(DocumentMutationError::InvalidCurrentDocument)?;
    if expected_revision != document.revision() {
        return Err(DocumentMutationError::StaleRevision {
            expected: expected_revision,
            current: document.revision(),
        });
    }
    let committed_revision = document
        .revision()
        .checked_next()
        .map_err(|error| DocumentMutationError::RevisionOverflow(error.to_string()))?;

    let mut candidate = document.clone();
    let outcome = apply_op(&mut candidate).map_err(DocumentMutationError::Operation)?;
    candidate.set_revision(committed_revision);
    let candidate = normalize_document(registry, &candidate)
        .map_err(DocumentMutationError::InvalidCandidate)?;

    Ok((committed_revision, outcome, candidate))
}

/// Stateless authoritative Surface lifecycle engine.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMutationEngine<'a> {
    registry: &'a LayoutDefinitionRegistry,
    empty_window_policy: EmptyWindowPolicy,
}

impl<'a> SurfaceMutationEngine<'a> {
    /// Binds mutation to one validated definition registry and empty-window policy.
    #[must_use]
    pub const fn new(
        registry: &'a LayoutDefinitionRegistry,
        empty_window_policy: EmptyWindowPolicy,
    ) -> Self {
        Self {
            registry,
            empty_window_policy,
        }
    }

    /// Applies one request to a private candidate or returns exact unchanged evidence.
    // A rejection deliberately carries the exact unchanged authoritative
    // document: that is the protocol's evidence that nothing moved, and Card
    // 179 made the document bigger by folding layout state into it. Boxing the
    // error would change the wire shape to save a stack move on a path that
    // only runs when a mutation is refused.
    #[allow(clippy::result_large_err)]
    pub fn apply(
        &self,
        document: &SurfaceDocument,
        request: &SurfaceMutationRequest,
    ) -> Result<SurfaceMutationReceipt, SurfaceMutationRejection> {
        let (committed_revision, outcome, candidate) = apply_document_mutation(
            self.registry,
            document,
            request.expected_revision(),
            |candidate| {
                apply_command(
                    candidate,
                    request.command(),
                    self.registry,
                    self.empty_window_policy,
                )
            },
        )
        .map_err(|error| match error {
            DocumentMutationError::InvalidCurrentDocument(error) => rejection(
                document,
                request,
                SurfaceMutationRejectionCode::InvalidCurrentDocument,
                format!(
                    "current Surface document failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ),
            DocumentMutationError::StaleRevision { expected, current } => rejection(
                document,
                request,
                SurfaceMutationRejectionCode::StaleRevision,
                format!(
                    "expected Surface revision {}; current revision is {}",
                    expected.get(),
                    current.get()
                ),
            ),
            DocumentMutationError::RevisionOverflow(detail) => rejection(
                document,
                request,
                SurfaceMutationRejectionCode::RevisionOverflow,
                detail,
            ),
            DocumentMutationError::Operation(error) => {
                rejection(document, request, error.code, error.detail)
            }
            DocumentMutationError::InvalidCandidate(error) => rejection(
                document,
                request,
                SurfaceMutationRejectionCode::InvalidCandidate,
                format!(
                    "Surface mutation candidate failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ),
        })?;

        Ok(SurfaceMutationReceipt::new(
            request.request_id().clone(),
            document.revision(),
            committed_revision,
            outcome,
            candidate,
        ))
    }
}

fn rejection(
    document: &SurfaceDocument,
    request: &SurfaceMutationRequest,
    code: SurfaceMutationRejectionCode,
    detail: impl Into<String>,
) -> SurfaceMutationRejection {
    SurfaceMutationRejection::new(request.request_id().clone(), code, detail, document)
}
