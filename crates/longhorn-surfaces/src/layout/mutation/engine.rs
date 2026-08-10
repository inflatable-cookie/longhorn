use crate::layout::validation::{normalize_document, validate_document};
use crate::{LayoutDefinitionRegistry, LayoutDocument};

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
    pub fn apply(
        &self,
        document: &LayoutDocument,
        request: &LayoutMutationRequest,
    ) -> Result<LayoutMutationReceipt, LayoutMutationRejection> {
        if let Err(error) = validate_document(self.registry, document) {
            return Err(rejection(
                document,
                request,
                LayoutMutationRejectionCode::InvalidCurrentDocument,
                format!(
                    "current layout document failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ));
        }
        if request.expected_revision() != document.revision() {
            return Err(rejection(
                document,
                request,
                LayoutMutationRejectionCode::StaleRevision,
                format!(
                    "expected layout revision {}; current revision is {}",
                    request.expected_revision().get(),
                    document.revision().get()
                ),
            ));
        }
        let committed_revision = document.revision().checked_next().map_err(|error| {
            rejection(
                document,
                request,
                LayoutMutationRejectionCode::RevisionOverflow,
                error.to_string(),
            )
        })?;

        let mut candidate = document.clone();
        let outcome = apply_command(self.registry, &mut candidate, request.command())
            .map_err(|error| rejection(document, request, error.code, error.detail))?;
        candidate.set_revision(committed_revision);
        let candidate = normalize_document(self.registry, &candidate).map_err(|error| {
            let code = map_candidate_validation(error.code());
            rejection(
                document,
                request,
                code,
                format!(
                    "layout mutation candidate failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            )
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
    pub fn apply_with_replay(
        &self,
        document: &LayoutDocument,
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
    document: &LayoutDocument,
    request: &LayoutMutationRequest,
    code: LayoutMutationRejectionCode,
    detail: impl Into<String>,
) -> LayoutMutationRejection {
    LayoutMutationRejection::new(request.request_id().clone(), code, detail, document)
}
