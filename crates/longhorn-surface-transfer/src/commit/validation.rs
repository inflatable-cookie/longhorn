use longhorn_core::{DomainId, LayoutContainerId, SurfaceId, WindowId};
use longhorn_surfaces::{SurfaceDocument, SurfaceMutationRejectionCode};
use longhorn_surfaces_config::{
    RegisteredSurfaceDomain, SurfaceConfigMutationError, SurfaceMigration,
};

use crate::{
    SurfaceHostBinding, SurfaceTransferError, SurfaceTransferErrorCode, SurfaceTransferPolicy,
};

use super::evidence::{SurfaceSource, consumed};

pub(super) fn require_domain<M>(
    domain: &RegisteredSurfaceDomain<M>,
    document_id: &DomainId,
) -> Result<(), SurfaceTransferError>
where
    M: SurfaceMigration,
{
    if domain.descriptor().id() != document_id {
        return Err(consumed(
            SurfaceTransferErrorCode::CrossDocument,
            format!(
                "Surface transfer requires registered domain {}; target is {document_id}",
                domain.descriptor().id()
            ),
        ));
    }
    Ok(())
}

pub(super) fn require_binding(
    binding: &SurfaceHostBinding,
    window_id: &WindowId,
    document_id: &DomainId,
) -> Result<(), SurfaceTransferError> {
    if binding.window_id() != window_id || binding.document_id() != document_id {
        return Err(consumed(
            SurfaceTransferErrorCode::StaleHostBinding,
            format!(
                "binding {} no longer maps the recorded window and Surface domain",
                binding.id()
            ),
        ));
    }
    Ok(())
}

pub(super) fn require_fresh_source(
    document: &SurfaceDocument,
    source: &SurfaceSource<'_>,
) -> Result<LayoutContainerId, SurfaceTransferError> {
    if document.revision().get() != source.revision {
        return Err(consumed(
            SurfaceTransferErrorCode::StaleSurfaceRevision,
            format!(
                "current Surface revision {} differs from recorded revision {}",
                document.revision().get(),
                source.revision
            ),
        ));
    }
    let surface = document.surface(&source.surface_id).ok_or_else(|| {
        consumed(
            SurfaceTransferErrorCode::UnknownSurface,
            format!("source Surface {} no longer exists", source.surface_id),
        )
    })?;
    let primary = surface
        .host_preferences()
        .first()
        .expect("validated Surface has a host preference")
        .window_id();
    if primary != source.window_id {
        return Err(consumed(
            SurfaceTransferErrorCode::SourceChanged,
            format!(
                "source Surface {} moved from {} to {primary}",
                source.surface_id, source.window_id
            ),
        ));
    }
    Ok(surface.layout_container_id().clone())
}

pub(super) fn require_target(
    document: &SurfaceDocument,
    policy: &SurfaceTransferPolicy,
    surface_id: &SurfaceId,
    target_window_id: &WindowId,
) -> Result<(), SurfaceTransferError> {
    if !policy.allows_window(target_window_id) {
        return Err(consumed(
            SurfaceTransferErrorCode::IneligibleTarget,
            format!("consumer policy rejects Surface target {target_window_id}"),
        ));
    }
    if document.window(target_window_id).is_none() {
        return Err(consumed(
            SurfaceTransferErrorCode::TargetChanged,
            format!("target window {target_window_id} no longer participates"),
        ));
    }
    let declared = document.surface(surface_id).is_some_and(|surface| {
        surface
            .host_preferences()
            .iter()
            .any(|preference| preference.window_id() == target_window_id)
    });
    if !declared {
        return Err(consumed(
            SurfaceTransferErrorCode::TargetChanged,
            format!("Surface {surface_id} does not declare target {target_window_id}"),
        ));
    }
    Ok(())
}

pub(super) fn insertion_index(
    document: &SurfaceDocument,
    surface_id: &SurfaceId,
    target_window_id: &WindowId,
    requested: Option<u32>,
) -> Result<u32, SurfaceTransferError> {
    let count = document
        .surfaces()
        .iter()
        .filter(|surface| surface.id() != surface_id)
        .filter(|surface| {
            surface
                .host_preferences()
                .iter()
                .any(|preference| preference.window_id() == target_window_id)
        })
        .count();
    let maximum = u32::try_from(count).map_err(|_| {
        consumed(
            SurfaceTransferErrorCode::InvalidInsertionPosition,
            "current target Surface count exceeds u32",
        )
    })?;
    let insertion = requested.unwrap_or(maximum);
    if insertion > maximum {
        return Err(consumed(
            SurfaceTransferErrorCode::InvalidInsertionPosition,
            format!("target insertion {insertion} exceeds current membership length {maximum}"),
        ));
    }
    Ok(insertion)
}

pub(super) fn map_mutation_error(error: SurfaceConfigMutationError) -> SurfaceTransferError {
    match error {
        SurfaceConfigMutationError::Rejected(rejection) => {
            let code = match rejection.code() {
                SurfaceMutationRejectionCode::StaleRevision => {
                    SurfaceTransferErrorCode::StaleSurfaceRevision
                }
                SurfaceMutationRejectionCode::UnknownSurface => {
                    SurfaceTransferErrorCode::UnknownSurface
                }
                SurfaceMutationRejectionCode::UnknownWindow
                | SurfaceMutationRejectionCode::UndeclaredTargetWindow => {
                    SurfaceTransferErrorCode::TargetChanged
                }
                SurfaceMutationRejectionCode::InvalidInsertionIndex => {
                    SurfaceTransferErrorCode::InvalidInsertionPosition
                }
                SurfaceMutationRejectionCode::EmptyWindowNotAllowed => {
                    SurfaceTransferErrorCode::IneligibleTarget
                }
                _ => SurfaceTransferErrorCode::SurfaceMutationRejected,
            };
            consumed(code, rejection.detail()).with_surface_code(rejection.code())
        }
        SurfaceConfigMutationError::Config(error) => consumed(
            SurfaceTransferErrorCode::PublicationFailed,
            error.to_string(),
        ),
    }
}
