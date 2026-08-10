mod evidence;
mod existing;
mod provisioned;
mod validation;

use longhorn_config::ConfigStore;
use longhorn_surfaces::LayoutDocument;
use longhorn_surfaces_config::{RegisteredSurfaceDomain, SurfaceMigration};
use longhorn_transfer::{MonotonicClock, TerminalTransferResolution, TransferCoordinator};

use crate::{
    SurfaceHostBindings, SurfaceTransferCommitReceipt, SurfaceTransferCommitRequest,
    SurfaceTransferError, SurfaceTransferPolicy, SurfaceWindowProvisioner,
};

use self::{existing::commit_existing, provisioned::commit_provisioned};

/// Consumes one target attempt and publishes one authoritative whole-Surface move.
#[allow(clippy::too_many_arguments)]
pub fn commit_surface_transfer<M, P>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    layout_document: &LayoutDocument,
    coordinator: &mut TransferCoordinator,
    clock: &impl MonotonicClock,
    bindings: &SurfaceHostBindings,
    policy: &SurfaceTransferPolicy,
    provisioner: &mut P,
    request: SurfaceTransferCommitRequest,
) -> Result<SurfaceTransferCommitReceipt, SurfaceTransferError>
where
    M: SurfaceMigration,
    P: SurfaceWindowProvisioner,
{
    let resolution = coordinator
        .attempt_target_or_empty_display(
            clock,
            request.session_id(),
            request.selector().clone(),
            request.live_windows(),
        )
        .map_err(SurfaceTransferError::from_transfer)?;
    match resolution {
        TerminalTransferResolution::Target(attempt) => commit_existing(
            store,
            domain,
            layout_document,
            bindings,
            policy,
            request.mutation_options(),
            attempt,
        ),
        TerminalTransferResolution::EmptyDisplay(attempt) => commit_provisioned(
            store,
            domain,
            layout_document,
            bindings,
            policy,
            provisioner,
            request.mutation_options(),
            attempt,
        ),
    }
}
