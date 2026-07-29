use longhorn_config::{ConfigStore, MutationOptions};
use longhorn_core::{SurfaceRequestId, SurfaceRevision};
use longhorn_layout::LayoutDocument;
use longhorn_surfaces::{SurfaceMutationCommand, SurfaceMutationRequest};
use longhorn_surfaces_config::{
    RegisteredSurfaceDomain, SurfaceMigration, publish_surface_mutation,
};
use longhorn_transfer::TerminalTransferAttempt;

use crate::{
    SurfaceHostBindings, SurfaceTerminalAttempt, SurfaceTransferCommitReceipt,
    SurfaceTransferError, SurfaceTransferPolicy,
};

use super::{
    evidence::{existing_source, existing_target},
    validation::{
        insertion_index, map_mutation_error, require_binding, require_domain, require_fresh_source,
        require_target,
    },
};
use crate::admission::load_surface;

#[allow(clippy::too_many_arguments)]
pub(super) fn commit_existing<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    layout_document: &LayoutDocument,
    bindings: &SurfaceHostBindings,
    policy: &SurfaceTransferPolicy,
    options: MutationOptions,
    attempt: TerminalTransferAttempt,
) -> Result<SurfaceTransferCommitReceipt, SurfaceTransferError>
where
    M: SurfaceMigration,
{
    let source = existing_source(&attempt)?;
    let target = existing_target(&attempt)?;
    require_domain(domain, source.document_id)?;
    require_domain(domain, target.document_id)?;
    if source.revision != target.revision {
        return Err(super::evidence::consumed(
            crate::SurfaceTransferErrorCode::StaleSurfaceRevision,
            "source and target advertised different Surface revisions",
        ));
    }
    let source_binding = bindings.get(source.host_binding_id)?;
    require_binding(source_binding, source.window_id, source.document_id)?;
    let target_binding = bindings.get(target.host_binding_id)?;
    require_binding(target_binding, target.window_id, target.document_id)?;

    let document = load_surface(store, domain)?.clone();
    let layout_container_id = require_fresh_source(&document, &source)?;
    require_target(&document, policy, &source.surface_id, target.window_id)?;
    let insertion = insertion_index(
        &document,
        &source.surface_id,
        target.window_id,
        target.insertion_index,
    )?;
    let mutation = move_request(
        attempt.session_id(),
        source.surface_id.clone(),
        source.revision,
        target.window_id.clone(),
        insertion,
    );
    let publication = publish_surface_mutation(
        store,
        domain,
        options,
        layout_document,
        policy.empty_window_policy(),
        &mutation,
    )
    .map_err(map_mutation_error)?;
    let committed = publication
        .surface()
        .authoritative_document()
        .surface(&source.surface_id)
        .expect("successful move retains the Surface");
    assert_eq!(
        committed.layout_container_id(),
        &layout_container_id,
        "Surface move must retain its external layout-container binding"
    );
    Ok(SurfaceTransferCommitReceipt::new(
        SurfaceTerminalAttempt::Existing(attempt),
        source_binding.id().clone(),
        target_binding.id().clone(),
        publication,
        None,
    ))
}

pub(super) fn move_request(
    session_id: longhorn_transfer::DragSessionId,
    surface_id: longhorn_core::SurfaceId,
    revision: u64,
    target_window_id: longhorn_core::WindowId,
    insertion_index: u32,
) -> SurfaceMutationRequest {
    SurfaceMutationRequest::new(
        SurfaceRequestId::new(format!("transfer:{session_id}"))
            .expect("transfer-derived Surface request id is bounded and grammatical"),
        SurfaceRevision::new(revision),
        SurfaceMutationCommand::MoveSurface {
            surface_id,
            target_window_id,
            insertion_index,
        },
    )
}
