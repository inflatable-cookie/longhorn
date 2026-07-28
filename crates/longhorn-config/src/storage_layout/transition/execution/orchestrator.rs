use std::fs;

use crate::{
    DomainLocation, DurabilityRequirement, Sha256Digest, StorageProfileLocator,
    storage_layout::bootstrap::{encode_locator, locator_matches},
};

use super::super::{
    StorageTransitionAction, StorageTransitionError, StorageTransitionExecutionOptions,
    StorageTransitionOutcome, StorageTransitionPlan, StorageTransitionReceipt,
    StorageTransitionRecoveryReceipt, StorageTransitionRequest, inventory, io,
    journal::{self, JournalPhase, TransitionJournal},
};
use super::{
    acquire_adapter_guards, acquire_store_guards, capture_custom, fs_error, receipt_digest,
    restore_custom, stage_ordinary, stage_path, verify_journal_authority, verify_path,
};

/// Executes staged ordinary copies and adapter restores, then commits the locator last.
pub fn execute_storage_transition(
    request: &StorageTransitionRequest<'_>,
    plan: &StorageTransitionPlan,
    confirmation: &Sha256Digest,
    options: StorageTransitionExecutionOptions,
) -> Result<StorageTransitionReceipt, StorageTransitionError> {
    execute_inner(request, plan, confirmation, options, None)
}

/// Recovers one interrupted transition to the authority selected by the locator.
pub fn recover_storage_transition(
    request: &StorageTransitionRequest<'_>,
    lock_timeout: std::time::Duration,
) -> Result<Option<StorageTransitionRecoveryReceipt>, StorageTransitionError> {
    let Some(active) = journal::read(&request.bootstrap)? else {
        return Ok(None);
    };
    if active.source_layout_digest != *request.source_layout.digest()
        || active.target_layout_digest != *request.target_layout.digest()
    {
        return Err(StorageTransitionError::RecoveryRequired(
            "journal layout evidence does not match supplied layouts".into(),
        ));
    }
    let _adapter_guards = acquire_adapter_guards(request, lock_timeout)?;
    let (_first, _second) = acquire_store_guards(request, lock_timeout)?;
    let target_committed = locator_matches(
        &request.bootstrap,
        request.source_layout.identity().canonical_application_id(),
        &active.transition_id,
        &active.target_layout_digest,
    )
    .map_err(StorageTransitionError::Locator)?;
    verify_journal_authority(request, &active, target_committed)?;
    journal::cleanup(&request.bootstrap, &active.transition_id)?;
    Ok(Some(StorageTransitionRecoveryReceipt {
        transition_id: active.transition_id,
        outcome: if target_committed {
            StorageTransitionOutcome::TargetCommitted
        } else {
            StorageTransitionOutcome::SourceRetained
        },
    }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InjectedFailure {
    BeforeLocator,
    AfterLocator,
}

pub(crate) fn execute_inner(
    request: &StorageTransitionRequest<'_>,
    plan: &StorageTransitionPlan,
    confirmation: &Sha256Digest,
    options: StorageTransitionExecutionOptions,
    injected: Option<InjectedFailure>,
) -> Result<StorageTransitionReceipt, StorageTransitionError> {
    if plan.confirmation_digest() != confirmation {
        return Err(StorageTransitionError::StalePlan);
    }
    if journal::read(&request.bootstrap)?.is_some() {
        return Err(StorageTransitionError::RecoveryRequired(
            "an unresolved storage transition journal exists".into(),
        ));
    }

    let _adapter_guards = acquire_adapter_guards(request, options.lock_timeout)?;
    let captured_custom = capture_custom(request, plan)?;
    let (_first, _second) = acquire_store_guards(request, options.lock_timeout)?;
    let current = inventory::inspect_storage_transition(request)?;
    if current.confirmation_digest() != plan.confirmation_digest() {
        return Err(StorageTransitionError::StalePlan);
    }
    let staging = request.bootstrap.transitions().join(&options.transition_id);
    stage_ordinary(plan, &staging)?;

    let mut active = TransitionJournal::from_plan(options.transition_id.clone(), plan);
    journal::write(&request.bootstrap, &active)?;
    active.phase = JournalPhase::Publishing;
    journal::write(&request.bootstrap, &active)?;

    let mut copied_domains = Vec::new();
    let mut custom_domains = Vec::new();
    let mut retained_sources = Vec::new();
    for entry in plan.domains() {
        match &entry.action {
            StorageTransitionAction::CopyOrdinary => {
                let descriptor = request
                    .target_store
                    .registered_descriptor(&entry.domain)
                    .ok_or_else(|| StorageTransitionError::DescriptorMismatch {
                        domain: entry.domain.clone(),
                    })?;
                let DomainLocation::File(target) = request.target_store.roots.resolve(descriptor)
                else {
                    return Err(StorageTransitionError::UnavailableDomain {
                        domain: entry.domain.clone(),
                    });
                };
                let bytes = fs::read(stage_path(&staging, &entry.domain))
                    .map_err(|error| fs_error(target.full_path(), error))?;
                crate::store::publication::publish(&target, &bytes, DurabilityRequirement::Durable)
                    .map_err(|error| StorageTransitionError::Filesystem {
                        path: error.path,
                        detail: error.detail,
                    })?;
                verify_path(target.full_path(), entry.source_evidence.as_ref())?;
                copied_domains.push(entry.domain.clone());
                if let (Some(path), Some(evidence)) = (&entry.source_path, &entry.source_evidence) {
                    retained_sources.push((path.clone(), evidence.clone()));
                }
            }
            StorageTransitionAction::CustomAdapter { .. } => {
                restore_custom(request, entry, &captured_custom)?;
                custom_domains.push(entry.domain.clone());
            }
            StorageTransitionAction::Absent
            | StorageTransitionAction::SameAuthority
            | StorageTransitionAction::Identical
            | StorageTransitionAction::Excluded(_) => {}
        }
    }
    if injected == Some(InjectedFailure::BeforeLocator) {
        return Err(StorageTransitionError::RecoveryRequired(
            "injected failure before locator commit".into(),
        ));
    }

    let locator = StorageProfileLocator::new(
        request
            .target_layout
            .identity()
            .canonical_application_id()
            .to_owned(),
        plan.preview.target_selection.clone(),
        Some(options.transition_id.clone()),
        Some(plan.preview.target_layout_digest.clone()),
    );
    io::atomic_write(request.bootstrap.locator(), &encode_locator(&locator))?;
    if injected == Some(InjectedFailure::AfterLocator) {
        return Err(StorageTransitionError::RecoveryRequired(
            "injected failure after locator commit".into(),
        ));
    }
    active.phase = JournalPhase::LocatorCommitted;
    journal::write(&request.bootstrap, &active)?;
    verify_journal_authority(request, &active, true)?;

    copied_domains.sort();
    custom_domains.sort();
    retained_sources.sort_by(|left, right| left.0.cmp(&right.0));
    let (retained_source_paths, retained_source_evidence): (Vec<_>, Vec<_>) =
        retained_sources.into_iter().unzip();
    let receipt_digest = receipt_digest(
        &options.transition_id,
        &plan.preview.target_layout_digest,
        &copied_domains,
        &custom_domains,
        &retained_source_paths,
        &retained_source_evidence,
    );
    journal::cleanup(&request.bootstrap, &options.transition_id)?;
    Ok(StorageTransitionReceipt {
        transition_id: options.transition_id,
        outcome: StorageTransitionOutcome::TargetCommitted,
        target_layout_digest: plan.preview.target_layout_digest.clone(),
        copied_domains,
        custom_domains,
        retained_source_paths,
        retained_source_evidence,
        receipt_digest,
    })
}
