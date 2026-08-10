use longhorn_config::{ConfigStore, LoadOutcome};
use longhorn_core::{PanelInstanceId, RegionId, SurfaceId, TransferSubjectId};
use longhorn_surfaces::SurfaceDocument;
use longhorn_surfaces_config::{LayoutMigration, RegisteredLayoutDomain};

use crate::{
    DragSessionIdAllocator, MonotonicClock, SessionCreationReceipt, TransferCoordinator,
    TransferRevision, TransferSessionRequest, TransferSourceAuthority,
};

use super::{
    PanelHostBinding, PanelHostBindings, PanelSessionAdmission, PanelTransferError,
    PanelTransferErrorCode,
};

/// Resolves fresh layout authority and admits one bounded movable-panel session.
pub fn admit_panel_session<M>(
    store: &ConfigStore,
    domain: &RegisteredLayoutDomain<M>,
    coordinator: &mut TransferCoordinator,
    clock: &impl MonotonicClock,
    allocator: &mut impl DragSessionIdAllocator,
    bindings: &PanelHostBindings,
    request: PanelSessionAdmission,
) -> Result<SessionCreationReceipt, PanelTransferError>
where
    M: LayoutMigration,
{
    let binding = bindings.get(request.host_binding_id())?;
    require_source_binding(domain, binding, &request)?;
    let document = load_layout(store, domain)?;
    let panel = document
        .panel_instance(request.panel_instance_id())
        .ok_or_else(|| {
            PanelTransferError::new(
                PanelTransferErrorCode::UnknownPanel,
                format!(
                    "panel instance {} is absent from current layout",
                    request.panel_instance_id()
                ),
            )
        })?;
    let definition = domain
        .registry()
        .panel_definition(panel.definition_id())
        .expect("a registered layout domain returns a validated document");
    if !definition.is_movable() {
        return Err(PanelTransferError::new(
            PanelTransferErrorCode::PanelNotMovable,
            format!(
                "panel instance {} is not movable",
                request.panel_instance_id()
            ),
        ));
    }
    let (surface_id, region_id) = panel_placement(&document, request.panel_instance_id())
        .ok_or_else(|| {
            PanelTransferError::new(
                PanelTransferErrorCode::SourceChanged,
                format!(
                    "panel instance {} has no current layout placement",
                    request.panel_instance_id()
                ),
            )
        })?;
    if &surface_id != binding.surface_id() {
        return Err(PanelTransferError::new(
            PanelTransferErrorCode::StaleHostBinding,
            format!(
                "binding {} hosts {}, but panel {} occupies {}",
                binding.id(),
                binding.surface_id(),
                request.panel_instance_id(),
                surface_id
            ),
        ));
    }

    let subject_id = TransferSubjectId::new(request.panel_instance_id().as_str())
        .expect("panel and transfer subject ids share the same grammar");
    let source = TransferSourceAuthority::Panel {
        client_id: request.client_id().clone(),
        client_epoch: request.client_epoch(),
        source_window_id: request.source_window_id().clone(),
        subject_id,
        host_binding_id: request.host_binding_id().clone(),
        document_id: domain.descriptor().id().clone(),
        revision: TransferRevision::new(document.revision().get()),
        surface_id,
        region_id,
    };
    coordinator
        .create_session(
            clock,
            allocator,
            TransferSessionRequest::new(source, request.lifetime()),
        )
        .map_err(PanelTransferError::from_transfer)
}

pub(crate) fn load_layout<M>(
    store: &ConfigStore,
    domain: &RegisteredLayoutDomain<M>,
) -> Result<SurfaceDocument, PanelTransferError>
where
    M: LayoutMigration,
{
    match store.load(domain) {
        Ok(LoadOutcome::Ready(loaded)) => Ok(loaded.value),
        Ok(LoadOutcome::Recovery(state)) => Err(PanelTransferError::new(
            PanelTransferErrorCode::LayoutUnavailable,
            format!("registered layout requires recovery: {}", state.detail),
        )),
        Ok(LoadOutcome::Unavailable(state)) => Err(PanelTransferError::new(
            PanelTransferErrorCode::LayoutUnavailable,
            format!("registered layout authority is unavailable: {state:?}"),
        )),
        Err(error) => Err(PanelTransferError::new(
            PanelTransferErrorCode::LayoutLoadFailed,
            error.to_string(),
        )),
    }
}

pub(crate) fn panel_placement(
    document: &SurfaceDocument,
    panel_instance_id: &PanelInstanceId,
) -> Option<(SurfaceId, RegionId)> {
    document.surfaces().iter().find_map(|container| {
        container.regions().iter().find_map(|region| {
            region
                .panel_instance_ids()
                .contains(panel_instance_id)
                .then(|| (container.id().clone(), region.region_id().clone()))
        })
    })
}

fn require_source_binding<M>(
    domain: &RegisteredLayoutDomain<M>,
    binding: &PanelHostBinding,
    request: &PanelSessionAdmission,
) -> Result<(), PanelTransferError>
where
    M: LayoutMigration,
{
    if binding.window_id() != request.source_window_id()
        || binding.document_id() != domain.descriptor().id()
    {
        return Err(PanelTransferError::new(
            PanelTransferErrorCode::StaleHostBinding,
            format!(
                "binding {} does not match current source window and layout domain",
                binding.id()
            ),
        ));
    }
    Ok(())
}
