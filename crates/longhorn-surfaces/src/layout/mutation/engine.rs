use crate::{
    LayoutDefinitionRegistry, SurfaceDocument,
    mutation::engine::{DocumentMutationError, apply_document_mutation},
};

use super::{
    BoundedLayoutReplayStore, LayoutMutationReceipt, LayoutMutationRejection,
    LayoutMutationRejectionCode, LayoutMutationRequest,
    operation::{apply_command, map_candidate_validation},
};

/// Stateless authoritative layout mutation engine over one immutable registry.
#[derive(Clone, Copy, Debug)]
pub struct LayoutMutationEngine<'a> {
    registry: &'a LayoutDefinitionRegistry,
}

impl<'a> LayoutMutationEngine<'a> {
    /// Binds the engine to one validated immutable definition registry.
    #[must_use]
    pub const fn new(registry: &'a LayoutDefinitionRegistry) -> Self {
        Self { registry }
    }

    /// Applies one request without any implicit replay authority.
    // A rejection deliberately carries the exact unchanged authoritative
    // document: that is the protocol's evidence that nothing moved, and Card
    // 179 made the document bigger by folding layout state into it. Boxing the
    // error would change the wire shape to save a stack move on a path that
    // only runs when a mutation is refused.
    #[allow(clippy::result_large_err)]
    pub fn apply(
        &self,
        document: &SurfaceDocument,
        request: &LayoutMutationRequest,
    ) -> Result<LayoutMutationReceipt, LayoutMutationRejection> {
        let (committed_revision, outcome, candidate) = apply_document_mutation(
            self.registry,
            document,
            request.expected_revision(),
            |candidate| apply_command(self.registry, candidate, request.command()),
        )
        .map_err(|error| match error {
            DocumentMutationError::InvalidCurrentDocument(error) => rejection(
                document,
                request,
                LayoutMutationRejectionCode::InvalidCurrentDocument,
                format!(
                    "current layout document failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ),
            DocumentMutationError::StaleRevision { expected, current } => rejection(
                document,
                request,
                LayoutMutationRejectionCode::StaleRevision,
                format!(
                    "expected layout revision {}; current revision is {}",
                    expected.get(),
                    current.get()
                ),
            ),
            DocumentMutationError::RevisionOverflow(detail) => rejection(
                document,
                request,
                LayoutMutationRejectionCode::RevisionOverflow,
                detail,
            ),
            DocumentMutationError::Operation(error) => {
                rejection(document, request, error.code, error.detail)
            }
            DocumentMutationError::InvalidCandidate(error) => rejection(
                document,
                request,
                map_candidate_validation(error.code()),
                format!(
                    "layout mutation candidate failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ),
        })?;

        Ok(LayoutMutationReceipt::new(
            request.request_id().clone(),
            document.revision(),
            committed_revision,
            outcome,
            candidate,
        ))
    }

    /// Applies or exactly replays one request through explicit bounded authority.
    #[allow(clippy::result_large_err)]
    pub fn apply_with_replay(
        &self,
        document: &SurfaceDocument,
        request: &LayoutMutationRequest,
        replay_store: &mut BoundedLayoutReplayStore,
    ) -> Result<LayoutMutationReceipt, LayoutMutationRejection> {
        if let Some(replayed) = replay_store.lookup(request) {
            return replayed.map_err(|()| {
                rejection(
                    document,
                    request,
                    LayoutMutationRejectionCode::RequestIdConflict,
                    format!(
                        "layout request id {} was already used for different content",
                        request.request_id()
                    ),
                )
            });
        }

        let receipt = self.apply(document, request)?;
        replay_store.record(request, &receipt);
        Ok(receipt)
    }
}

fn rejection(
    document: &SurfaceDocument,
    request: &LayoutMutationRequest,
    code: LayoutMutationRejectionCode,
    detail: impl Into<String>,
) -> LayoutMutationRejection {
    LayoutMutationRejection::new(request.request_id().clone(), code, detail, document)
}
