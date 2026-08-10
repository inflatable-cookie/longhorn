use longhorn_config::{ConfigStore, MutationOptions};
use longhorn_surfaces::LayoutDefinitionRegistry;
use longhorn_surfaces_config::{
    RegisteredSurfaceDomain, SurfaceMigration, publish_surface_mutation,
};
use longhorn_transfer::EmptyDisplayTransferAttempt;

use crate::{
    CompletedSurfaceProvision, ProvisionCleanupOutcome, SurfaceHostBindings,
    SurfaceProvisionFailureEvidence, SurfaceTerminalAttempt, SurfaceTransferCommitReceipt,
    SurfaceTransferError, SurfaceTransferErrorCode, SurfaceTransferPolicy,
    SurfaceWindowProvisionFailure, SurfaceWindowProvisionRequest, SurfaceWindowProvisionStage,
    SurfaceWindowProvisioner, policy::EmptyTargetResolution,
};

use super::{
    evidence::{consumed, empty_source},
    existing::move_request,
    validation::{
        insertion_index, map_mutation_error, require_binding, require_domain, require_fresh_source,
        require_target,
    },
};
use crate::admission::load_surface;

#[allow(clippy::too_many_arguments)]
pub(super) fn commit_provisioned<M, P>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    registry: &LayoutDefinitionRegistry,
    bindings: &SurfaceHostBindings,
    policy: &SurfaceTransferPolicy,
    provisioner: &mut P,
    options: MutationOptions,
    attempt: EmptyDisplayTransferAttempt,
) -> Result<SurfaceTransferCommitReceipt, SurfaceTransferError>
where
    M: SurfaceMigration,
    P: SurfaceWindowProvisioner,
{
    let source = empty_source(&attempt)?;
    require_domain(domain, source.document_id)?;
    // The empty-display attempt already consumed the session; every failure
    // from here on must say so or the renderer will replay a dead session.
    let source_binding = bindings
        .get(source.host_binding_id)
        .map_err(SurfaceTransferError::consumed)?;
    require_binding(source_binding, source.window_id, source.document_id)?;
    let target = policy
        .empty_target(attempt.screen_point())
        .map_err(map_empty_target)?;

    let document = load_surface(store, domain).map_err(SurfaceTransferError::consumed)?;
    let surface_id = require_fresh_source(&document, &source)?;
    require_target(&document, policy, &source.surface_id, target.window_id())?;
    let insertion = insertion_index(
        &document,
        &source.surface_id,
        target.window_id(),
        target.insertion_index(),
    )?;
    let request = SurfaceWindowProvisionRequest::new(
        attempt.session_id(),
        source.surface_id.clone(),
        target.window_id().clone(),
        target.display_id().clone(),
        attempt.screen_point(),
        target.placement(),
    );
    let prepared = provisioner.provision(&request).map_err(|failure| {
        consumed(SurfaceTransferErrorCode::ProvisionFailed, failure.detail())
            .with_provisioning(SurfaceProvisionFailureEvidence::ProvisionFailed(failure))
    })?;
    let (mut authority, provision) = prepared.parts();
    if provision.window_id() != target.window_id()
        || provision.display_id() != target.display_id()
        || provision.placement() != target.placement()
    {
        let cleanup = cleanup(provisioner, &mut authority);
        let cleanup_failed = matches!(cleanup, ProvisionCleanupOutcome::Failed(_));
        let error = consumed(
            SurfaceTransferErrorCode::ProvisionReceiptMismatch,
            "provisioner returned authority for another admitted target",
        )
        .with_provisioning(SurfaceProvisionFailureEvidence::PreparedTargetRejected {
            provision,
            cleanup,
        });
        return Err(if cleanup_failed {
            error.reconciliation_required(
                "provisioner returned another target and cleanup remains unresolved",
            )
        } else {
            error
        });
    }

    let mutation = move_request(
        attempt.session_id(),
        source.surface_id.clone(),
        source.revision,
        target.window_id().clone(),
        insertion,
    );
    let publication = match publish_surface_mutation(
        store,
        domain,
        options,
        registry,
        policy.empty_window_policy(),
        &mutation,
    ) {
        Ok(publication) => publication,
        Err(error) => {
            let cleanup = cleanup(provisioner, &mut authority);
            let cleanup_failed = matches!(cleanup, ProvisionCleanupOutcome::Failed(_));
            let mapped = map_mutation_error(error).with_provisioning(
                SurfaceProvisionFailureEvidence::PublicationFailed { provision, cleanup },
            );
            return Err(if cleanup_failed {
                mapped.reconciliation_required(
                    "Surface publication failed and provisioned target cleanup remains unresolved",
                )
            } else {
                mapped
            });
        }
    };
    let committed = publication
        .surface()
        .authoritative_document()
        .surface(&source.surface_id)
        .expect("successful move retains the Surface");
    // The document is durably committed and a native window is provisioned;
    // a binding drift is reconciliation evidence, never a release-profile
    // abort mid-reconciliation.
    if committed.id() != &surface_id {
        let failure = SurfaceWindowProvisionFailure::new(
            SurfaceWindowProvisionStage::Commit,
            "Surface moved but its retained external layout-container binding changed",
        );
        return Err(consumed(
            SurfaceTransferErrorCode::HostReconciliationRequired,
            failure.detail(),
        )
        .with_provisioning(SurfaceProvisionFailureEvidence::ReconciliationRequired {
            provision,
            publication: Box::new(publication),
            failure,
        }));
    }
    let commit = provisioner.commit(&mut authority).map_err(|failure| {
        consumed(
            SurfaceTransferErrorCode::HostReconciliationRequired,
            "Surface committed but provisioned window host commit failed",
        )
        .with_provisioning(SurfaceProvisionFailureEvidence::ReconciliationRequired {
            provision: provision.clone(),
            publication: Box::new(publication.clone()),
            failure,
        })
    })?;
    if commit.window_id() != target.window_id() {
        let failure = SurfaceWindowProvisionFailure::new(
            SurfaceWindowProvisionStage::Commit,
            "provisioner committed another logical target",
        );
        return Err(consumed(
            SurfaceTransferErrorCode::HostReconciliationRequired,
            failure.detail(),
        )
        .with_provisioning(SurfaceProvisionFailureEvidence::ReconciliationRequired {
            provision,
            publication: Box::new(publication),
            failure,
        }));
    }
    let target_binding_id = provision.host_binding_id().clone();
    Ok(SurfaceTransferCommitReceipt::new(
        SurfaceTerminalAttempt::EmptyDisplay(attempt),
        source_binding.id().clone(),
        target_binding_id,
        publication,
        Some(CompletedSurfaceProvision::new(provision, commit)),
    ))
}

fn cleanup<P: SurfaceWindowProvisioner>(
    provisioner: &mut P,
    authority: &mut P::Authority,
) -> ProvisionCleanupOutcome {
    match provisioner.cleanup(authority) {
        Ok(receipt) => ProvisionCleanupOutcome::Succeeded(receipt),
        Err(failure) => ProvisionCleanupOutcome::Failed(failure),
    }
}

fn map_empty_target(error: EmptyTargetResolution) -> SurfaceTransferError {
    match error {
        EmptyTargetResolution::Disabled => consumed(
            SurfaceTransferErrorCode::EmptyDisplayDisabled,
            "empty-display Surface provisioning is disabled",
        ),
        EmptyTargetResolution::NoMatch => consumed(
            SurfaceTransferErrorCode::NoEmptyDisplayTarget,
            "no consumer-approved display target contains the drop point",
        ),
        EmptyTargetResolution::Ambiguous => consumed(
            SurfaceTransferErrorCode::AmbiguousEmptyDisplayTarget,
            "multiple consumer-approved display targets contain the drop point",
        ),
    }
}
