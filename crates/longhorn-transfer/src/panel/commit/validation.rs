use longhorn_core::{DomainId, SurfaceId, WindowId};
use longhorn_surfaces::LayoutMutationRejectionCode;
use longhorn_surfaces_config::{
    LayoutConfigMutationError, LayoutMigration, RegisteredLayoutDomain,
};

use crate::panel::{PanelHostBinding, PanelTransferError, PanelTransferErrorCode};

pub(super) fn require_same_domain<M>(
    domain: &RegisteredLayoutDomain<M>,
    source_document_id: &DomainId,
    target_document_id: &DomainId,
) -> Result<(), PanelTransferError>
where
    M: LayoutMigration,
{
    let registered_id = domain.descriptor().id();
    if source_document_id != target_document_id
        || source_document_id != registered_id
        || target_document_id != registered_id
    {
        return Err(consumed(
            PanelTransferErrorCode::CrossDocument,
            format!(
                "panel transfer requires registered domain {registered_id}; source is \
                 {source_document_id} and target is {target_document_id}"
            ),
        ));
    }
    Ok(())
}

pub(super) fn require_binding(
    binding: &PanelHostBinding,
    window_id: &WindowId,
    document_id: &DomainId,
    surface_id: &SurfaceId,
) -> Result<(), PanelTransferError> {
    if binding.window_id() != window_id
        || binding.document_id() != document_id
        || binding.surface_id() != surface_id
    {
        return Err(consumed(
            PanelTransferErrorCode::StaleHostBinding,
            format!(
                "binding {} no longer maps the recorded window, domain, and container",
                binding.id()
            ),
        ));
    }
    Ok(())
}

pub(super) fn map_mutation_error(error: LayoutConfigMutationError) -> PanelTransferError {
    match error {
        LayoutConfigMutationError::Rejected(rejection) => {
            let code = match rejection.code() {
                LayoutMutationRejectionCode::StaleRevision => {
                    PanelTransferErrorCode::StaleSurfaceRevision
                }
                LayoutMutationRejectionCode::UnknownPanelInstance => {
                    PanelTransferErrorCode::UnknownPanel
                }
                LayoutMutationRejectionCode::PanelNotMovable => {
                    PanelTransferErrorCode::PanelNotMovable
                }
                LayoutMutationRejectionCode::UnknownSurface
                | LayoutMutationRejectionCode::UnknownRegion => {
                    PanelTransferErrorCode::TargetChanged
                }
                LayoutMutationRejectionCode::PanelPlacementNotAllowed => {
                    PanelTransferErrorCode::IneligibleTarget
                }
                LayoutMutationRejectionCode::InstancePolicyExceeded => {
                    PanelTransferErrorCode::InstancePolicyExceeded
                }
                LayoutMutationRejectionCode::InvalidInsertionIndex => {
                    PanelTransferErrorCode::InvalidInsertionPosition
                }
                _ => PanelTransferErrorCode::LayoutMutationRejected,
            };
            PanelTransferError::from_layout_rejection(code, &rejection)
        }
        LayoutConfigMutationError::Config(error) => {
            consumed(PanelTransferErrorCode::PublicationFailed, error.to_string())
        }
    }
}

pub(super) fn consumed(
    code: PanelTransferErrorCode,
    detail: impl Into<String>,
) -> PanelTransferError {
    PanelTransferError::new(code, detail).consumed()
}

pub(super) fn as_consumed(error: PanelTransferError) -> PanelTransferError {
    error.consumed()
}
