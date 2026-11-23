use longhorn_config::{ConfigStore, MutationOptions};
use longhorn_core::{LayoutRequestId, PanelInstanceId, SurfaceRevision};
use longhorn_surfaces::{LayoutMutationCommand, LayoutMutationRequest};
use longhorn_surfaces_config::{
    RegisteredSurfaceDomain, SurfaceMigration, publish_layout_mutation,
};

use crate::{MonotonicClock, TerminalTransferAttempt, TransferCoordinator};

use super::{
    PanelHostBindings, PanelTransferCommitReceipt, PanelTransferCommitRequest, PanelTransferError,
    PanelTransferErrorCode, PanelTransferOperation,
    admission::{load_layout, panel_placement},
};

use self::{
    evidence::{panel_source, panel_target},
    validation::{as_consumed, consumed, map_mutation_error, require_binding, require_same_domain},
};

mod evidence;
mod validation;

/// Consumes one target attempt and publishes one authoritative same-document move.
pub fn commit_panel_transfer<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    coordinator: &mut TransferCoordinator,
    clock: &impl MonotonicClock,
    bindings: &PanelHostBindings,
    options: MutationOptions,
    request: PanelTransferCommitRequest,
) -> Result<PanelTransferCommitReceipt, PanelTransferError>
where
    M: SurfaceMigration,
{
    let attempt = coordinator
        .attempt_target_resolution(
            clock,
            request.session_id(),
            request.selector().clone(),
            request.live_windows(),
        )
        .map_err(PanelTransferError::from_transfer)?;
    commit_resolved_panel_transfer(
        store,
        domain,
        bindings,
        options,
        request.operation(),
        attempt,
    )
}

/// Publishes one authoritative same-document move for an already-resolved
/// terminal attempt without holding a coordinator lock.
pub fn commit_resolved_panel_transfer<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    bindings: &PanelHostBindings,
    options: MutationOptions,
    operation: PanelTransferOperation,
    attempt: TerminalTransferAttempt,
) -> Result<PanelTransferCommitReceipt, PanelTransferError>
where
    M: SurfaceMigration,
{
    if operation == PanelTransferOperation::Copy {
        return Err(consumed(
            PanelTransferErrorCode::CopyUnsupported,
            "panel copy is unsupported by the first transfer line",
        ));
    }

    let source = panel_source(&attempt)?;
    let target = panel_target(&attempt)?;
    require_same_domain(domain, source.document_id, target.document_id)?;
    if source.revision != target.revision {
        return Err(consumed(
            PanelTransferErrorCode::StaleSurfaceRevision,
            "source and target advertised different layout revisions",
        ));
    }

    let source_binding = bindings.get(source.host_binding_id).map_err(as_consumed)?;
    require_binding(
        source_binding,
        source.window_id,
        source.document_id,
        source.surface_id,
    )?;
    let target_binding = bindings.get(target.host_binding_id).map_err(as_consumed)?;
    require_binding(
        target_binding,
        target.window_id,
        target.document_id,
        target.surface_id,
    )?;

    let document = load_layout(store, domain).map_err(as_consumed)?;
    if document.revision().get() != source.revision {
        return Err(consumed(
            PanelTransferErrorCode::StaleSurfaceRevision,
            format!(
                "current layout revision {} differs from recorded revision {}",
                document.revision().get(),
                source.revision
            ),
        ));
    }
    let panel_instance_id = PanelInstanceId::from(source.subject_id);
    let panel = document.panel_instance(&panel_instance_id).ok_or_else(|| {
        consumed(
            PanelTransferErrorCode::UnknownPanel,
            format!("source panel {panel_instance_id} no longer exists"),
        )
    })?;
    let definition = domain
        .registry()
        .panel_definition(panel.definition_id())
        .expect("a registered layout domain returns a validated document");
    if !definition.is_movable() {
        return Err(consumed(
            PanelTransferErrorCode::PanelNotMovable,
            format!("source panel {panel_instance_id} is no longer movable"),
        ));
    }
    let placement = panel_placement(&document, &panel_instance_id).ok_or_else(|| {
        consumed(
            PanelTransferErrorCode::SourceChanged,
            format!("source panel {panel_instance_id} has no current placement"),
        )
    })?;
    if placement.0 != *source.surface_id || placement.1 != *source.region_id {
        return Err(consumed(
            PanelTransferErrorCode::SourceChanged,
            format!("source panel {panel_instance_id} moved after admission"),
        ));
    }

    let target_container = document.surface(target.surface_id).ok_or_else(|| {
        consumed(
            PanelTransferErrorCode::TargetChanged,
            format!("target container {} no longer exists", target.surface_id),
        )
    })?;
    let target_region = target_container.region(target.region_id).ok_or_else(|| {
        consumed(
            PanelTransferErrorCode::TargetChanged,
            format!("target region {} no longer exists", target.region_id),
        )
    })?;
    let insertion_index = attempt.target().zone().insertion_position().map_or_else(
        || {
            u32::try_from(target_region.panel_instance_ids().len()).map_err(|_| {
                consumed(
                    PanelTransferErrorCode::InvalidInsertionPosition,
                    "current target region length exceeds u32",
                )
            })
        },
        |position| Ok(position.get()),
    )?;

    let mutation = LayoutMutationRequest::new(
        LayoutRequestId::new(format!("transfer:{}", attempt.session_id()))
            .expect("transfer-derived layout request id is bounded and grammatical"),
        SurfaceRevision::new(source.revision),
        LayoutMutationCommand::MovePanel {
            panel_instance_id,
            target_surface_id: target.surface_id.clone(),
            target_region_id: target.region_id.clone(),
            insertion_index,
        },
    );
    let publication =
        publish_layout_mutation(store, domain, options, &mutation).map_err(map_mutation_error)?;
    Ok(PanelTransferCommitReceipt::new(
        attempt,
        source_binding.kind(),
        target_binding.kind(),
        publication,
    ))
}
