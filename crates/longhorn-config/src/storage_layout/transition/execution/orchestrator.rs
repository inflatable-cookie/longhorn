use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use cap_std::{ambient_authority, fs::Dir};
use longhorn_core::DomainId;

use crate::StorageRootsResolve;
use crate::{DomainLocation, DurabilityRequirement, Sha256Digest};
use longhorn_storage_layout::{StorageProfileLocator, encode_locator, locator_matches};

use super::super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionError,
    StorageTransitionExecutionOptions, StorageTransitionOutcome, StorageTransitionPlan,
    StorageTransitionReceipt, StorageTransitionRecoveryReceipt, StorageTransitionRequest,
    inventory, io as transition_io,
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
    recover_inner(request, lock_timeout, crate::dir_sync::sync_dir_handle)
}

pub(crate) fn recover_inner(
    request: &StorageTransitionRequest<'_>,
    lock_timeout: std::time::Duration,
    sync_dir: impl Fn(&Dir) -> io::Result<()>,
) -> Result<Option<StorageTransitionRecoveryReceipt>, StorageTransitionError> {
    let _adapter_guards = acquire_adapter_guards(request, lock_timeout)?;
    let (_first, _second) = acquire_store_guards(request, lock_timeout)?;
    let Some(active) = journal::read(&request.bootstrap)? else {
        return Ok(None);
    };
    if !crate::storage_layout::layout_digest_matches(
        &active.source_layout_digest,
        request.source_layout.digest(),
    ) || !crate::storage_layout::layout_digest_matches(
        &active.target_layout_digest,
        request.target_layout.digest(),
    ) {
        return Err(StorageTransitionError::RecoveryRequired(
            "journal layout evidence does not match supplied layouts".into(),
        ));
    }
    let target_committed = locator_matches(
        &request.bootstrap,
        request.source_layout.identity().canonical_application_id(),
        &active.transition_id,
        &crate::storage_layout::layout_evidence(&active.target_layout_digest),
    )
    .map_err(|error| StorageTransitionError::Locator(error.to_string()))?;
    verify_journal_authority(request, &active, target_committed)?;
    if !target_committed && active.phase != JournalPhase::Prepared {
        remove_verified_interrupted_targets(request, &active, &sync_dir)?;
    }
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

/// Removes ordinary target copies written by an interrupted transition only
/// when resolved through the registered destination capability and proven to
/// match the interrupted copy's exact expected evidence. Unexpected or
/// modified target files are preserved untouched and require manual recovery.
///
/// Even when the target file is already absent (for example, a prior recovery
/// attempt unlinked it and then failed before syncing the parent directory),
/// the destination parent directory is still synced if it exists so a crash
/// cannot resurrect the unlinked entry after the journal is cleared.
fn remove_verified_interrupted_targets(
    request: &StorageTransitionRequest<'_>,
    active: &TransitionJournal,
    sync_dir: &impl Fn(&Dir) -> io::Result<()>,
) -> Result<(), StorageTransitionError> {
    for entry in &active.domains {
        if entry.custom || !entry.target_was_absent {
            continue;
        }
        let id = DomainId::new(&entry.domain)
            .map_err(|_| StorageTransitionError::Journal("invalid journal domain".into()))?;
        let descriptor = request
            .target_store
            .registered_descriptor(&id)
            .ok_or_else(|| StorageTransitionError::DescriptorMismatch { domain: id.clone() })?;
        let DomainLocation::File(target_file) = request.target_store.roots.resolve(descriptor)
        else {
            return Err(StorageTransitionError::UnavailableDomain { domain: id });
        };
        if entry.target_path.as_deref() != Some(target_file.full_path()) {
            return Err(StorageTransitionError::RecoveryRequired(format!(
                "target path for {} no longer matches transition journal",
                entry.domain
            )));
        }
        let Some(StorageFileEvidence::Present {
            byte_length: expected_len,
            sha256: expected_sha,
        }) = entry.target_expected.as_ref()
        else {
            return Err(StorageTransitionError::Journal(format!(
                "transition journal missing expected copy evidence for {}",
                entry.domain
            )));
        };

        let root = Dir::open_ambient_dir(target_file.root(), ambient_authority())
            .map_err(|error| fs_error(target_file.root(), error))?;
        let relative = target_file.relative_path().as_path();
        let parent_path = relative.parent().unwrap_or_else(|| Path::new(""));
        let mut input = match root.open(relative) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A previous recovery attempt may have unlinked the file and
                // failed before syncing its parent; sync the parent if it exists.
                let parent = if parent_path.as_os_str().is_empty() {
                    Some(
                        root.try_clone()
                            .map_err(|error| fs_error(target_file.full_path(), error))?,
                    )
                } else {
                    match root.open_dir(parent_path) {
                        Ok(dir) => Some(dir),
                        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                        Err(error) => return Err(fs_error(target_file.full_path(), error)),
                    }
                };
                if let Some(parent) = parent {
                    sync_dir(&parent).map_err(|error| fs_error(target_file.full_path(), error))?;
                }
                continue;
            }
            Err(error) => return Err(fs_error(target_file.full_path(), error)),
        };
        let observed_len = input
            .metadata()
            .map_err(|error| fs_error(target_file.full_path(), error))?
            .len();
        if observed_len != *expected_len as u64 {
            return Err(StorageTransitionError::RecoveryRequired(format!(
                "target at {} does not match interrupted copy; preserving file",
                target_file.full_path().display()
            )));
        }
        let mut bytes = Vec::with_capacity(*expected_len);
        input
            .by_ref()
            .take((*expected_len as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| fs_error(target_file.full_path(), error))?;
        drop(input);
        if bytes.len() != *expected_len || Sha256Digest::from_bytes(&bytes) != *expected_sha {
            return Err(StorageTransitionError::RecoveryRequired(format!(
                "target at {} does not match interrupted copy; preserving file",
                target_file.full_path().display()
            )));
        }

        let parent = if parent_path.as_os_str().is_empty() {
            root.try_clone()
        } else {
            root.open_dir(parent_path)
        }
        .map_err(|error| fs_error(target_file.full_path(), error))?;
        let file_name = relative
            .file_name()
            .ok_or_else(|| StorageTransitionError::Filesystem {
                path: target_file.full_path().to_path_buf(),
                detail: "target has no file name".into(),
            })?;
        match parent.remove_file(file_name) {
            Ok(()) => {
                sync_dir(&parent).map_err(|error| fs_error(target_file.full_path(), error))?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                sync_dir(&parent).map_err(|error| fs_error(target_file.full_path(), error))?
            }
            Err(error) => return Err(fs_error(target_file.full_path(), error)),
        }
    }
    Ok(())
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
            | StorageTransitionAction::Excluded(_)
            | StorageTransitionAction::Conflict => {}
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
        Some(crate::storage_layout::layout_evidence(
            &plan.preview.target_layout_digest,
        )),
    );
    let encoded = encode_locator(&locator)
        .map_err(|error| StorageTransitionError::Locator(error.to_string()))?;
    transition_io::atomic_write(request.bootstrap.locator(), &encoded)?;
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
