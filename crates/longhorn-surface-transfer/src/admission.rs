use longhorn_config::ConfigStore;
use longhorn_core::TransferSubjectId;
use longhorn_surfaces::SurfaceDocument;
use longhorn_surfaces_config::{RegisteredSurfaceDomain, SurfaceMigration};
use longhorn_transfer::{
    DragSessionIdAllocator, MonotonicClock, SessionCreationReceipt, SurfaceDocumentLoadError,
    TransferCoordinator, TransferRevision, TransferSessionRequest, TransferSourceAuthority,
    load_registered_surface_document,
};

use crate::{
    SurfaceHostBindings, SurfaceSessionAdmission, SurfaceTransferError, SurfaceTransferErrorCode,
};

/// Resolves fresh Surface authority and admits one bounded whole-Surface session.
pub fn admit_surface_session<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    coordinator: &mut TransferCoordinator,
    clock: &impl MonotonicClock,
    allocator: &mut impl DragSessionIdAllocator,
    bindings: &SurfaceHostBindings,
    request: SurfaceSessionAdmission,
) -> Result<SessionCreationReceipt, SurfaceTransferError>
where
    M: SurfaceMigration,
{
    let binding = bindings.get(request.host_binding_id())?;
    if binding.window_id() != request.source_window_id()
        || binding.document_id() != domain.descriptor().id()
    {
        return Err(SurfaceTransferError::new(
            SurfaceTransferErrorCode::StaleHostBinding,
            format!(
                "binding {} does not match current source window and Surface domain",
                binding.id()
            ),
        ));
    }
    let document = load_surface(store, domain)?;
    let surface = document.surface(request.surface_id()).ok_or_else(|| {
        SurfaceTransferError::new(
            SurfaceTransferErrorCode::UnknownSurface,
            format!(
                "Surface {} is absent from current topology",
                request.surface_id()
            ),
        )
    })?;
    let primary = surface
        .host_preferences()
        .first()
        .expect("a registered Surface domain returns a validated document")
        .window_id();
    if primary != request.source_window_id() {
        return Err(SurfaceTransferError::new(
            SurfaceTransferErrorCode::SourceChanged,
            format!(
                "Surface {} is currently hosted by {primary}, not {}",
                request.surface_id(),
                request.source_window_id()
            ),
        ));
    }

    let subject_id = TransferSubjectId::from(request.surface_id());
    let source = TransferSourceAuthority::Surface {
        client_id: request.client_id().clone(),
        client_epoch: request.client_epoch(),
        source_window_id: request.source_window_id().clone(),
        subject_id,
        host_binding_id: request.host_binding_id().clone(),
        document_id: domain.descriptor().id().clone(),
        revision: TransferRevision::new(document.revision().get()),
    };
    coordinator
        .create_session(
            clock,
            allocator,
            TransferSessionRequest::new(source, request.lifetime()),
        )
        .map_err(SurfaceTransferError::from_transfer)
}

pub(crate) fn load_surface<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
) -> Result<SurfaceDocument, SurfaceTransferError>
where
    M: SurfaceMigration,
{
    load_registered_surface_document(store, domain, "Surface").map_err(|error| match error {
        SurfaceDocumentLoadError::Unavailable(detail) => {
            SurfaceTransferError::new(SurfaceTransferErrorCode::SurfaceUnavailable, detail)
        }
        SurfaceDocumentLoadError::Failed(detail) => {
            SurfaceTransferError::new(SurfaceTransferErrorCode::SurfaceLoadFailed, detail)
        }
    })
}
