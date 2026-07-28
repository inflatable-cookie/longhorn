use std::{fs, path::Path, time::Duration};

use crate::{DomainLocation, StorageFileEvidence};

use super::{
    StorageTransitionCleanupPlan, StorageTransitionCleanupReceipt, StorageTransitionError,
    StorageTransitionOutcome, StorageTransitionReceipt, StorageTransitionRequest, execution, io,
};
use crate::storage_layout::bootstrap::locator_matches;

/// Applies an exact, receipt-bound cleanup after target authority is reverified.
pub fn apply_storage_transition_cleanup(
    request: &StorageTransitionRequest<'_>,
    receipt: &StorageTransitionReceipt,
    plan: &StorageTransitionCleanupPlan,
    lock_timeout: Duration,
) -> Result<StorageTransitionCleanupReceipt, StorageTransitionError> {
    validate_binding(receipt, plan)?;
    if receipt.target_layout_digest != *request.target_layout.digest()
        || request.source_layout.identity().canonical_application_id()
            != request.target_layout.identity().canonical_application_id()
    {
        return Err(StorageTransitionError::CleanupRefused(
            "cleanup request does not match the receipt target layout".into(),
        ));
    }
    let (_first, _second) = execution::acquire_store_guards(request, lock_timeout)?;
    let target_committed = locator_matches(
        &request.bootstrap,
        request.source_layout.identity().canonical_application_id(),
        receipt.transition_id(),
        receipt.target_layout_digest(),
    )
    .map_err(StorageTransitionError::Locator)?;
    if !target_committed {
        return Err(StorageTransitionError::CleanupRefused(
            "fixed locator does not select the receipt target".into(),
        ));
    }

    let targets = resolve_targets(request, plan)?;
    for ((source, evidence), target) in plan.paths.iter().zip(&plan.evidence).zip(&targets) {
        verify_evidence(target, evidence, "target")?;
        if source.exists() {
            verify_evidence(source, evidence, "source")?;
        }
    }

    let mut deleted_paths = Vec::new();
    let mut already_absent_paths = Vec::new();
    for source in &plan.paths {
        if source.exists() {
            io::remove_file(source)?;
            deleted_paths.push(source.clone());
        } else {
            already_absent_paths.push(source.clone());
        }
    }
    Ok(StorageTransitionCleanupReceipt {
        transition_id: receipt.transition_id.clone(),
        deleted_paths,
        already_absent_paths,
    })
}

fn validate_binding(
    receipt: &StorageTransitionReceipt,
    plan: &StorageTransitionCleanupPlan,
) -> Result<(), StorageTransitionError> {
    if receipt.outcome != StorageTransitionOutcome::TargetCommitted
        || plan.transition_id != receipt.transition_id
        || plan.receipt_digest != receipt.receipt_digest
        || plan.paths != receipt.retained_source_paths
        || plan.evidence != receipt.retained_source_evidence
        || plan.paths.len() != plan.evidence.len()
    {
        return Err(StorageTransitionError::CleanupRefused(
            "cleanup plan does not match its committed receipt".into(),
        ));
    }
    Ok(())
}

fn resolve_targets(
    request: &StorageTransitionRequest<'_>,
    plan: &StorageTransitionCleanupPlan,
) -> Result<Vec<std::path::PathBuf>, StorageTransitionError> {
    plan.paths
        .iter()
        .map(|source_path| {
            request
                .source_store
                .registered_descriptors()
                .find_map(|descriptor| {
                    let DomainLocation::File(source) =
                        request.source_store.roots.resolve(descriptor)
                    else {
                        return None;
                    };
                    if source.full_path() != source_path {
                        return None;
                    }
                    let target_descriptor = request
                        .target_store
                        .registered_descriptor(descriptor.id())?;
                    if target_descriptor != descriptor {
                        return None;
                    }
                    let DomainLocation::File(target) =
                        request.target_store.roots.resolve(target_descriptor)
                    else {
                        return None;
                    };
                    (target.full_path() != source.full_path())
                        .then(|| target.full_path().to_path_buf())
                })
                .ok_or_else(|| {
                    StorageTransitionError::CleanupRefused(format!(
                        "{} is not an exact retained registered source",
                        source_path.display()
                    ))
                })
        })
        .collect()
}

fn verify_evidence(
    path: &Path,
    expected: &StorageFileEvidence,
    authority: &str,
) -> Result<(), StorageTransitionError> {
    let observed = match fs::read(path) {
        Ok(bytes) => StorageFileEvidence::Present {
            byte_length: bytes.len(),
            sha256: crate::Sha256Digest::from_bytes(&bytes),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => StorageFileEvidence::Absent,
        Err(error) => {
            return Err(StorageTransitionError::Filesystem {
                path: path.to_path_buf(),
                detail: error.to_string(),
            });
        }
    };
    if &observed == expected {
        Ok(())
    } else {
        Err(StorageTransitionError::CleanupRefused(format!(
            "{authority} evidence changed at {}",
            path.display()
        )))
    }
}
