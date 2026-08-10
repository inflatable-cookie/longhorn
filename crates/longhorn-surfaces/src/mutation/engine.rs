use crate::{
    LayoutDefinitionRegistry, SurfaceDocument, SurfaceLimits, normalize_document, validate_document,
};

use super::{
    EmptyWindowPolicy, SurfaceMutationReceipt, SurfaceMutationRejection,
    SurfaceMutationRejectionCode, SurfaceMutationRequest,
    operation::{apply_command, map_candidate_validation},
};

/// Stateless authoritative Surface lifecycle engine.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMutationEngine<'a> {
    limits: SurfaceLimits,
    registry: &'a LayoutDefinitionRegistry,
    empty_window_policy: EmptyWindowPolicy,
}

impl<'a> SurfaceMutationEngine<'a> {
    /// Binds mutation to explicit limits, the definition registry, and empty-window policy.
    #[must_use]
    pub const fn new(
        limits: SurfaceLimits,
        registry: &'a LayoutDefinitionRegistry,
        empty_window_policy: EmptyWindowPolicy,
    ) -> Self {
        Self {
            limits,
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
        if let Err(error) = validate_document(self.limits, document) {
            return Err(rejection(
                document,
                request,
                SurfaceMutationRejectionCode::InvalidCurrentDocument,
                format!(
                    "current Surface document failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            ));
        }
        if request.expected_revision() != document.revision() {
            return Err(rejection(
                document,
                request,
                SurfaceMutationRejectionCode::StaleRevision,
                format!(
                    "expected Surface revision {}; current revision is {}",
                    request.expected_revision().get(),
                    document.revision().get()
                ),
            ));
        }
        let committed_revision = document.revision().checked_next().map_err(|error| {
            rejection(
                document,
                request,
                SurfaceMutationRejectionCode::RevisionOverflow,
                error.to_string(),
            )
        })?;

        let mut candidate = document.clone();
        let outcome = apply_command(
            &mut candidate,
            request.command(),
            self.registry,
            self.empty_window_policy,
        )
        .map_err(|error| rejection(document, request, error.code, error.detail))?;
        candidate.set_revision(committed_revision);
        let candidate = normalize_document(self.limits, &candidate).map_err(|error| {
            rejection(
                document,
                request,
                map_candidate_validation(error.code()),
                format!(
                    "Surface mutation candidate failed {:?}: {}",
                    error.code(),
                    error.detail()
                ),
            )
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
